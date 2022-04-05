use crate::{
    fee::LinearFee,
    testing::{
        ledger::ConfigBuilder,
        scenario::{prepare_scenario, wallet},
        verifiers::LedgerStateVerifier,
    },
    value::Value,
};
use chain_addr::Discrimination;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;
use rand::Rng;

#[quickcheck]
pub fn validate_ledger_state_after_transaction(amount: u64) -> TestResult {
    if amount == 0 {
        return TestResult::discard();
    };

    let mut rng = rand::thread_rng();

    let (fee, total_fees) = (LinearFee::new(1, 1, 1), 3);
    let (alice_initial_balance, bob_initial_balance) =
        (amount + total_fees + 1, rng.gen_range(1..10));

    let (mut ledger, controller) = prepare_scenario()
        .with_config(
            ConfigBuilder::new()
                .with_discrimination(Discrimination::Test)
                .with_fee(fee),
        )
        .with_initials(vec![
            wallet("Alice").with(alice_initial_balance),
            wallet("Bob").with(bob_initial_balance),
        ])
        .build()
        .unwrap();

    let mut alice = controller.wallet("Alice").unwrap();
    let bob = controller.wallet("Bob").unwrap();

    controller
        .transfer_funds(&alice, &bob, &mut ledger, amount + total_fees)
        .unwrap();
    alice.confirm_transaction();

    LedgerStateVerifier::new(ledger.into())
        .address_has_expected_balance(bob.as_account_data(), Value(bob_initial_balance + amount));

    TestResult::passed()
}

#[quickcheck]
pub fn validate_ledger_state_after_invalid_transaction(amount: u64) -> TestResult {
    if amount == 0 {
        return TestResult::discard();
    };

    let mut rng = rand::thread_rng();

    let (fee, total_fees) = (LinearFee::new(1, 1, 1), 3);
    let (alice_initial_balance, bob_initial_balance) =
        (amount + total_fees + 1, rng.gen_range(1..10));

    let (mut ledger, controller) = prepare_scenario()
        .with_config(
            ConfigBuilder::new()
                .with_discrimination(Discrimination::Test)
                .with_fee(fee),
        )
        .with_initials(vec![
            wallet("Alice").with(alice_initial_balance),
            wallet("Bob").with(bob_initial_balance),
        ])
        .build()
        .unwrap();

    let alice = controller.wallet("Alice").unwrap();
    let bob = controller.wallet("Bob").unwrap();

    controller
        .transfer_funds(&alice, &bob, &mut ledger, amount + total_fees)
        .unwrap();

    TestResult::from_bool(
        controller
            .transfer_funds(&alice, &bob, &mut ledger, amount + total_fees)
            .is_err(),
    )
}
