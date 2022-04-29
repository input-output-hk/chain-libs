use crate::{
    fee::LinearFee,
    testing::{
        ledger::ConfigBuilder,
        scenario::{prepare_scenario, wallet},
        verifiers::LedgerStateVerifier,
        arbitrary::Random1to10,
    },
    value::Value,
};
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

const BASIC_BALANCE: u64 = 1000;

#[quickcheck]
pub fn validate_ledger_state_after_transaction(amount: Random1to10, linear_fee: LinearFee) {
    println!("amount: {:?}", amount.0);

    let total_fees = linear_fee.constant + linear_fee.coefficient + linear_fee.certificate - (linear_fee.certificate - linear_fee.coefficient);
    let valid_transaction_amount = total_fees + amount.0;
    let alice_initial_balance = BASIC_BALANCE + valid_transaction_amount + total_fees;
    let bob_initial_balance = BASIC_BALANCE;

    let (mut ledger, controller) = prepare_scenario()
        .with_config(
            ConfigBuilder::new()
                .with_fee(linear_fee),
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
        .transfer_funds(&alice, &bob, &mut ledger, valid_transaction_amount + total_fees)
        .unwrap();
    alice.confirm_transaction();

    LedgerStateVerifier::new(ledger.into())
        .address_has_expected_balance(bob.as_account_data(), Value(bob_initial_balance + valid_transaction_amount));
}

#[quickcheck]
pub fn validate_ledger_state_after_invalid_transaction(amount: Random1to10, linear_fee: LinearFee) {
    let total_fees = linear_fee.constant + linear_fee.coefficient + linear_fee.certificate;
    let valid_transaction_amount = total_fees + amount.0;
    let alice_initial_balance = amount.0 + valid_transaction_amount + total_fees;
    let bob_initial_balance = BASIC_BALANCE;

    let (mut ledger, controller) = prepare_scenario()
        .with_config(
            ConfigBuilder::new()
                .with_fee(linear_fee),
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
        .transfer_funds(&alice, &bob, &mut ledger, valid_transaction_amount + total_fees)
        .unwrap();

    alice.confirm_transaction();

    // this second transaction should fail as alice does not have the balance to cover for it
    let _ = controller.transfer_funds(&alice, &bob, &mut ledger, valid_transaction_amount + total_fees);

    alice.confirm_transaction();

    LedgerStateVerifier::new(ledger.into())
        .address_has_expected_balance(alice.as_account_data(), Value(alice_initial_balance - (valid_transaction_amount + total_fees)));
}
