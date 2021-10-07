use crate::account::Identifier;
use crate::vote::Weight;

use imhamt::InsertError;

use std::collections::hash_map::DefaultHasher;

use imhamt::Hamt;
use thiserror::Error;

/// The structure that holds the weight of a vote per each user as well as the
/// total weight of all votes. Weights are meant to be immutable.
#[derive(Clone, PartialEq, Eq)]
pub struct VotingPowerProvider {
    voting_power_by_account: Hamt<DefaultHasher, Identifier, Weight>,
    total_power: Weight,
}

#[derive(Debug, Error)]
#[error("Voting power is already set for this account")]
pub struct VotingPowerAlreadySet;

impl VotingPowerProvider {
    pub fn new() -> Self {
        Self {
            voting_power_by_account: Hamt::new(),
            total_power: 0u64.into(),
        }
    }

    pub fn set(
        &mut self,
        account_id: Identifier,
        voting_power: Weight,
    ) -> Result<(), VotingPowerAlreadySet> {
        self.voting_power_by_account = self
            .voting_power_by_account
            .insert(account_id, voting_power)
            .map_err(|err| match err {
                InsertError::EntryExists => VotingPowerAlreadySet,
            })?;
        self.total_power = self.total_power.saturating_add(voting_power);
        Ok(())
    }

    /// The amount of voting power per account.
    pub fn get(&self, account_id: &Identifier) -> Option<Weight> {
        self.voting_power_by_account.lookup(account_id).cloned()
    }

    /// The total amount of voting power.
    pub fn total(&self) -> Weight {
        self.total_power
    }
}
