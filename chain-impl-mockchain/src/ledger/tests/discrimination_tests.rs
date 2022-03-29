#![cfg(test)]

use crate::{
    date::BlockDate,
    testing::{
        builders::TestTxBuilder,
        data::AddressDataValue,
        kind_type::pt::kind_type_without_multisig,
        ledger::{ConfigBuilder, LedgerBuilder},
    },
    value::Value,
};
use chain_addr::Discrimination;
use proptest::arbitrary::any;

proptest::proptest! {
    fn ledger_verifies_faucet_discrimination(
        arbitrary_faucet_disc in any::<Discrimination>(),
        arbitrary_faucet_address_kind in kind_type_without_multisig(),
        arbitrary_ledger_disc in any::<Discrimination>(),
    ) {
        let config = ConfigBuilder::new().with_discrimination(arbitrary_ledger_disc);

        let faucet = AddressDataValue::from_discrimination_and_kind_type(
            arbitrary_faucet_disc,
            arbitrary_faucet_address_kind,
            Value(1000),
        );

        let are_discriminations_unified = arbitrary_faucet_disc == arbitrary_ledger_disc;

        match (
            are_discriminations_unified,
            LedgerBuilder::from_config(config).faucet(&faucet).build(),
        ) {
            (false, Ok(_)) => {
                panic!("Ledger should reject transaction with mixed discriminations")
            }
            (true, Err(_)) => {
                panic!("Ledger should accept transaction with unified discriminations")
            }
            _ => {},
        };
    }
}

proptest::proptest! {
    fn ledger_verifies_transaction_discrimination(
        arbitrary_input_disc in any::<Discrimination>(),
        arbitrary_output_disc in any::<Discrimination>(),
        arbitrary_input_address_kind in kind_type_without_multisig(),
        arbitrary_output_address_kind in kind_type_without_multisig(),
    ) {
        let faucet = AddressDataValue::from_discrimination_and_kind_type(
            arbitrary_input_disc,
            arbitrary_input_address_kind,
            Value(100),
        );
        let receiver = AddressDataValue::from_discrimination_and_kind_type(
            arbitrary_output_disc,
            arbitrary_output_address_kind,
            Value(100),
        );

        let config = ConfigBuilder::new().with_discrimination(arbitrary_input_disc);

        let mut ledger = LedgerBuilder::from_config(config)
            .initial_fund(&faucet)
            .build()
            .unwrap();
        let fragment = TestTxBuilder::new(ledger.block0_hash)
            .move_all_funds(&mut ledger, &faucet, &receiver)
            .get_fragment();

        let are_discriminations_unified = arbitrary_input_disc == arbitrary_output_disc;
        let actual_result = ledger.apply_transaction(fragment, BlockDate::first());

        match (are_discriminations_unified, actual_result) {
            (false, Ok(_)) => {
                panic!("Ledger should reject transaction with mixed discriminations")
            }
            (true, Err(err)) => panic!(
                "Ledger should accept transaction with unified discriminations. Err: {}",
                err
            ),
            _ => {}
        }
    }

}
