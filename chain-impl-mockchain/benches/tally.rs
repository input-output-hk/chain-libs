use chain_impl_mockchain::testing::scenario::template::WalletTemplateBuilder;
use chain_impl_mockchain::{
    certificate::{
        DecryptedPrivateTally, DecryptedPrivateTallyProposal, EncryptedVoteTally, VotePlan,
        VoteTally,
    },
    fee::LinearFee,
    header::BlockDate,
    testing::{
        data::CommitteeMembersManager,
        ledger::ConfigBuilder,
        scenario::{prepare_scenario, proposal, vote_plan, wallet},
        VoteTestGen,
    },
    value::Value,
    vote::{Choice, PayloadType},
};
use criterion::{criterion_group, criterion_main, Criterion};

use rand_chacha::ChaCha20Rng;
use rand_core::SeedableRng;

const ALICE: &str = "Alice";
const STAKE_POOL: &str = "stake_pool";
const VOTE_PLAN: &str = "fund1";

fn tally_benchmark(
    bench_name_suffix: &str,
    voting_powers: impl Iterator<Item = u64>,
    c: &mut Criterion,
) {
    const MEMBERS_NO: usize = 3;
    const THRESHOLD: usize = 2;
    let favorable = Choice::new(1);

    let mut wallets: Vec<&mut WalletTemplateBuilder> = Vec::new();

    let mut alice_wallet_builder = wallet(ALICE);
    alice_wallet_builder
        .with(1_000)
        .owns(STAKE_POOL)
        .committee_member();
    wallets.push(&mut alice_wallet_builder);

    let mut voters_aliases = Vec::new();
    let mut voters_wallets = Vec::new();
    let mut total_votes = 0u64;

    for (i, voting_power) in voting_powers.enumerate() {
        let alias = format!("voter_{}", i);
        let mut wallet_builder = wallet(&alias);
        wallet_builder.with(voting_power);
        voters_wallets.push(wallet_builder);
        voters_aliases.push(alias);
        total_votes += voting_power;
    }

    voters_wallets
        .iter_mut()
        .for_each(|wallet| wallets.push(wallet));

    let mut rng = TestCryptoRng::from_seed([0u8; 16]);
    let members = CommitteeMembersManager::new(&mut rng, THRESHOLD, MEMBERS_NO);

    let committee_keys = members
        .members()
        .iter()
        .map(|committee_member| committee_member.public_key())
        .collect::<Vec<_>>();

    let (mut ledger, controller) = prepare_scenario()
        .with_config(
            ConfigBuilder::new(0)
                .with_fee(LinearFee::new(0, 0, 0))
                .with_rewards(Value(1000)),
        )
        .with_initials(wallets)
        .with_vote_plans(vec![vote_plan(VOTE_PLAN)
            .owner(ALICE)
            .consecutive_epoch_dates()
            .payload_type(PayloadType::Private)
            .committee_keys(committee_keys)
            .with_proposal(
                proposal(VoteTestGen::external_proposal_id())
                    .options(3)
                    .action_transfer_to_rewards(100),
            )])
        .build()
        .unwrap();

    let mut alice = controller.wallet(ALICE).unwrap();

    let vote_plan_def = controller.vote_plan(VOTE_PLAN).unwrap();
    let vote_plan: VotePlan = vote_plan_def.clone().into();
    let proposal = vote_plan_def.proposal(0);

    for alias in voters_aliases {
        let mut private_voter = controller.wallet(&alias).unwrap();

        controller
            .cast_vote_private(
                &private_voter,
                &vote_plan_def,
                &proposal.id(),
                favorable,
                &mut ledger,
                &mut rng,
            )
            .unwrap();
        private_voter.confirm_transaction();
    }

    ledger.fast_forward_to(BlockDate {
        epoch: 1,
        slot_id: 1,
    });

    let encrypted_tally = EncryptedVoteTally::new(vote_plan.to_id());
    let fragment = controller
        .fragment_factory()
        .vote_encrypted_tally(&alice, encrypted_tally);

    let parameters = ledger.parameters.clone();
    let date = ledger.date();

    c.bench_function(
        &format!("vote_encrypted_tally_{}", bench_name_suffix),
        |b| {
            b.iter(|| {
                ledger
                    .ledger
                    .apply_fragment(&parameters, &fragment, date)
                    .unwrap();
            })
        },
    );

    ledger.apply_fragment(&fragment, ledger.date()).unwrap();
    alice.confirm_transaction();

    let vote_plans = ledger.ledger.active_vote_plans();
    let vote_plan_status = vote_plans
        .iter()
        .find(|c_vote_plan| {
            let vote_plan: VotePlan = vote_plan.clone().into();
            c_vote_plan.id == vote_plan.to_id()
        })
        .unwrap();

    c.bench_function(&format!("tally_decrypt_share_{}", bench_name_suffix), |b| {
        b.iter(|| {
            members.members()[0].produce_decrypt_shares(&vote_plan_status);
        })
    });

    let decrypt_shares: Vec<_> = members
        .members()
        .iter()
        // We use only one proposal in this benchmark so here's a bit of a dirty hack.
        .map(|member| member.produce_decrypt_shares(&vote_plan_status).into_iter())
        .flatten()
        .collect();

    let decrypt_tally = || {
        let tally_state = vote_plan_status.proposals[0]
            .tally
            .clone()
            .unwrap()
            .private_encrypted()
            .unwrap()
            .0
            .state();
        let table = chain_vote::TallyOptimizationTable::generate_with_balance(total_votes, 1);
        chain_vote::tally(total_votes, &tally_state, &decrypt_shares, &table).unwrap()
    };

    c.bench_function(
        &format!("decrypt_private_tally_{}", bench_name_suffix),
        |b| b.iter(decrypt_tally),
    );

    let tally = decrypt_tally();
    let shares = DecryptedPrivateTallyProposal {
        decrypt_shares: decrypt_shares.into_boxed_slice(),
        tally_result: tally.votes.into_boxed_slice(),
    };

    let decrypted_tally =
        VoteTally::new_private(vote_plan.to_id(), DecryptedPrivateTally::new(vec![shares]));
    let fragment = controller
        .fragment_factory()
        .vote_tally(&alice, decrypted_tally);

    c.bench_function(&format!("vote_tally_{}", bench_name_suffix), |b| {
        b.iter(|| {
            ledger
                .ledger
                .apply_fragment(&parameters, &fragment, ledger.date())
                .unwrap();
        })
    });

    ledger.apply_fragment(&fragment, ledger.date()).unwrap();
}

fn tally_benchmark_128_voters_1000_ada(c: &mut Criterion) {
    tally_benchmark("128_voters_1000_ada", std::iter::repeat(1000).take(128), c);
}

fn tally_benchmark_200_voters_1000_ada(c: &mut Criterion) {
    tally_benchmark("200_voters_1000_ada", std::iter::repeat(1000).take(200), c);
}

fn tally_benchmark_200_voters_1_000_000_ada(c: &mut Criterion) {
    tally_benchmark(
        "200_voters_1_000_000_ada",
        std::iter::repeat(1_000_000).take(200),
        c,
    );
}

fn tally_benchmark_1000_voters_1000_ada(c: &mut Criterion) {
    tally_benchmark(
        "1000_voters_1000_ada",
        std::iter::repeat(1000).take(1000),
        c,
    );
}

fn tally_benchmark_fund2_stake(c: &mut Criterion) {
    let mut csv_reader = csv::Reader::from_path("./benches/fund2_sim.csv").unwrap();
    let csv_iter = csv_reader
        .records()
        .map(|record| record.unwrap()[0].parse().unwrap());
    tally_benchmark("fund2_stake_data_based", csv_iter, c);
}

criterion_group!(
    benches,
    tally_benchmark_128_voters_1000_ada,
    tally_benchmark_200_voters_1000_ada,
    tally_benchmark_200_voters_1_000_000_ada,
    tally_benchmark_1000_voters_1000_ada,
    tally_benchmark_fund2_stake,
);
criterion_main!(benches);
