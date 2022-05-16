use crate::testing::scenario::wallet;
use crate::testing::TestGen;
use crate::testing::{scenario::prepare_scenario, verifiers::LedgerStateVerifier, ConfigBuilder};
use chain_evm::Config;

const ALICE: &str = "Alice";
const BOB: &str = "Bob";

#[test]
pub fn evm_mapping() {
    let (mut ledger, controller) = prepare_scenario()
        .with_initials(vec![
            wallet(ALICE).with(1_000).owns("alice_stake_pool"),
            wallet(BOB).with(1_000).owns("bob_stake_pool"),
        ])
        .with_config(ConfigBuilder::new().with_evm_params(Config::default()))
        .build()
        .unwrap();

    let alice = controller.wallet(ALICE).unwrap();
    let mut bob = controller.wallet(BOB).unwrap();

    let alice_evm_mapping = TestGen::evm_mapping_for_wallet(&alice);
    let bob_evm_mapping = TestGen::evm_mapping_for_wallet(&bob);

    LedgerStateVerifier::new(ledger.clone().into())
        .is_not_mapped_to_evm(&alice)
        .is_not_mapped_to_evm(&bob);

    controller
        .evm_mapping(&alice, alice_evm_mapping.clone(), &mut ledger)
        .unwrap();

    LedgerStateVerifier::new(ledger.clone().into())
        .is_mapped_to_evm(&alice_evm_mapping)
        .is_not_mapped_to_evm(&bob);

    controller
        .evm_mapping(&bob, bob_evm_mapping.clone(), &mut ledger)
        .unwrap();

    bob.confirm_transaction();

    LedgerStateVerifier::new(ledger.clone().into())
        .is_mapped_to_evm(&bob_evm_mapping)
        .is_mapped_to_evm(&alice_evm_mapping);

    assert!(controller
        .evm_mapping(&bob, bob_evm_mapping.clone(), &mut ledger)
        .is_err());
}
