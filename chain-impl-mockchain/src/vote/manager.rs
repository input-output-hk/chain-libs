use crate::{
    certificate::{Proposal, VoteCast, VoteCastPayload, VotePlan, VotePlanId},
    date::BlockDate,
    transaction::UnspecifiedAccountIdentifier,
};
use imhamt::Hamt;
use std::{collections::hash_map::DefaultHasher, sync::Arc};
use thiserror::Error;

/// Manage the vote plan and the associated votes in the ledger
///
/// this structure manage the lifespan of the vote plan, the votes
/// casted and the associated parameters
#[derive(Clone, PartialEq, Eq)]
pub struct VotePlanManager {
    id: VotePlanId,
    plan: Arc<VotePlan>,

    proposal_managers: ProposalManagers,
}

#[derive(Clone, PartialEq, Eq)]
struct ProposalManagers(Vec<ProposalManager>);

#[derive(Clone, PartialEq, Eq)]
struct ProposalManager {
    votes_by_voters: Hamt<DefaultHasher, UnspecifiedAccountIdentifier, VoteCastPayload>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VoteError {
    #[error("Invalid vote plan, expected {expected}")]
    InvalidVotePlan {
        expected: VotePlanId,
        vote: VoteCast,
    },

    #[error("It is not possible to vote at the moment for the proposals, time to vote is between {start} to {end}.")]
    VoteTimeElapsed {
        start: BlockDate,
        end: BlockDate,
        vote: VoteCast,
    },

    #[error("Invalid vote proposal, only {num_proposals} available in the vote plan")]
    InvalidVoteProposal {
        num_proposals: usize,
        vote: VoteCast,
    },
}

impl ProposalManager {
    /// construct a `ProposalManager` to track down the votes associated to this
    /// proposal.
    ///
    /// the proposal is passed on as parameter so we could add some form
    /// of verification in the future about the content of the vote (if
    /// possible : ZK is not necessarily allowing this).
    ///
    fn new(_proposal: &Proposal) -> Self {
        Self {
            votes_by_voters: Hamt::new(),
        }
    }

    /// apply the given vote cast to the proposal
    ///
    /// if there is already a vote present for this proposal it will
    /// simply replace the previously set one
    ///
    #[must_use = "Add the vote in a new ProposalManager, does not modify self"]
    pub fn vote(
        &self,
        identifier: UnspecifiedAccountIdentifier,
        cast: VoteCast,
    ) -> Result<Self, VoteError> {
        let payload = cast.into_payload();

        // we don't mind if we are replacing a vote
        let votes_by_voters =
            self.votes_by_voters
                .insert_or_update_simple(identifier, payload.clone(), |_| Some(payload));
        Ok(Self { votes_by_voters })
    }
}

impl ProposalManagers {
    fn new(plan: &VotePlan) -> Self {
        let proposal_managers = plan
            .proposals()
            .iter()
            .map(|proposal| ProposalManager::new(proposal))
            .collect();

        Self(proposal_managers)
    }

    /// attempt to apply the vote to one of the proposals
    ///
    /// if the proposal is not found this function will return an error.
    /// otherwise it will apply the vote. If the given identifier
    /// already had a vote, the previous vote will be discarded
    /// and only the new one will be kept
    pub fn vote(
        &self,
        identifier: UnspecifiedAccountIdentifier,
        cast: VoteCast,
    ) -> Result<Self, VoteError> {
        let proposal_index = cast.proposal_index() as usize;
        if let Some(manager) = self.0.get(proposal_index) {
            let updated_manager = manager.vote(identifier, cast)?;

            // only clone the array if it does make sens to do so:
            //
            // * the index exist
            // * updated_manager succeed
            let mut updated = self.clone();

            // not unsafe to call this function since we already know this
            // `proposal_index` already exist in the array
            unsafe { *updated.0.get_unchecked_mut(proposal_index) = updated_manager };

            Ok(updated)
        } else {
            Err(VoteError::InvalidVoteProposal {
                num_proposals: self.0.len(),
                vote: cast,
            })
        }
    }
}

impl VotePlanManager {
    pub fn new(plan: VotePlan) -> Self {
        let id = plan.to_id();
        let proposal_managers = ProposalManagers::new(&plan);

        Self {
            id,
            plan: Arc::new(plan),
            proposal_managers,
        }
    }

    pub fn id(&self) -> &VotePlanId {
        &self.id
    }

    pub fn plan(&self) -> &VotePlan {
        &self.plan
    }

    pub fn can_vote(&self, date: BlockDate) -> bool {
        self.plan().can_vote(date)
    }

    pub fn can_committee(&self, date: BlockDate) -> bool {
        self.plan().committee_time(date)
    }

    /// return true if the vote plan has elapsed i.e. the vote is
    /// no longer interesting to track in the ledger and it can be
    /// GCed.
    pub fn vote_plan_elapsed(&self, date: BlockDate) -> bool {
        self.plan().committee_end() < date
    }

    /// attempt to apply the vote to one of the proposals
    ///
    /// If the given identifier already had a vote, the previous vote will
    /// be discarded and only the new one will be kept.
    ///
    /// # errors
    ///
    /// * this function may fail if the proposal identifier is different
    /// * if the proposal index is not one one of the proposal listed
    /// * if the block_date show it is no longer valid to cast a vote for any
    ///   of the managed proposals
    ///
    pub fn vote(
        &self,
        block_date: BlockDate,
        identifier: UnspecifiedAccountIdentifier,
        cast: VoteCast,
    ) -> Result<Self, VoteError> {
        if cast.vote_plan() != self.id() {
            Err(VoteError::InvalidVotePlan {
                expected: self.id().clone(),
                vote: cast,
            })
        } else if !self.can_vote(block_date) {
            Err(VoteError::VoteTimeElapsed {
                start: self.plan().vote_start(),
                end: self.plan().vote_end(),
                vote: cast,
            })
        } else {
            let proposal_managers = self.proposal_managers.vote(identifier, cast)?;

            Ok(Self {
                proposal_managers,
                plan: Arc::clone(&self.plan),
                id: self.id.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockDate;
    use crate::testing::{TestGen, VoteTestGen};
    use chain_core::property::BlockDate as BlockDateProp;
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    #[test]
    pub fn proposal_manager_insert_vote() {
        let vote_plan = VoteTestGen::vote_plan();
        let vote_cast_payload = VoteCastPayload::new(vec![1u8]);
        let vote_cast = VoteCast::new(vote_plan.to_id(), 0, vote_cast_payload.clone());

        let mut proposal_manager = ProposalManager::new(vote_plan.proposals().get(0).unwrap());

        let identifier = TestGen::unspecified_account_identifier();
        proposal_manager = proposal_manager
            .vote(identifier.clone(), vote_cast)
            .unwrap();

        let (_, actual_vote_cast_payload) = proposal_manager
            .votes_by_voters
            .iter()
            .find(|(x, _y)| **x == identifier)
            .unwrap();
        assert_eq!(*actual_vote_cast_payload, vote_cast_payload);
    }

    #[test]
    pub fn proposal_manager_replace_vote() {
        let vote_plan = VoteTestGen::vote_plan();
        let first_vote_cast_payload = VoteTestGen::vote_cast_payload();
        let second_vote_cast_payload = VoteTestGen::vote_cast_payload();

        let first_vote_cast = VoteCast::new(vote_plan.to_id(), 0, first_vote_cast_payload.clone());
        let second_vote_cast =
            VoteCast::new(vote_plan.to_id(), 0, second_vote_cast_payload.clone());

        let mut proposal_manager = ProposalManager::new(vote_plan.proposals().get(0).unwrap());

        let identifier = TestGen::unspecified_account_identifier();
        proposal_manager = proposal_manager
            .vote(identifier.clone(), first_vote_cast)
            .unwrap();
        proposal_manager = proposal_manager
            .vote(identifier.clone(), second_vote_cast)
            .unwrap();

        let (_, actual_vote_cast_payload) = proposal_manager
            .votes_by_voters
            .iter()
            .find(|(x, _y)| **x == identifier)
            .unwrap();
        assert_eq!(*actual_vote_cast_payload, second_vote_cast_payload);
    }

    #[test]
    pub fn proposal_managers_many_votes() {
        let vote_plan = VoteTestGen::vote_plan_with_proposals(2);
        let first_vote_cast_payload = VoteTestGen::vote_cast_payload();
        let second_vote_cast_payload = VoteTestGen::vote_cast_payload();

        let first_vote_cast = VoteCast::new(vote_plan.to_id(), 0, first_vote_cast_payload.clone());
        let second_vote_cast =
            VoteCast::new(vote_plan.to_id(), 1, second_vote_cast_payload.clone());

        let mut proposal_managers = ProposalManagers::new(&vote_plan);

        let identifier = TestGen::unspecified_account_identifier();
        proposal_managers = proposal_managers
            .vote(identifier.clone(), first_vote_cast)
            .unwrap();
        proposal_managers = proposal_managers
            .vote(identifier.clone(), second_vote_cast)
            .unwrap();

        let (_, actual_vote_cast_payload) = proposal_managers
            .0
            .get(0)
            .unwrap()
            .votes_by_voters
            .iter()
            .find(|(x, _y)| **x == identifier)
            .unwrap();
        assert_eq!(*actual_vote_cast_payload, first_vote_cast_payload);

        let (_, actual_vote_cast_payload) = proposal_managers
            .0
            .get(1)
            .unwrap()
            .votes_by_voters
            .iter()
            .find(|(x, _y)| **x == identifier)
            .unwrap();
        assert_eq!(*actual_vote_cast_payload, second_vote_cast_payload);
    }

    #[test]
    pub fn vote_for_nonexisting_proposal() {
        let vote_plan = VoteTestGen::vote_plan_with_proposals(1);
        let vote_cast = VoteCast::new(vote_plan.to_id(), 2, VoteTestGen::vote_cast_payload());

        let proposal_managers = ProposalManagers::new(&vote_plan);
        assert!(proposal_managers
            .vote(TestGen::unspecified_account_identifier(), vote_cast)
            .is_err());
    }

    #[test]
    pub fn proposal_managers_update_vote() {
        let vote_plan = VoteTestGen::vote_plan_with_proposals(2);
        let first_vote_cast_payload = VoteTestGen::vote_cast_payload();
        let second_vote_cast_payload = VoteTestGen::vote_cast_payload();

        let first_vote_cast = VoteCast::new(vote_plan.to_id(), 0, first_vote_cast_payload.clone());
        let second_vote_cast =
            VoteCast::new(vote_plan.to_id(), 0, second_vote_cast_payload.clone());

        let mut proposal_managers = ProposalManagers::new(&vote_plan);

        let identifier = TestGen::unspecified_account_identifier();
        proposal_managers = proposal_managers
            .vote(identifier.clone(), first_vote_cast)
            .unwrap();
        proposal_managers = proposal_managers
            .vote(identifier.clone(), second_vote_cast)
            .unwrap();

        let (_, actual_vote_cast_payload) = proposal_managers
            .0
            .get(0)
            .unwrap()
            .votes_by_voters
            .iter()
            .find(|(x, _y)| **x == identifier)
            .unwrap();
        assert_eq!(*actual_vote_cast_payload, second_vote_cast_payload);
    }

    #[quickcheck]
    pub fn vote_plan_manager_can_vote(vote_plan: VotePlan, date: BlockDate) -> TestResult {
        let vote_plan_manager = VotePlanManager::new(vote_plan.clone());
        TestResult::from_bool(
            should_be_in_vote_time(&vote_plan, date) == vote_plan_manager.can_vote(date),
        )
    }

    #[quickcheck]
    pub fn vote_plan_manager_can_committee(vote_plan: VotePlan, date: BlockDate) -> TestResult {
        let vote_plan_manager = VotePlanManager::new(vote_plan.clone());
        TestResult::from_bool(
            should_be_in_committee_time(&vote_plan, date) == vote_plan_manager.can_committee(date),
        )
    }

    fn should_be_in_vote_time(vote_plan: &VotePlan, date: BlockDate) -> bool {
        let vote_start_date = vote_plan.vote_start();
        let vote_finish_date = vote_plan.vote_end();

        date >= vote_start_date && date < vote_finish_date
    }

    fn should_be_in_committee_time(vote_plan: &VotePlan, date: BlockDate) -> bool {
        let comittee_end_date = vote_plan.committee_end();
        let vote_finish_date = vote_plan.vote_end();

        date >= vote_finish_date && date < comittee_end_date
    }

    #[quickcheck]
    pub fn vote_plan_manager_plan_elapsed(vote_plan: VotePlan, date: BlockDate) -> TestResult {
        let vote_plan_manager = VotePlanManager::new(vote_plan.clone());
        let committee_end_date = vote_plan.committee_end();

        let vote_plan_elapsed = committee_end_date < date;
        TestResult::from_bool(vote_plan_elapsed == vote_plan_manager.vote_plan_elapsed(date))
    }

    #[test]
    pub fn vote_manager_vote_cast_different_id() {
        let vote_plan = VoteTestGen::vote_plan_with_proposals(1);
        let wrong_plan = VoteTestGen::vote_plan_with_proposals(1);
        let vote_plan_manager = VotePlanManager::new(vote_plan.clone());
        let vote_cast = VoteCast::new(wrong_plan.to_id(), 0, VoteTestGen::vote_cast_payload());

        assert_eq!(
            vote_plan_manager
                .vote(
                    BlockDate::first(),
                    TestGen::unspecified_account_identifier(),
                    vote_cast.clone()
                )
                .err()
                .unwrap(),
            VoteError::InvalidVotePlan {
                expected: vote_plan_manager.id().clone(),
                vote: vote_cast.clone(),
            }
        );
    }

    #[test]
    pub fn vote_manager_too_late_to_vote() {
        let vote_plan = VoteTestGen::vote_plan_with_proposals(1);
        let vote_plan_manager = VotePlanManager::new(vote_plan.clone());
        let vote_cast = VoteCast::new(vote_plan.to_id(), 0, VoteTestGen::vote_cast_payload());

        assert_eq!(
            vote_plan_manager
                .vote(
                    vote_plan.vote_end().next_epoch(),
                    TestGen::unspecified_account_identifier(),
                    vote_cast.clone()
                )
                .err()
                .unwrap(),
            VoteError::VoteTimeElapsed {
                start: vote_plan.vote_start(),
                end: vote_plan.vote_end(),
                vote: vote_cast,
            }
        );
    }

    #[test]
    pub fn vote_manager_too_early_to_vote() {
        let vote_plan = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(3),
        );

        let vote_plan_manager = VotePlanManager::new(vote_plan.clone());
        let vote_cast = VoteCast::new(vote_plan.to_id(), 0, VoteTestGen::vote_cast_payload());

        assert_eq!(
            vote_plan_manager
                .vote(
                    BlockDate::first(),
                    TestGen::unspecified_account_identifier(),
                    vote_cast.clone()
                )
                .err()
                .unwrap(),
            VoteError::VoteTimeElapsed {
                start: vote_plan.vote_start(),
                end: vote_plan.vote_end(),
                vote: vote_cast,
            }
        );
    }

    #[test]
    pub fn vote_manager_correct_vote() {
        let vote_plan = VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(3),
        );

        let vote_plan_manager = VotePlanManager::new(vote_plan.clone());
        let vote_cast = VoteCast::new(vote_plan.to_id(), 0, VoteTestGen::vote_cast_payload());

        assert!(vote_plan_manager
            .vote(
                BlockDate::from_epoch_slot_id(1, 1),
                TestGen::unspecified_account_identifier(),
                vote_cast.clone()
            )
            .is_ok());
    }
}
