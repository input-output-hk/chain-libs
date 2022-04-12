#![cfg(test)]

use crate::fragment::Fragment;
use crate::ledger::Error::FeeCalculationError;
use crate::testing::OldAddressBuilder;
use crate::value::ValueError;
use crate::{
    accounting::account::DelegationType,
    ledger::{Block0Error, Error::Block0},
    testing::{
        arbitrary::address::ArbitraryAddressDataValueVec,
        create_initial_stake_pool_owner_delegation, create_initial_transaction,
        data::AddressDataValue,
        data::Wallet,
        ledger::{ConfigBuilder, LedgerBuilder},
        InitialFaultTolerantTxBuilder, InitialFaultTolerantTxCertBuilder, VoteTestGen,
    },
    value::Value,
};
use chain_addr::Discrimination;
use chain_core::property::Fragment as _;
#[cfg(test)]
use proptest::prop_assume;
use test_strategy::proptest;

#[proptest]
fn ledger_verifies_value_of_initial_funds(arbitrary_faucets: ArbitraryAddressDataValueVec) {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);
    let result = LedgerBuilder::from_config(config)
        .initial_funds(&arbitrary_faucets.values())
        .build();
    prop_assume!(!matches!(
        result,
        Err(FeeCalculationError(ValueError::Overflow))
    ));
    result.unwrap();
}

#[test]
pub fn ledger_fails_to_start_when_there_is_zero_output() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let address = AddressDataValue::account(Discrimination::Test, Value::zero());

    assert!(
        LedgerBuilder::from_config(config)
            .faucet(&address)
            .build()
            .is_err(),
        "Ledger should fail to start with zero value output"
    );
}

#[test]
pub fn ledger_fails_to_start_when_utxo_ammount_is_too_big() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let address_1 = AddressDataValue::account(Discrimination::Test, Value(u64::MAX));
    let address_2 = AddressDataValue::account(Discrimination::Test, Value(u64::MAX));

    assert_eq!(
        LedgerBuilder::from_config(config)
            .faucets(&[address_1, address_2])
            .build()
            .err()
            .unwrap(),
        Block0(Block0Error::UtxoTotalValueTooBig)
    );
}

#[test]
#[should_panic]
pub fn ledger_fails_to_start_on_wrong_old_utxo_declaration_length() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);
    LedgerBuilder::from_config(config)
        .fragments(&[Fragment::OldUtxoDeclaration(
            OldAddressBuilder::build_utxo_declaration(Some(256)),
        )])
        .build()
        .unwrap();
}

#[test]
pub fn ledger_starts_with_old_utxo_declaration() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let old_address = OldAddressBuilder::build_utxo_declaration(Some(254));
    let fragment = Fragment::OldUtxoDeclaration(old_address.clone());
    let ledger = LedgerBuilder::from_config(config)
        .fragments(&[fragment.clone()])
        .build()
        .unwrap();

    let output = ledger
        .ledger
        .oldutxos
        .get(&fragment.id(), 0)
        .unwrap()
        .output
        .clone();
    let (address, value) = (output.address, output.value);

    assert_eq!((address, value), old_address.addrs[0]);
}

#[test]
pub fn ledger_fails_to_starts_with_tx_with_input() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let faucet = Wallet::new("faucet", Value(10));
    let receiver = Wallet::new("receiver", Value(10));

    let faucet_fragment = create_initial_transaction(&faucet);
    let fragment_with_input =
        InitialFaultTolerantTxBuilder::new(faucet, receiver).transaction_with_input_output();

    assert_eq!(
        LedgerBuilder::from_config(config)
            .fragments(&[faucet_fragment, fragment_with_input])
            .build()
            .err()
            .unwrap(),
        Block0(Block0Error::TransactionHasInput)
    );
}

#[test]
pub fn ledger_fails_to_starts_with_cert_with_input() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let faucet = Wallet::new("faucet", Value(10));
    let faucet_fragment = create_initial_transaction(&faucet);
    let fragment_with_input =
        InitialFaultTolerantTxCertBuilder::new(VoteTestGen::vote_plan().into(), faucet)
            .transaction_with_input_only();

    assert_eq!(
        LedgerBuilder::from_config(config)
            .fragments(&[faucet_fragment, fragment_with_input])
            .build()
            .err()
            .unwrap(),
        Block0(Block0Error::CertTransactionHasInput)
    );
}

#[test]
pub fn ledger_fails_to_starts_with_owner_stake_delegation() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);
    let owner_delegation_cert =
        create_initial_stake_pool_owner_delegation(DelegationType::NonDelegated);

    assert_eq!(
        LedgerBuilder::from_config(config)
            .certs(&[owner_delegation_cert])
            .build()
            .err()
            .unwrap(),
        Block0(Block0Error::HasOwnerStakeDelegation)
    );
}

#[test]
pub fn ledger_fails_to_starts_with_cert_with_output() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let faucet = Wallet::new("faucet", Value(10));
    let faucet_fragment = create_initial_transaction(&faucet);
    let fragment_with_input =
        InitialFaultTolerantTxCertBuilder::new(VoteTestGen::vote_plan().into(), faucet)
            .transaction_with_output_only();

    assert_eq!(
        LedgerBuilder::from_config(config)
            .fragments(&[faucet_fragment, fragment_with_input])
            .build()
            .err()
            .unwrap(),
        Block0(Block0Error::CertTransactionHasOutput)
    );
}

#[test]
pub fn ledger_fails_to_starts_with_cert_with_witness_only() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let faucet = Wallet::new("faucet", Value(10));
    let receiver = Wallet::new("receiver", Value(10));
    let faucet_fragment = create_initial_transaction(&faucet);
    let fragment_with_input =
        InitialFaultTolerantTxBuilder::new(faucet, receiver).transaction_with_witness_only();

    let _ = LedgerBuilder::from_config(config)
        .fragments(&[faucet_fragment, fragment_with_input])
        .build()
        .unwrap();
}

#[test]
pub fn ledger_fails_to_start_with_old_utxo_declaration() {
    let config = ConfigBuilder::new().with_discrimination(Discrimination::Test);

    let old_address = OldAddressBuilder::build_utxo_declaration(Some(254));
    let fragment = Fragment::OldUtxoDeclaration(old_address.clone());
    let ledger = LedgerBuilder::from_config(config)
        .fragments(&[fragment.clone()])
        .build()
        .unwrap();

    let output = ledger
        .ledger
        .oldutxos
        .get(&fragment.id(), 0)
        .unwrap()
        .output
        .clone();
    let (address, value) = (output.address, output.value);

    assert_eq!((address, value), old_address.addrs[0]);
}
