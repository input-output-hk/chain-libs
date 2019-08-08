use crate::{
    block::HeaderHash,
    certificate::Certificate,
    fragment::{Fragment, FragmentId},
    leadership::bft::LeaderId,
    stake::StakePoolInfo,
    testing::address::AddressData,
    testing::builders::{cert_builder, tx_builder, tx_cert_builder},
    transaction::*,
};
use chain_addr::{Address, Discrimination};
use chain_core::property::Fragment as FragmentProperty;
use std::vec::Vec;

pub fn create_initial_transaction(output: Output<Address>) -> Fragment {
    let mut builder = tx_builder::TransactionBuilder::new();
    let authenticator = builder.with_output(output).authenticate();
    authenticator.as_message()
}

pub fn create_initial_transactions(outputs: &Vec<Output<Address>>) -> Fragment {
    let mut builder = tx_builder::TransactionBuilder::new();
    let authenticator = builder.with_outputs(outputs.to_vec()).authenticate();
    authenticator.as_message()
}

pub fn create_stake_delegation_cert(
    block0_hash: &HeaderHash,
    stake_owner: &AddressData,
    stake_pool: &StakePoolInfo,
) -> (
    FragmentId,
    AuthenticatedTransaction<chain_addr::Address, Certificate>,
) {
    let signed_cert = cert_builder::build_stake_delegation_cert(stake_pool, stake_owner);
    let signed_tx_cert = create_cert_transaction(signed_cert, block0_hash, stake_owner);
    
    let fragment_id = Fragment::Certificate(signed_tx_cert.clone()).id();
    (fragment_id, signed_tx_cert)
}

pub fn create_register_stake_pool_cert(
    block0_hash: &HeaderHash,
    owner: &AddressData,
    stake_pool: &StakePoolInfo,
) -> (
    FragmentId,
    AuthenticatedTransaction<chain_addr::Address, Certificate>,
) {
    let signed_cert = cert_builder::build_stake_pool_registration_cert(stake_pool, owner);
    let signed_tx_cert = create_cert_transaction(signed_cert, block0_hash, owner);

    let fragment_id = Fragment::Certificate(signed_tx_cert.clone()).id();
    (fragment_id, signed_tx_cert)
}

pub fn create_retire_stake_pool_cert(
    block0_hash: &HeaderHash,
    owner: &AddressData,
    stake_pool: &StakePoolInfo,
) -> (
    FragmentId,
    AuthenticatedTransaction<chain_addr::Address, Certificate>,
) {
    let signed_cert = cert_builder::build_stake_pool_registration_cert(stake_pool, owner);
    let signed_tx_cert = create_cert_transaction(signed_cert, block0_hash, owner);

    let fragment_id = Fragment::Certificate(signed_tx_cert.clone()).id();
    (fragment_id, signed_tx_cert)
}

fn create_cert_transaction(
    signed_cert: Certificate,
    block0_hash: &HeaderHash,
    address_data: &AddressData,
) -> AuthenticatedTransaction<Address, Certificate> {
    tx_cert_builder::TransactionCertBuilder::new()
        .with_certificate(signed_cert)
        .authenticate()
        .with_witness(&block0_hash, &address_data)
        .seal()
}
