use crate::{
    account::Identifier,
    certificate::{
        Certificate, PoolManagement, PoolOwnersSigned, PoolRegistration, PoolRetirement,
        StakeDelegation,
    },
    testing::address::AddressData,
    transaction::AccountIdentifier,
};
use chain_time::timeline::TimeOffsetSeconds;
use chain_time::units::DurationSeconds;
use typed_bytes::ByteBuilder;

pub fn build_stake_delegation_cert(
    stake_pool: &PoolRegistration,
    delegate_from: &AddressData,
) -> Certificate {
    let account_id =
        AccountIdentifier::from_single_account(Identifier::from(delegate_from.delegation_key()));
    let mut cert = Certificate::StakeDelegation(StakeDelegation {
        account_id: account_id,
        pool_id: stake_pool.to_id(),
    });
    cert
}

pub fn build_stake_pool_registration_cert(
    stake_pool: &PoolRegistration,
    owner: &AddressData,
) -> Certificate {
    let mut cert = Certificate::PoolRegistration(stake_pool.clone());
    cert
}

pub fn build_stake_pool_retirement_cert(
    stake_pool: PoolRegistration,
    owners: &[AddressData],
) -> Certificate {
    let retirement = PoolRetirement {
        pool_id: stake_pool.to_id(),
        retirement_time: DurationSeconds(0).into(),
    };

    let mut signatures = Vec::new();
    for (i, owner) in owners.iter().enumerate() {
        let byte_array = retirement.serialize_in(ByteBuilder::new()).finalize();
        signatures.push((i as u8, owner.private_key().sign(&byte_array)));
    }

    let mut cert = Certificate::PoolManagement(PoolManagement::Retirement(PoolOwnersSigned {
        inner: retirement,
        signatures: signatures,
    }));
    cert
}
