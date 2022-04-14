use crate::certificate::{PoolId, PoolRegistration, PoolRegistrationHash};
use crate::date::Epoch;
use crate::value::Value;
use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Debug};
use std::sync::Arc;

/// A structure that keeps track of stake keys and stake pools.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct PoolsState {
    pub(crate) stake_pools: Hamt<DefaultHasher, PoolId, PoolState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    AlreadyExists(PoolId),
    NotFound(PoolId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "property-test-api"),
    derive(test_strategy::Arbitrary)
)]
pub struct PoolLastRewards {
    pub epoch: Epoch,
    pub value_taxed: Value,
    pub value_for_stakers: Value,
}

impl PoolLastRewards {
    pub fn default() -> Self {
        PoolLastRewards {
            epoch: 0,
            value_taxed: Value::zero(),
            value_for_stakers: Value::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "property-test-api"),
    derive(test_strategy::Arbitrary)
)]
pub struct PoolState {
    pub last_rewards: PoolLastRewards,
    pub registration: Arc<PoolRegistration>,
}

impl PoolState {
    pub fn new(reg: PoolRegistration) -> Self {
        PoolState {
            last_rewards: PoolLastRewards::default(),
            registration: Arc::new(reg),
        }
    }

    pub fn current_pool_registration_hash(&self) -> PoolRegistrationHash {
        self.registration.to_id()
    }
}

impl Debug for PoolsState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.stake_pools
                .iter()
                .map(|(id, stake)| (id.clone(), stake.clone()))
                .collect::<Vec<(PoolId, PoolState)>>()
        )
    }
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PoolError::AlreadyExists(pool_id) => write!(
                f,
                "Block attempts to register pool '{:?}' which already exists",
                pool_id
            ),
            PoolError::NotFound(pool_id) => write!(
                f,
                "Block references a pool '{:?}' which does not exist",
                pool_id
            ),
        }
    }
}

impl std::error::Error for PoolError {}

impl PoolsState {
    pub fn new() -> Self {
        PoolsState {
            stake_pools: Hamt::new(),
        }
    }

    pub fn lookup(&self, id: &PoolId) -> Option<&PoolState> {
        self.stake_pools.lookup(id)
    }

    pub fn lookup_reg(&self, id: &PoolId) -> Option<&PoolRegistration> {
        self.stake_pools.lookup(id).map(|x| x.registration.as_ref())
    }

    pub fn stake_pool_ids(&self) -> impl Iterator<Item = PoolId> + '_ {
        self.stake_pools.iter().map(|(id, _)| id.clone())
    }

    pub fn stake_pool_exists(&self, pool_id: &PoolId) -> bool {
        self.stake_pools
            .lookup(pool_id)
            .map_or_else(|| false, |_| true)
    }

    pub fn stake_pool_get_state(&self, pool_id: &PoolId) -> Result<&PoolState, PoolError> {
        self.stake_pools
            .lookup(pool_id)
            .ok_or_else(|| PoolError::NotFound(pool_id.clone()))
    }

    pub fn stake_pool_set_state(
        &mut self,
        pool_id: &PoolId,
        pool_state: PoolState,
    ) -> Result<(), PoolError> {
        self.stake_pools = self
            .stake_pools
            .replace(pool_id, pool_state)
            .map(|r| r.0)
            .map_err(|_| PoolError::NotFound(pool_id.clone()))?;
        Ok(())
    }

    pub fn stake_pool_get(&self, pool_id: &PoolId) -> Result<&PoolRegistration, PoolError> {
        self.stake_pools
            .lookup(pool_id)
            .ok_or_else(|| PoolError::NotFound(pool_id.clone()))
            .map(|s| s.registration.as_ref())
    }

    pub fn stake_pool_set_rewards(
        &mut self,
        pool_id: &PoolId,
        epoch: Epoch,
        value_taxed: Value,
        value_for_stakers: Value,
    ) -> Result<(), PoolError> {
        let rw = PoolLastRewards {
            epoch,
            value_taxed,
            value_for_stakers,
        };
        self.stake_pools = self
            .stake_pools
            .replace_with(pool_id, |st| {
                let mut st = st.clone();
                st.last_rewards = rw;
                st
            })
            .map_err(|_| PoolError::NotFound(pool_id.clone()))?;
        Ok(())
    }

    pub fn register_stake_pool(&self, owner: PoolRegistration) -> Result<Self, PoolError> {
        let id = owner.to_id();
        let new_pools = self
            .stake_pools
            .insert(id.clone(), PoolState::new(owner))
            .map_err(|_| PoolError::AlreadyExists(id))?;
        Ok(PoolsState {
            stake_pools: new_pools,
        })
    }

    pub fn deregister_stake_pool(&self, pool_id: &PoolId) -> Result<Self, PoolError> {
        Ok(PoolsState {
            stake_pools: self
                .stake_pools
                .remove(pool_id)
                .map_err(|_| PoolError::NotFound(pool_id.clone()))?,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::certificate::PoolRegistration;
    use proptest::{prop_assert, prop_assume};
    use quickcheck::{Arbitrary, Gen};
    use std::iter;
    use test_strategy::proptest;

    impl Arbitrary for PoolsState {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let size = usize::arbitrary(gen);
            let arbitrary_stake_pools: Vec<PoolRegistration> =
                iter::from_fn(|| Some(PoolRegistration::arbitrary(gen)))
                    .take(size)
                    .collect();
            let mut delegation_state = PoolsState::new();
            for stake_pool in arbitrary_stake_pools {
                delegation_state = delegation_state.register_stake_pool(stake_pool).unwrap();
            }
            delegation_state
        }
    }

    impl Arbitrary for PoolState {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let registration = Arc::new(PoolRegistration::arbitrary(gen));
            PoolState {
                last_rewards: PoolLastRewards::arbitrary(gen),
                registration,
            }
        }
    }

    impl Arbitrary for PoolLastRewards {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            PoolLastRewards {
                value_for_stakers: Value(u64::arbitrary(gen)),
                value_taxed: Value(u64::arbitrary(gen)),
                epoch: u32::arbitrary(gen),
            }
        }
    }

    #[proptest]
    fn delegation_state_tests(delegation_state: PoolsState, stake_pool: PoolRegistration) {
        // it's possible (but unlikely) that the randomly generated pool will already contain the
        // id
        prop_assume!(delegation_state
            .stake_pool_ids()
            .all(|x| x != stake_pool.to_id()));
        // register stake pool first time should be ok
        let delegation_state = delegation_state
            .register_stake_pool(stake_pool.clone())
            .unwrap();

        // register stake pool again should throw error
        delegation_state
            .register_stake_pool(stake_pool.clone())
            .unwrap_err();

        let stake_pool_id = stake_pool.to_id();

        // stake pool should be in collection
        prop_assert!(delegation_state
            .stake_pool_ids()
            .any(|x| x == stake_pool_id));

        // stake pool should exist in collection
        prop_assert!(delegation_state.stake_pool_exists(&stake_pool_id));

        // deregister stake pool should be ok
        let delegation_state = delegation_state
            .deregister_stake_pool(&stake_pool_id)
            .unwrap();

        // deregister stake pool again should throw error
        delegation_state
            .deregister_stake_pool(&stake_pool_id)
            .unwrap_err();

        // stake pool should not exist in collection
        prop_assert!(!delegation_state.stake_pool_exists(&stake_pool_id));

        // stake pool should not be in collection
        prop_assert!(!delegation_state
            .stake_pool_ids()
            .any(|x| x == stake_pool_id));
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod prop_impls {
    use proptest::arbitrary::StrategyFor;
    use proptest::collection::{vec, VecStrategy};
    use proptest::prelude::*;
    use proptest::strategy::Map;

    use crate::certificate::PoolRegistration;

    use super::PoolsState;

    impl Arbitrary for PoolsState {
        type Parameters = ();
        type Strategy =
            Map<VecStrategy<StrategyFor<PoolRegistration>>, fn(Vec<PoolRegistration>) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            vec(any::<PoolRegistration>(), 0..10000).prop_map(|pools| {
                let mut delegation_state = PoolsState::new();
                for stake_pool in pools {
                    delegation_state = delegation_state.register_stake_pool(stake_pool).unwrap();
                }
                delegation_state
            })
        }
    }
}
