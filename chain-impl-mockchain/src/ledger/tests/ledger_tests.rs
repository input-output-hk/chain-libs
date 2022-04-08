#![cfg(test)]

use crate::{
    chaintypes::ConsensusType,
    config::ConfigParam,
    date::BlockDate,
    fragment::{config::ConfigParams, Fragment},
    ledger::{
        ledger::{
            Block0Error,
            Error::{Block0, ExpectingInitialMessage},
        },
        Ledger,
    },
    milli::Milli,
    testing::{
        arbitrary::{AccountStatesVerifier, ArbitraryValidTransactionData, UtxoVerifier},
        builders::{OldAddressBuilder, TestTxBuilder},
        data::AddressDataValue,
        ledger::{ConfigBuilder, LedgerBuilder},
        TestGen,
    },
};

use chain_addr::Discrimination;
use proptest::{prop_assert, prop_assert_eq, prop_oneof, strategy::Strategy};
use test_strategy::proptest;

#[proptest]
fn ledger_accepts_correct_transaction(faucet: AddressDataValue, receiver: AddressDataValue) {
    let mut ledger = LedgerBuilder::from_config(ConfigBuilder::new())
        .initial_fund(&faucet)
        .build()
        .unwrap();
    let fragment = TestTxBuilder::new(ledger.block0_hash)
        .move_funds(&mut ledger, &faucet, &receiver, faucet.value)
        .get_fragment();
    let total_funds_before = ledger.total_funds();
    ledger
        .apply_transaction(fragment, BlockDate::first())
        .unwrap();

    let total_funds_after = ledger.total_funds();
    prop_assert_eq!(
        total_funds_before,
        total_funds_after,
        "Total funds in ledger before and after transaction is not equal {} <> {} ",
        total_funds_before,
        total_funds_after
    )
}

#[proptest]
fn total_funds_are_const_in_ledger(transaction_data: ArbitraryValidTransactionData) {
    let config = ConfigBuilder::new()
        .with_discrimination(Discrimination::Test)
        .with_fee(transaction_data.fee);

    let mut ledger = LedgerBuilder::from_config(config)
        .initial_funds(&transaction_data.addresses)
        .build()
        .unwrap();
    let signed_tx = TestTxBuilder::new(ledger.block0_hash).move_funds_multiple(
        &mut ledger,
        &transaction_data.input_addresses,
        &transaction_data.output_addresses,
    );
    let total_funds_before = ledger.total_funds();
    ledger
        .apply_transaction(signed_tx.get_fragment(), BlockDate::first())
        .unwrap();

    let total_funds_after = ledger.total_funds();

    prop_assert_eq!(
        total_funds_before,
        total_funds_after,
        "Total funds in ledger before and after transaction is not equal {} <> {}",
        total_funds_before,
        total_funds_after
    );

    let utxo_verifier = UtxoVerifier::new(transaction_data.clone());
    utxo_verifier.verify(&ledger).unwrap();

    let account_state_verifier = AccountStatesVerifier::new(transaction_data);
    account_state_verifier.verify(ledger.accounts()).unwrap();
}

#[test]
pub fn test_first_initial_fragment_empty() {
    let header_id = TestGen::hash();
    let content = Vec::new();
    assert_eq!(
        Ledger::new(header_id, content).err().unwrap(),
        Block0(Block0Error::InitialMessageMissing)
    );
}

#[test]
pub fn test_first_initial_fragment_wrong_type() {
    let header_id = TestGen::hash();
    let fragment = Fragment::OldUtxoDeclaration(OldAddressBuilder::build_utxo_declaration(Some(1)));
    assert_eq!(
        Ledger::new(header_id, &vec![fragment]).err().unwrap(),
        ExpectingInitialMessage
    );
}

#[test]
pub fn ledger_new_no_block_start_time() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));

    assert_eq!(
        Ledger::new(header_id, vec![&Fragment::Initial(ie)])
            .err()
            .unwrap(),
        Block0(Block0Error::InitialMessageNoDate)
    );
}

#[test]
pub fn ledger_new_dupicated_initial_fragments() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));

    assert_eq!(
        Ledger::new(
            header_id,
            vec![&Fragment::Initial(ie.clone()), &Fragment::Initial(ie)]
        )
        .err()
        .unwrap(),
        Block0(Block0Error::InitialMessageMany)
    );
}

#[test]
pub fn ledger_new_duplicated_block0() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));

    Ledger::new(header_id, vec![&Fragment::Initial(ie)]).unwrap();
}

#[test]
pub fn ledger_new_duplicated_discrimination() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));

    Ledger::new(header_id, vec![&Fragment::Initial(ie)]).unwrap();
}

#[test]
pub fn ledger_new_duplicated_consensus_version() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::ConsensusVersion(ConsensusType::Bft));
    ie.push(ConfigParam::ConsensusVersion(ConsensusType::Bft));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));

    Ledger::new(header_id, vec![&Fragment::Initial(ie)]).unwrap();
}

#[test]
pub fn ledger_new_duplicated_slot_duration() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotDuration(11u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));

    Ledger::new(header_id, vec![&Fragment::Initial(ie)]).unwrap();
}

#[test]
pub fn ledger_new_duplicated_epoch_stability_depth() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::ConsensusVersion(ConsensusType::Bft));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::EpochStabilityDepth(10u32));
    ie.push(ConfigParam::EpochStabilityDepth(11u32));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));

    Ledger::new(header_id, vec![&Fragment::Initial(ie)]).unwrap();
}

#[test]
pub fn ledger_new_duplicated_active_slots_coeff() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::ConsensusVersion(ConsensusType::Bft));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
        Milli::from_millis(500),
    ));
    ie.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
        Milli::from_millis(600),
    ));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));

    Ledger::new(header_id, vec![&Fragment::Initial(ie)]).unwrap();
}

#[test]
pub fn ledger_new_no_discrimination() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));

    assert_eq!(
        Ledger::new(header_id, vec![&Fragment::Initial(ie)])
            .err()
            .unwrap(),
        Block0(Block0Error::InitialMessageNoDiscrimination)
    );
}

#[test]
pub fn ledger_new_no_slot_duration() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));

    assert_eq!(
        Ledger::new(header_id, vec![&Fragment::Initial(ie)])
            .err()
            .unwrap(),
        Block0(Block0Error::InitialMessageNoSlotDuration)
    );
}

#[test]
pub fn ledger_new_no_slots_per_epoch() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::KesUpdateSpeed(3600));

    assert_eq!(
        Ledger::new(header_id, vec![&Fragment::Initial(ie)])
            .err()
            .unwrap(),
        Block0(Block0Error::InitialMessageNoSlotsPerEpoch)
    );
}

#[test]
pub fn ledger_new_no_kes_update_speed() {
    let leader_pair = TestGen::leader_pair();
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));

    assert_eq!(
        Ledger::new(header_id, vec![&Fragment::Initial(ie)])
            .err()
            .unwrap(),
        Block0(Block0Error::InitialMessageNoKesUpdateSpeed)
    );
}

#[test]
pub fn ledger_new_no_bft_leader() {
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));

    assert_eq!(
        Ledger::new(header_id, vec![&Fragment::Initial(ie)])
            .err()
            .unwrap(),
        Block0(Block0Error::InitialMessageNoConsensusLeaderId)
    );
}

fn fragment_strategy() -> impl Strategy<Value = Fragment> {
    use crate::{
        certificate::{
            MintToken, OwnerStakeDelegation, PoolRetirement, PoolUpdate, UpdateProposal,
            UpdateVote, VoteCast, VoteTally,
        },
        transaction::Transaction,
    };
    use proptest::prelude::*;

    prop_oneof![
        any::<ConfigParams>().prop_map(Fragment::Initial),
        any::<Transaction<OwnerStakeDelegation>>().prop_map(Fragment::OwnerStakeDelegation),
        any::<Transaction<PoolRetirement>>().prop_map(Fragment::PoolRetirement),
        any::<Transaction<PoolUpdate>>().prop_map(Fragment::PoolUpdate),
        any::<Transaction<UpdateProposal>>().prop_map(Fragment::UpdateProposal),
        any::<Transaction<UpdateVote>>().prop_map(Fragment::UpdateVote),
        any::<Transaction<VoteCast>>().prop_map(Fragment::VoteCast),
        any::<Transaction<VoteTally>>().prop_map(Fragment::VoteTally),
        any::<Transaction<MintToken>>().prop_map(Fragment::MintToken),
    ]
}

#[proptest]
fn wrong_fragment_at_block0(#[strategy(fragment_strategy())] fragment: Fragment) {
    let header_id = TestGen::hash();
    let mut ie = ConfigParams::new();
    let leader_pair = TestGen::leader_pair();
    ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
    ie.push(ConfigParam::Discrimination(Discrimination::Test));
    ie.push(ConfigParam::AddBftLeader(leader_pair.id()));
    ie.push(ConfigParam::SlotDuration(10u8));
    ie.push(ConfigParam::SlotsPerEpoch(10u32));
    ie.push(ConfigParam::KesUpdateSpeed(3600));

    prop_assert!(Ledger::new(header_id, vec![&Fragment::Initial(ie), &fragment]).is_err());
}
