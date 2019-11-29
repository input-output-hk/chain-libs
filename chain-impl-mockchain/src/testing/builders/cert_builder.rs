use crate::{
    account::{DelegationType, Identifier},
    certificate::{
        Certificate, OwnerStakeDelegation, PoolId, PoolRegistration, PoolRetirement,
        StakeDelegation,
    },
    testing::data::AddressData,
    transaction::UnspecifiedAccountIdentifier,
};
use chain_time::units::DurationSeconds;

pub fn build_stake_delegation_cert(
    stake_pool: &PoolRegistration,
    delegate_from: &AddressData,
) -> Certificate {
    let account_id = UnspecifiedAccountIdentifier::from_single_account(Identifier::from(
        delegate_from.delegation_key(),
    ));
    Certificate::StakeDelegation(StakeDelegation {
        account_id: account_id,
        delegation: DelegationType::Full(stake_pool.to_id()),
    })
}

pub fn build_stake_pool_registration_cert(stake_pool: &PoolRegistration) -> Certificate {
    Certificate::PoolRegistration(stake_pool.clone())
}

pub fn build_owner_stake_full_delegation(stake_pool: PoolId) -> Certificate {
    Certificate::OwnerStakeDelegation(OwnerStakeDelegation {
        delegation: DelegationType::Full(stake_pool),
    })
}

pub fn build_no_stake_delegation() -> Certificate {
    Certificate::OwnerStakeDelegation(OwnerStakeDelegation {
        delegation: DelegationType::NonDelegated
    })
}

pub fn build_owner_stake_delegation(delegation_type: DelegationType) -> Certificate {
    Certificate::OwnerStakeDelegation(OwnerStakeDelegation {
        delegation: delegation_type
    })
}

pub fn build_stake_pool_retirement_cert(
    pool_id: PoolId,
    start_validity: u64,
) -> Certificate {
    let retirement = PoolRetirement {
        pool_id: pool_id,
        retirement_time: DurationSeconds(start_validity).into(),
    };

    Certificate::PoolRetirement(retirement)
}
