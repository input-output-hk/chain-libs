use super::TestGen;
use crate::{
    block::BlockDate,
    certificate::{
        ExternalProposalId, Proposal, Proposals, PushProposal, VoteAction, VoteCast, VotePlan,
        VoteTally,
    },
    ledger::governance::{ParametersGovernance, TreasuryGovernance},
    vote::{self, Choice, EncryptedVote, Payload, ProofOfCorrectVote},
};
use chain_core::property::BlockDate as BlockDateProp;
use chain_crypto::digest::DigestOf;
use chain_vote::CRS;
use rand_core::{CryptoRng, RngCore};
use typed_bytes::ByteBuilder;

pub struct VoteTestGen;

impl VoteTestGen {
    pub fn proposal_with_action(vote_action: VoteAction) -> Proposal {
        Proposal::new(
            VoteTestGen::external_proposal_id(),
            vote::Options::new_length(4).unwrap(),
            vote_action,
        )
    }

    pub fn proposal() -> Proposal {
        Self::proposal_with_action(VoteAction::OffChain)
    }

    pub fn proposals(count: usize) -> Proposals {
        let mut proposals = Proposals::new();
        for _ in 0..count {
            assert_eq!(
                PushProposal::Success,
                proposals.push(VoteTestGen::proposal()),
                "generate_proposal method is only for correct data preparation"
            );
        }
        proposals
    }

    pub fn proposals_with_action(vote_action: VoteAction, count: usize) -> Proposals {
        let mut proposals = Proposals::new();
        for _ in 0..count {
            assert_eq!(
                PushProposal::Success,
                proposals.push(VoteTestGen::proposal_with_action(vote_action.clone())),
                "generate_proposal method is only for correct data preparation"
            );
        }
        proposals
    }

    pub fn external_proposal_id() -> ExternalProposalId {
        DigestOf::digest_byteslice(
            &ByteBuilder::new()
                .bytes(&TestGen::bytes())
                .finalize()
                .as_byteslice(),
        )
    }

    pub fn vote_cast() -> VoteCast {
        let vote_plan = Self::vote_plan();
        VoteCast::new(vote_plan.to_id(), 0, vote::Payload::public(Choice::new(1)))
    }

    pub fn vote_tally() -> VoteTally {
        let vote_plan = Self::vote_plan();
        VoteTally::new_public(vote_plan.to_id())
    }

    pub fn vote_plan() -> VotePlan {
        VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(3),
            vote::PayloadType::Public,
            Vec::new(),
        )
    }

    pub fn vote_plan_with_proposals(count: usize) -> VotePlan {
        VotePlan::new(
            BlockDate::from_epoch_slot_id(1, 0),
            BlockDate::from_epoch_slot_id(2, 0),
            BlockDate::from_epoch_slot_id(3, 0),
            VoteTestGen::proposals(count),
            vote::PayloadType::Public,
            Vec::new(),
        )
    }

    pub fn vote_cast_payload_for(choice: &Choice) -> vote::Payload {
        vote::Payload::public(*choice)
    }

    pub fn private_vote_cast_payload_for<R: RngCore + CryptoRng>(
        vote_plan: &VotePlan,
        proposal: &Proposal,
        choice: Choice,
        rng: &mut R,
    ) -> Payload {
        let encrypting_key =
            chain_vote::EncryptingVoteKey::from_participants(vote_plan.committee_public_keys());

        let crs = CRS::from_hash(&vote_plan.to_id().as_ref());
        let (encrypted_vote, proof) = chain_vote::encrypt_vote(
            rng,
            &crs,
            &encrypting_key,
            chain_vote::Vote::new(
                proposal.options().choice_range().clone().max().unwrap() as usize + 1,
                choice.as_byte() as usize,
            ),
        );

        Payload::Private {
            encrypted_vote: EncryptedVote::from_inner(encrypted_vote),
            proof: ProofOfCorrectVote::from_inner(proof),
        }
    }

    pub fn vote_cast_payload() -> vote::Payload {
        vote::Payload::public(vote::Choice::new(1))
    }

    pub fn vote_cast_for(vote_plan: &VotePlan) -> VoteCast {
        VoteCast::new(vote_plan.to_id(), 0, VoteTestGen::vote_cast_payload())
    }

    pub fn treasury_governance() -> TreasuryGovernance {
        let mut governance = TreasuryGovernance::new();
        governance.set_default_acceptance_criteria(Default::default());
        governance
    }

    pub fn parameters_governance() -> ParametersGovernance {
        let mut governance = ParametersGovernance::new();
        governance.set_default_acceptance_criteria(Default::default());
        governance
    }
}
