//! Generic account like accounting
//!
//! This is effectively an immutable clonable-HAMT of bank style account,
//! which contains a non negative value representing your balance with the
//! identifier of this account as key.

pub mod account_state;
pub mod last_rewards;
pub mod spending;

use crate::tokens::identifier::TokenIdentifier;
use crate::{date::Epoch, value::*};
use imhamt::{Hamt, InsertError, UpdateError};
use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Debug};
use std::hash::Hash;
use thiserror::Error;

pub use account_state::*;
pub use last_rewards::LastRewards;
pub use spending::{SpendingCounter, SpendingCounterIncreasing};

#[cfg(any(test, feature = "property-test-api"))]
pub mod test;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum LedgerError {
    #[error("Account does not exist")]
    NonExistent,
    #[error("Account already exists")]
    AlreadyExists,
    #[error("Removed account is not empty")]
    NonZero,
    #[error("Spending credential invalid")]
    SpendingCredentialInvalid,
    #[error("Value calculation failed")]
    ValueError(#[from] ValueError),
}

impl From<UpdateError<LedgerError>> for LedgerError {
    fn from(e: UpdateError<LedgerError>) -> Self {
        match e {
            UpdateError::KeyNotFound => LedgerError::NonExistent,
            UpdateError::ValueCallbackError(v) => v,
        }
    }
}

impl From<InsertError> for LedgerError {
    fn from(e: InsertError) -> Self {
        match e {
            InsertError::EntryExists => LedgerError::AlreadyExists,
        }
    }
}

/// The public ledger of all accounts associated with their current state
#[derive(Clone, PartialEq, Eq)]
pub struct Ledger<ID: Hash + Eq, Extra>(Hamt<DefaultHasher, ID, AccountState<Extra>>);

impl<ID: Clone + Eq + Hash, Extra: Clone> Default for Ledger<ID, Extra> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ID: Clone + Eq + Hash, Extra: Clone> Ledger<ID, Extra> {
    /// Create a new empty account ledger
    pub fn new() -> Self {
        Ledger(Hamt::new())
    }

    /// Add a new account into this ledger.
    ///
    /// If the identifier is already present, error out.
    pub fn add_account(
        &self,
        identifier: ID,
        initial_value: Value,
        extra: Extra,
    ) -> Result<Self, LedgerError> {
        self.0
            .insert(identifier, AccountState::new(initial_value, extra))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Set the delegation of an account in this ledger
    pub fn set_delegation(
        &self,
        identifier: &ID,
        delegation: &DelegationType,
    ) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| {
                Ok(Some(st.set_delegation(delegation.clone())))
            })
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// check if an account already exist
    #[inline]
    pub fn exists(&self, identifier: &ID) -> bool {
        self.0.contains_key(identifier)
    }

    /// Get account state
    ///
    /// If the identifier does not match any account, error out
    pub fn get_state(&self, account: &ID) -> Result<&AccountState<Extra>, LedgerError> {
        self.0.lookup(account).ok_or(LedgerError::NonExistent)
    }

    /// Remove an account from this ledger
    ///
    /// If the account still have value > 0, then error
    pub fn remove_account(&self, identifier: &ID) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| {
                if st.value == Value::zero() {
                    Ok(None)
                } else {
                    Err(LedgerError::NonZero)
                }
            })
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Add value to an existing account.
    ///
    /// If the account doesn't exist, error out.
    pub fn add_value(&self, identifier: &ID, value: Value) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| st.add(value).map(Some))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    /// Add value to an existing account.
    ///
    /// If the account doesn't exist, it creates it with the value
    pub fn add_value_or_account(
        &self,
        identifier: &ID,
        value: Value,
        extra: Extra,
    ) -> Result<Self, ValueError> {
        self.0
            .insert_or_update(identifier.clone(), AccountState::new(value, extra), |st| {
                st.add_value(value).map(Some)
            })
            .map(Ledger)
    }

    /// Add rewards to an existing account.
    ///
    /// If the account doesn't exist, it creates it with the value
    pub fn add_rewards_to_account(
        &self,
        identifier: &ID,
        epoch: Epoch,
        value: Value,
        extra: Extra,
    ) -> Result<Self, ValueError> {
        self.0
            .insert_or_update(
                identifier.clone(),
                AccountState::new_reward(epoch, value, extra),
                |st| st.add_rewards(epoch, value).map(Some),
            )
            .map(Ledger)
    }

    /// Subtract value to an existing account.
    ///
    /// If the account doesn't exist, or that the value would become negative, errors out.
    pub fn remove_value(
        &self,
        identifier: &ID,
        spending: SpendingCounter,
        value: Value,
    ) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| st.sub(spending, value))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    pub fn get_total_value(&self) -> Result<Value, ValueError> {
        let values = self
            .0
            .iter()
            .map(|(_, account_state)| account_state.value());
        Value::sum(values)
    }

    pub fn token_add(
        &self,
        identifier: &ID,
        token: TokenIdentifier,
        value: Value,
    ) -> Result<Self, LedgerError> {
        self.0
            .update(identifier, |st| st.token_add(token, value).map(Some))
            .map(Ledger)
            .map_err(|e| e.into())
    }

    #[cfg(feature = "evm")]
    pub fn evm_insert_or_update(
        &self,
        identifier: &ID,
        value: Value,
        evm_state: chain_evm::state::AccountState,
        extra: Extra,
    ) -> Result<Self, LedgerError> {
        self.0
            .insert_or_update(
                identifier.clone(),
                AccountState::new_evm(evm_state.clone(), value, extra),
                |st| {
                    Ok(Some(AccountState {
                        evm_state,
                        value,
                        ..st.clone()
                    }))
                },
            )
            .map(Ledger)
    }

    pub fn iter(&self) -> Iter<'_, ID, Extra> {
        Iter(self.0.iter())
    }
}

impl<ID: Clone + Eq + Hash + Debug, Extra: Clone + Debug> Debug for Ledger<ID, Extra> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.0
                .iter()
                .map(|(id, account)| (id.clone(), account.clone()))
                .collect::<Vec<(ID, AccountState<Extra>)>>()
        )
    }
}

impl<ID: Clone + Eq + Hash, Extra: Clone> std::iter::FromIterator<(ID, AccountState<Extra>)>
    for Ledger<ID, Extra>
{
    fn from_iter<I: IntoIterator<Item = (ID, AccountState<Extra>)>>(iter: I) -> Self {
        Ledger(Hamt::from_iter(iter))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        account::{Identifier, Ledger},
        certificate::{PoolId, PoolRegistration},
        testing::{arbitrary::utils as arbitrary_utils, arbitrary::AverageValue, TestGen},
        value::Value,
    };

    use proptest::{prop_assert, prop_assert_eq, prop_assume};
    use quickcheck::{Arbitrary, Gen};
    use std::collections::HashSet;
    use std::iter;
    use test_strategy::proptest;

    impl Arbitrary for Ledger {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let account_size = std::cmp::max(usize::arbitrary(gen), 1);
            let stake_pool_size =
                std::cmp::min(account_size, usize::arbitrary(gen) % account_size + 1);
            let arbitrary_accounts_ids = iter::from_fn(|| Some(Identifier::arbitrary(gen)))
                .take(account_size)
                .collect::<HashSet<Identifier>>();

            let arbitrary_stake_pools = iter::from_fn(|| Some(PoolRegistration::arbitrary(gen)))
                .take(stake_pool_size)
                .collect::<Vec<_>>();

            let voting_tokens_size = usize::arbitrary(gen);
            let arbitrary_voting_tokens = iter::from_fn(|| Some(TokenIdentifier::arbitrary(gen)))
                .take(voting_tokens_size)
                .collect::<HashSet<_>>();

            let mut ledger = Ledger::new();

            // Add all arbitrary accounts
            for account_id in arbitrary_accounts_ids.iter() {
                ledger = ledger
                    .add_account(account_id.clone(), AverageValue::arbitrary(gen).into(), ())
                    .unwrap();

                for token in &arbitrary_voting_tokens {
                    // TODO: maybe less probability is better (for performance)
                    if bool::arbitrary(gen) {
                        ledger = ledger
                            .token_add(account_id, token.clone(), Value::arbitrary(gen))
                            .unwrap();
                    }
                }
            }

            // Choose random subset of arbitraty accounts and delegate stake to random stake pools
            for account_id in
                arbitrary_utils::choose_random_set_subset(&arbitrary_accounts_ids, gen)
            {
                let random_stake_pool =
                    arbitrary_utils::choose_random_item(&arbitrary_stake_pools, gen);
                ledger = ledger
                    .set_delegation(
                        &account_id,
                        &DelegationType::Full(random_stake_pool.to_id()),
                    )
                    .unwrap();
            }
            ledger
        }
    }

    #[proptest(proptest::prelude::ProptestConfig {
        cases: 10,
        max_flat_map_regens: 1,
        ..Default::default()
    })]
    fn account_ledger_test(
        mut ledger: Ledger,
        account_id: Identifier,
        #[strategy(Value::non_zero_strategy())] value: Value,
        stake_pool_id: PoolId,
    ) {
        prop_assume!(!ledger.exists(&account_id));

        let initial_total_value = ledger.get_total_value().unwrap();

        // add new account
        ledger = ledger.add_account(account_id.clone(), value, ()).unwrap();

        // add account again should throw an error
        ledger
            .add_account(account_id.clone(), value, ())
            .unwrap_err();
        prop_assert!(
            ledger.exists(&account_id),
            "Account with id {} should exist",
            account_id
        );
        prop_assert!(
            ledger.iter().any(|(x, _)| *x == account_id),
            "Account with id {} should be listed amongst other",
            account_id
        );

        // verify total value was increased
        prop_assert_eq!(
            (initial_total_value + value).unwrap(),
            ledger.get_total_value().unwrap()
        );

        // set delegation to stake pool
        ledger = ledger
            .set_delegation(&account_id, &DelegationType::Full(stake_pool_id.clone()))
            .unwrap();

        // verify total value is still the same
        prop_assert_eq!(
            (initial_total_value + value).unwrap(),
            ledger.get_total_value().unwrap()
        );

        // add value to account
        let result = ledger.add_value(&account_id, value);
        prop_assume!(!matches!(
            result,
            Err(LedgerError::ValueError(ValueError::Overflow))
        ));
        ledger = ledger.add_value(&account_id, value).unwrap();

        // verify total value was increased
        prop_assert_eq!(
            (initial_total_value + (value + value).unwrap()).unwrap(),
            ledger.get_total_value().unwrap()
        );

        //add reward to account
        ledger = ledger
            .add_rewards_to_account(&account_id, 0, value, ())
            .unwrap();

        let value_after_reward = Value(value.0 * 3);
        // verify total value was increased
        prop_assert_eq!(
            (initial_total_value + value_after_reward).unwrap(),
            ledger.get_total_value().unwrap()
        );

        let mut spending_counter = SpendingCounter::zero();
        //verify account state
        match ledger.get_state(&account_id) {
            Ok(account_state) => {
                let expected_account_state = AccountState {
                    spending: SpendingCounterIncreasing::default(),
                    last_rewards: LastRewards {
                        epoch: 0,
                        reward: value,
                    },
                    delegation: DelegationType::Full(stake_pool_id),
                    value: value_after_reward,
                    tokens: Hamt::new(),
                    #[cfg(feature = "evm")]
                    evm_state: chain_evm::state::AccountState::default(),
                    extra: (),
                };

                prop_assert_eq!(account_state, &expected_account_state);
            }
            Err(err) => {
                panic!(
                    "Get state for id {} should be successful: {:?}",
                    account_id, err
                )
            }
        }

        // remove value from account
        ledger = ledger
            .remove_value(&account_id, spending_counter, value)
            .unwrap();

        spending_counter = spending_counter.increment();
        let value_before_reward = Value(value.0 * 2);
        // verify total value was decreased
        prop_assert_eq!(
            (initial_total_value + value_before_reward).unwrap(),
            ledger.get_total_value().unwrap()
        );

        // verify remove account fails beause account still got some founds
        ledger.remove_account(&account_id).unwrap_err();

        // removes all funds from account
        ledger = ledger
            .remove_value(&account_id, spending_counter, value_before_reward)
            .unwrap();

        // commented line to prevent a warning, but it should be updated to reflect the correct state of spending credential
        // spending_counter = spending_counter.increment();

        // removes account
        ledger = ledger.remove_account(&account_id).unwrap();

        prop_assert!(!ledger.exists(&account_id), "account should not exist");
        prop_assert!(
            !ledger.iter().any(|(x, _)| *x == account_id),
            "Account with id {:?} should not be listed amongst accounts",
            account_id
        );
        prop_assert_eq!(
            initial_total_value,
            ledger.get_total_value().unwrap(),
            "total funds is not equal to initial total_value"
        );

        // Account state should be none

        prop_assert!(ledger.get_state(&account_id).is_err());
    }

    #[proptest]
    fn ledger_total_value_is_correct_after_remove_value(
        id: Identifier,
        account_state: AccountState<()>,
        value_to_remove: Value,
    ) {
        let mut ledger = Ledger::new();
        ledger = ledger
            .add_account(id.clone(), account_state.value(), ())
            .unwrap();
        let result = ledger.remove_value(&id, SpendingCounter::zero(), value_to_remove);
        let expected_result = account_state.value() - value_to_remove;
        match (result, expected_result) {
            (Err(_), Err(_)) => verify_total_value(ledger, account_state.value()),
            (Ok(ledger), Ok(value)) => verify_total_value(ledger, value),
            _ => panic!(),
        }
    }

    fn verify_total_value(ledger: Ledger, value: Value) {
        if ledger.get_total_value().unwrap() != value {
            panic!(
                "Wrong total value got {:?}, while expecting {:?}",
                ledger.get_total_value(),
                value
            )
        }
    }

    #[proptest]
    fn ledger_removes_account_only_if_zeroed(id: Identifier, account_state: AccountState<()>) {
        let mut ledger = Ledger::new();
        ledger = ledger
            .add_account(id.clone(), account_state.value(), ())
            .unwrap();
        let result = ledger.remove_account(&id);
        let expected_zero = account_state.value() == Value::zero();
        match (result, expected_zero) {
            (Err(_), false) => verify_account_exists(&ledger, &id),
            (Ok(ledger), true) => verify_account_does_not_exist(&ledger, &id),
            _ => panic!(),
        }
    }

    fn verify_account_exists(ledger: &Ledger, id: &Identifier) {
        if !ledger.exists(id) {
            panic!("Account ({:?}) does not exist, while it should", &id)
        }
    }

    fn verify_account_does_not_exist(ledger: &Ledger, id: &Identifier) {
        if ledger.exists(id) {
            panic!("Account ({:?}) exists, while it should not", &id)
        }
    }

    #[test]
    pub fn add_value_or_account_test() {
        let ledger = Ledger::new();
        assert!(ledger
            .add_value_or_account(&TestGen::identifier(), Value(10), ())
            .is_ok());
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod prop_impl {
    use proptest::collection::{hash_map, vec};
    use proptest::prelude::*;

    use crate::account::{Identifier, Ledger};
    use crate::certificate::PoolRegistration;
    use crate::testing::utils::proptest::random_non_zero_subset;
    use crate::testing::{average_value, maybe_average_value};
    use crate::tokens::identifier::TokenIdentifier;

    use super::DelegationType;

    fn ledger_strat() -> impl Strategy<Value = Ledger> {
        // this differs from quickcheck impl:
        //
        // quickcheck impl uses an arbitrary usize, but in practice it doesn't ever come close to
        // usize::MAX
        //
        // proptest will actually try to use usize::MAX if using any::<usize>() as a strategy,
        // which causes collections to fail to allocate
        (1..100usize, 0..100usize, 0..100usize).prop_flat_map(
            |(num_accounts, mut num_pools, num_tokens)| {
                num_pools = num_pools % num_accounts + 1;
                let ids_and_values = hash_map(any::<Identifier>(), average_value(), num_accounts);
                let stake_pools = vec(any::<PoolRegistration>(), num_pools);
                let tokens = hash_map(any::<TokenIdentifier>(), maybe_average_value(), num_tokens);
                let random_stake_pool_indices = vec(0..num_pools, num_accounts);
                (
                    ids_and_values,
                    stake_pools,
                    tokens,
                    random_stake_pool_indices,
                )
                    .prop_flat_map(
                        |(ids_and_values, stake_pools, tokens, indices)| {
                            let random_ids = random_non_zero_subset(Just(ids_and_values.clone()));
                            let mut ledger = Ledger::new();
                            for (account_id, value) in &ids_and_values {
                                ledger =
                                    ledger.add_account(account_id.clone(), *value, ()).unwrap();

                                for (token, value) in &tokens {
                                    if let Some(value) = value {
                                        ledger = ledger
                                            .token_add(account_id, token.clone(), *value)
                                            .unwrap();
                                    }
                                }
                            }

                            (random_ids, Just(ledger)).prop_map(move |(random_ids, mut ledger)| {
                                for (i, (account_id, _)) in random_ids.iter().enumerate() {
                                    let index = indices[i];
                                    let pool_id = stake_pools.get(index).unwrap().to_id();
                                    ledger = ledger
                                        .set_delegation(account_id, &DelegationType::Full(pool_id))
                                        .unwrap()
                                }
                                ledger
                            })
                        },
                    )
            },
        )
    }

    impl Arbitrary for Ledger {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            ledger_strat().boxed()
        }
    }
}
