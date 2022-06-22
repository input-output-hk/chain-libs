use crate::testing::scenario::wallet;
use crate::testing::TestGen;
use crate::testing::{scenario::prepare_scenario, verifiers::LedgerStateVerifier, ConfigBuilder};
use crate::value::Value;
use chain_evm::Config;

const ALICE: &str = "Alice";
const BOB: &str = "Bob";
const INITIAL_FUNDS: u64 = 1000;
const TRANSACTION_AMOUNT: u64 = 100;
const MAX_GAS_FEE: u64 = 0;
const FIRST_NONCE: u64 = 0;
//const SECOND_NONCE: u64 = 1;
//const INVALID_NONCE: u64 = 9845;

#[test] // Simple transaction scenario
pub fn evm_transaction() {
    let (mut ledger, controller) = prepare_scenario()
        .with_initials(vec![
            wallet(ALICE).with(INITIAL_FUNDS).owns("alice_stake_pool"),
            wallet(BOB).with(INITIAL_FUNDS).owns("bob_stake_pool"),
        ])
        .with_config(ConfigBuilder::new().with_evm_params(Config::default()))
        .build()
        .unwrap();

    println!("****************************************************************** START");
    println!("Ledger Settings: {:?}", ledger.settings());
    println!("****************************************************************** FLAG 1");

    let mut alice = controller.wallet(ALICE).unwrap();
    let mut bob = controller.wallet(BOB).unwrap();

    let alice_evm_mapping = TestGen::evm_mapping_for_wallet(&alice);
    let bob_evm_mapping = TestGen::evm_mapping_for_wallet(&bob);


    LedgerStateVerifier::new(ledger.clone().into())
        .info("Alice initial balance is incorrect")
        .account_has_expected_balance(alice.as_account_data(), Value(INITIAL_FUNDS));

    controller
    .evm_mapping(&alice, alice_evm_mapping.clone(), &mut ledger)
    .unwrap();
    
    controller
    .evm_mapping(&bob, bob_evm_mapping.clone(), &mut ledger)
    .unwrap();

    alice.confirm_transaction();
    bob.confirm_transaction();

    let alice_evm_address = ledger.get_evm_mapped_address(&alice.as_account().to_id()).unwrap();
    let bob_evm_address = ledger.get_evm_mapped_address(&bob.as_account().to_id()).unwrap();

    println!("Alice's address:{:?}", alice_evm_address);
    println!("Bob's address:{:?}", bob_evm_address);
    
    let evm_transaction = TestGen::evm_transaction(alice_evm_address, bob_evm_address, TRANSACTION_AMOUNT, MAX_GAS_FEE, FIRST_NONCE);

    println!("Transaction: {:?}", evm_transaction);

    controller.evm_transaction(evm_transaction.clone(), &mut ledger).unwrap();

    alice.confirm_transaction();
    
    LedgerStateVerifier::new(ledger.clone().into())
        .info("Bob final balance is incorrect.")
        .account_has_expected_balance(bob.as_account_data(), Value(INITIAL_FUNDS + TRANSACTION_AMOUNT));
}