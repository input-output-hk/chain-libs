use crate::{
    fee::LinearFee,
    testing::{
        ledger::{ConfigBuilder},
        scenario::{prepare_scenario, wallet},
        verifiers::LedgerStateVerifier,
    },
    value::Value,
};
use chain_addr::Discrimination;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn validate_ledger_state_after_transaction_quickcheck(amount: u64){
    let validated_amount = if amount == 0 { 42 } else { amount };

    let (alice_initial_balance, bob_initial_balance) = (validated_amount + 13, 42);

    let (mut ledger, controller) = prepare_scenario()
        .with_config(
            ConfigBuilder::new()
                .with_discrimination(Discrimination::Test)
                .with_fee(LinearFee::new(1, 1, 1)),
        )
        .with_initials(vec![
            wallet("Alice").with(alice_initial_balance),
            wallet("Bob").with(bob_initial_balance),
        ])
        .build()
        .unwrap();
    let mut alice = controller.wallet("Alice").unwrap();
    let bob = controller.wallet("Bob").unwrap();

    controller.transfer_funds(&alice, &bob, &mut ledger, validated_amount + 3).unwrap(); 
    alice.confirm_transaction();

    LedgerStateVerifier::new(ledger.into())
    .address_has_expected_balance(bob.as_account_data(), Value(bob_initial_balance + validated_amount));
}