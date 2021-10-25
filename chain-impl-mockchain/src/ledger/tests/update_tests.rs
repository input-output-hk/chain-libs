use crate::{
    block::{self, Block},
    chaintypes::HeaderId,
    date::BlockDate,
    fragment::Contents,
    header::BlockVersion,
    ledger::ledger::Ledger,
    testing::arbitrary::update_proposal::UpdateProposalData,
    testing::{ConfigBuilder, LedgerBuilder},
};
use chain_crypto::{Ed25519, SecretKey};
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_adopt_settings_from_update_proposal(
    update_proposal_data: UpdateProposalData,
) -> TestResult {
    let cb = ConfigBuilder::new().with_leaders(&update_proposal_data.leaders_ids());

    let testledger = LedgerBuilder::from_config(cb)
        .build()
        .expect("cannot build test ledger");
    let mut ledger = testledger.ledger;

    // apply proposal
    let date = ledger.date();
    ledger = ledger
        .apply_update_proposal(&update_proposal_data.proposal, date)
        .unwrap();

    // apply votes
    for vote in update_proposal_data.votes.iter() {
        ledger = ledger.apply_update_vote(&vote).unwrap();
    }

    // trigger proposal process (build block)
    let block = build_block(
        &ledger,
        testledger.block0_hash,
        date.next_epoch(),
        &update_proposal_data.block_signing_key,
    );
    let header_meta = block.header().get_content_eval_context();
    ledger = ledger
        .apply_block(
            ledger.get_ledger_parameters(),
            block.contents(),
            &header_meta,
        )
        .unwrap();

    // assert
    let actual_params = ledger.settings.to_config_params();
    let expected_params = update_proposal_data.proposal_settings();

    let mut all_settings_equal = true;
    for expected_param in expected_params.iter() {
        if !actual_params.iter().any(|x| x == expected_param) {
            all_settings_equal = false;
            break;
        }
    }

    if !ledger.updates.proposals.is_empty() {
        return TestResult::error(format!(
            "Error: proposal collection should be empty but contains:{:?}",
            ledger.updates.proposals
        ));
    }

    if all_settings_equal {
        TestResult::passed()
    } else {
        TestResult::error(format!("Error: proposed update reached required votes, but proposal was NOT updated, Expected: {:?} vs Actual: {:?}",
                                expected_params,actual_params))
    }
}

fn build_block(
    ledger: &Ledger,
    block0_hash: HeaderId,
    date: BlockDate,
    block_signing_key: &SecretKey<Ed25519>,
) -> Block {
    let contents = Contents::empty();
    block::builder(BlockVersion::Ed25519Signed, contents, |header_builder| {
        Ok::<_, ()>(
            header_builder
                .set_parent(&block0_hash, ledger.chain_length.increase())
                .set_date(date.next_epoch())
                .into_bft_builder()
                .unwrap()
                .sign_using(block_signing_key)
                .generalize(),
        )
    })
    .unwrap()
}
