use crate::{
    certificate::{VoteCast, VotePlan, VotePlanId},
    date::BlockDate,
    transaction::UnspecifiedAccountIdentifier,
    vote::{VoteError, VotePlanManager},
};
use imhamt::{Hamt, InsertError, UpdateError};
use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::fmt;
use thiserror::Error;

#[derive(Clone, PartialEq, Eq)]
pub struct VotePlanLedger {
    pub(crate) plans: Hamt<DefaultHasher, VotePlanId, (VotePlanManager, BlockDate)>,
    plans_by_end_date: BTreeMap<BlockDate, Vec<VotePlanId>>,
}

impl fmt::Debug for VotePlanLedger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {:?}",
            self.plans
                .iter()
                .map(|(id, (manager, date))| (id.clone(), (manager.clone(), date.clone())))
                .collect::<Vec<(VotePlanId, (VotePlanManager, BlockDate))>>(),
            self.plans_by_end_date
        )
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VotePlanLedgerError {
    #[error("cannot insert the vote plan {id}: {reason:?}")]
    VotePlanInsertionError { id: VotePlanId, reason: InsertError },

    #[error("cannot insert the vote plan {id}: {reason:?}")]
    VoteError {
        id: VotePlanId,
        reason: UpdateError<VoteError>,
    },

    #[error("Vote plan is set to finish in the passed ({vote_end}), current date {current_date}")]
    VotePlanVoteEndPassed {
        current_date: BlockDate,
        vote_end: BlockDate,
    },

    #[error("Vote plan already started ({vote_start}), current date {current_date}")]
    VotePlanVoteStartStartedAlready {
        current_date: BlockDate,
        vote_start: BlockDate,
    },
}

impl VotePlanLedger {
    pub fn new() -> Self {
        Self {
            plans: Hamt::new(),
            plans_by_end_date: BTreeMap::new(),
        }
    }

    /// garbage collect the vote plans that should no longer be tracked
    /// and return the new state
    ///
    /// the block_date is supposed to be the current block date for the
    /// new state.
    ///
    /// This function is not to use lightly as this will remove VotePlans
    /// that are still interesting to track down:
    ///
    /// * we still need to publish the vote result;
    /// * we still need to distribute the rewards?
    ///
    pub fn gc(&self, block_date: BlockDate) -> Self {
        let mut to_remove = self.plans_by_end_date.clone();
        let to_keep = to_remove.split_off(&block_date);

        let mut plans = self.plans.clone();
        for ids in to_remove.values() {
            for id in ids {
                plans = match plans.remove(id) {
                    Err(remove_error) => {
                        // it should not be possible to happen
                        // if it does then there is something else
                        // going on, maybe in the add_vote function?
                        unreachable!(
                            "It should not be possible to fail to remove an entry: {:?}",
                            remove_error
                        )
                    }
                    Ok(plans) => plans,
                };
            }
        }

        Self {
            plans,
            plans_by_end_date: to_keep,
        }
    }

    /// attempt to apply the vote to the appropriate Vote Proposal
    ///
    /// # errors
    ///
    /// can fail if:
    ///
    /// * the vote plan id does not exist;
    /// * the proposal's index does not exist;
    /// * it is no longer possible to vote (the date to vote expired)
    ///
    pub fn apply_vote(
        &self,
        block_date: BlockDate,
        identifier: UnspecifiedAccountIdentifier,
        vote: VoteCast,
    ) -> Result<Self, VotePlanLedgerError> {
        let id = vote.vote_plan().clone();

        let r = self.plans.update(&id, move |(v, _)| {
            v.vote(block_date, identifier, vote)
                .map(|v| Some((v, block_date)))
        });

        match r {
            Err(reason) => Err(VotePlanLedgerError::VoteError { reason, id }),
            Ok(plans) => Ok(Self {
                plans,
                plans_by_end_date: self.plans_by_end_date.clone(),
            }),
        }
    }

    /// add the vote plan in a new `VotePlanLedger`
    ///
    /// the given `VotePlanLedger` is not modified and instead a new `VotePlanLedger` is
    /// returned. They share read-only memory.
    ///
    /// # errors if
    ///
    /// * the vote_plan is set to finished votes in the past
    /// * the vote_plan has already started
    ///
    #[must_use = "This function does not modify the object, the result contains the resulted new version of the vote plan ledger"]
    pub fn add_vote_plan(
        &self,
        current_date: BlockDate,
        vote_plan: VotePlan,
    ) -> Result<Self, VotePlanLedgerError> {
        if current_date > vote_plan.vote_end() {
            return Err(VotePlanLedgerError::VotePlanVoteEndPassed {
                current_date,
                vote_end: vote_plan.vote_end(),
            });
        }

        if current_date > vote_plan.vote_start() {
            return Err(VotePlanLedgerError::VotePlanVoteStartStartedAlready {
                current_date,
                vote_start: vote_plan.vote_start(),
            });
        }

        let id = vote_plan.to_id();
        let end_date = vote_plan.committee_end();
        let manager = VotePlanManager::new(vote_plan);

        match self.plans.insert(id.clone(), (manager, end_date)) {
            Err(reason) => Err(VotePlanLedgerError::VotePlanInsertionError { id, reason }),
            Ok(plans) => {
                let mut plans_by_end_date = self.plans_by_end_date.clone();
                plans_by_end_date.entry(end_date).or_default().push(id);
                Ok(Self {
                    plans,
                    plans_by_end_date,
                })
            }
        }
    }

    /// apply the committee result for the associated vote plan
    ///
    /// TODO: this function is not implemented
    pub fn apply_committee_result(&self) -> Self {
        todo!()
    }
}

impl Default for VotePlanLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    use super::{VoteCast, VotePlan, VotePlanLedger, VotePlanLedgerError};
    use crate::block::BlockDate;
    use crate::testing::{TestGen, VoteTestGen};
    use chain_core::property::BlockDate as BlockDateProp;

    #[test]
    pub fn add_already_started_vote_plan() {
        let vote_plan_ledger = VotePlanLedger::new();
        let vote_plan_start = BlockDate::from_epoch_slot_id(1, 0);

        let vote_plan = VotePlan::new(
            vote_plan_start.clone(),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(3),
        );

        let current_date = BlockDate::from_epoch_slot_id(1, 1);

        assert_eq!(
            Err(VotePlanLedgerError::VotePlanVoteStartStartedAlready {
                current_date: current_date.clone(),
                vote_start: vote_plan_start.clone(),
            }),
            vote_plan_ledger.add_vote_plan(current_date, vote_plan)
        );
    }

    #[test]
    pub fn add_already_finished_vote_plan() {
        let vote_plan_ledger = VotePlanLedger::new();
        let vote_plan_finish = BlockDate::from_epoch_slot_id(2, 0);

        let vote_plan = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            vote_plan_finish,
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(3),
        );

        let current_date = BlockDate::from_epoch_slot_id(2, 1);

        assert_eq!(
            Err(VotePlanLedgerError::VotePlanVoteEndPassed {
                current_date: current_date.clone(),
                vote_end: vote_plan_finish,
            }),
            vote_plan_ledger.add_vote_plan(current_date, vote_plan)
        );
    }

    #[test]
    pub fn add_duplicated_vote_plan() {
        let mut vote_plan_ledger = VotePlanLedger::new();

        let vote_plan = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(3),
        );

        let current_date = BlockDate::from_epoch_slot_id(0, 9);
        vote_plan_ledger = vote_plan_ledger
            .add_vote_plan(current_date, vote_plan.clone())
            .expect("first vote plan should be successful");

        assert!(
            vote_plan_ledger
                .add_vote_plan(current_date, vote_plan)
                .is_err(),
            "should fail if we add duplicated vote plan"
        );
    }

    #[test]
    pub fn add_same_proposal_with_different_date() {
        let mut vote_plan_ledger = VotePlanLedger::new();
        let proposals = VoteTestGen::proposals(1);
        let vote_plan = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            proposals.clone(),
        );

        let vote_plan_2 = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 1),
            proposals.clone(),
        );

        let vote_plan_3 = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 1),
            BlockDate::from_epoch_slot_id(3, 0),
            proposals.clone(),
        );

        let vote_plan_4 = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 1),
            BlockDate::from_epoch_slot_id(2, 1),
            BlockDate::from_epoch_slot_id(3, 0),
            proposals.clone(),
        );

        let current_date = BlockDate::from_epoch_slot_id(0, 9);
        vote_plan_ledger = vote_plan_ledger
            .add_vote_plan(current_date, vote_plan)
            .expect("add first vote plan should be successful");
        vote_plan_ledger = vote_plan_ledger
            .add_vote_plan(current_date, vote_plan_2)
            .expect("add second vote plan should be successful");
        vote_plan_ledger = vote_plan_ledger
            .add_vote_plan(current_date, vote_plan_3)
            .expect("add third vote plan should be successful");
        vote_plan_ledger
            .add_vote_plan(current_date, vote_plan_4)
            .expect("add fourth vote plan should be successful");
    }

    #[test]
    pub fn apply_vote() {
        let vote_plan = default_vote_plan();
        let vote_plan_ledger = default_vote_plan_ledger(vote_plan.clone());
        let vote_date = BlockDate::from_epoch_slot_id(1, 1);

        let vote_cast_payload = VoteTestGen::vote_cast_payload();
        let id = TestGen::unspecified_account_identifier();

        let vote_cast = VoteCast::new(vote_plan.to_id(), 0, vote_cast_payload);
        assert!(vote_plan_ledger
            .apply_vote(vote_date, id, vote_cast)
            .is_ok());
    }

    #[test]
    pub fn apply_vote_for_nonexisting_vote_plan() {
        let nonexisting_vote_plan = default_vote_plan();
        let vote_plan = default_vote_plan();
        let vote_plan_ledger = default_vote_plan_ledger(vote_plan.clone());
        let vote_cast_payload = VoteTestGen::vote_cast_payload();
        let vote_cast = VoteCast::new(nonexisting_vote_plan.to_id(), 0, vote_cast_payload);
        let vote_date = BlockDate::from_epoch_slot_id(0, 9);
        let id = TestGen::unspecified_account_identifier();

        assert!(vote_plan_ledger
            .apply_vote(vote_date, id, vote_cast)
            .is_err());
    }

    #[test]
    pub fn apply_vote_for_nonexisting_proposal_index() {
        let vote_plan = default_vote_plan();
        let vote_plan_ledger = default_vote_plan_ledger(vote_plan.clone());
        let vote_cast_payload = VoteTestGen::vote_cast_payload();
        let vote_cast = VoteCast::new(vote_plan.to_id(), 100, vote_cast_payload);
        let vote_date = BlockDate::from_epoch_slot_id(0, 9);
        let id = TestGen::unspecified_account_identifier();

        assert!(vote_plan_ledger
            .apply_vote(vote_date, id, vote_cast)
            .is_err());
    }

    #[test]
    pub fn apply_vote_for_outside_voting_time_window() {
        let vote_plan = default_vote_plan();
        let vote_plan_ledger = default_vote_plan_ledger(vote_plan.clone());
        let vote_cast_payload = VoteTestGen::vote_cast_payload();
        let vote_cast = VoteCast::new(vote_plan.to_id(), 100, vote_cast_payload);

        let block_date_before_voting_start = BlockDate::from_epoch_slot_id(0, 100);
        let block_date_at_voting_finish = BlockDate::from_epoch_slot_id(2, 0);

        let first_id = TestGen::unspecified_account_identifier();
        let second_id = TestGen::unspecified_account_identifier();

        assert!(vote_plan_ledger
            .apply_vote(block_date_before_voting_start, first_id, vote_cast.clone())
            .is_err());
        assert!(vote_plan_ledger
            .apply_vote(block_date_at_voting_finish, second_id, vote_cast)
            .is_err());
    }

    #[test]
    pub fn gc_no_expired_plans() {
        let vote_plan = default_vote_plan();
        let mut vote_plan_ledger = default_vote_plan_ledger(vote_plan.clone());
        let block_date_at_voting_finish = BlockDate::from_epoch_slot_id(3, 0);

        vote_plan_ledger = vote_plan_ledger.gc(block_date_at_voting_finish);

        assert_eq!(
            vote_plan_ledger.plans.size(),
            1,
            "none vote plans should be garbage collected"
        );
    }

    #[test]
    pub fn gc_expired_plans() {
        let vote_plan = default_vote_plan();
        let mut vote_plan_ledger = default_vote_plan_ledger(vote_plan.clone());
        let block_date_before_voting_finish = BlockDate::from_epoch_slot_id(3, 1);

        vote_plan_ledger = vote_plan_ledger.gc(block_date_before_voting_finish);

        assert_eq!(
            vote_plan_ledger.plans.size(),
            0,
            "all vote plan should be garbage collected"
        );
    }

    fn default_vote_plan_ledger(initial_vote_plan: VotePlan) -> VotePlanLedger {
        let vote_plan_ledger = VotePlanLedger::new();
        let current_date = BlockDate::from_epoch_slot_id(0, 9);
        vote_plan_ledger
            .add_vote_plan(current_date, initial_vote_plan)
            .expect("add first vote plan should be successful")
    }

    fn default_vote_plan() -> VotePlan {
        VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(3),
        )
    }
}
