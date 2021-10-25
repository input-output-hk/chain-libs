//use crate::certificate::{verify_certificate, HasPublicKeys, SignatureRaw};
use crate::date::BlockDate;
use crate::fragment::config::ConfigParams;
use crate::key::{deserialize_signature, serialize_signature, verify_signature, BftLeaderId};
use crate::setting::{ActiveSlotsCoeffError, Settings};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property::{self, Serialize};
use chain_crypto::{Ed25519, Signature, Verification};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateState {
    pub proposals: HashMap<UpdateProposalId, UpdateProposalState>,
}

impl UpdateState {
    pub fn new() -> Self {
        UpdateState {
            proposals: HashMap::new(),
        }
    }

    pub fn apply_proposal(
        mut self,
        proposal_id: UpdateProposalId,
        proposal: &SignedUpdateProposal,
        settings: &Settings,
        cur_date: BlockDate,
    ) -> Result<Self, Error> {
        let proposer_id = &proposal.proposal.proposer_id;

        if proposal.verify() == Verification::Failed {
            return Err(Error::BadProposalSignature(
                proposal_id,
                proposer_id.clone(),
            ));
        }

        if !settings.bft_leaders.contains(proposer_id) {
            return Err(Error::BadProposer(proposal_id, proposer_id.clone()));
        }

        let proposal = &proposal.proposal.proposal;

        if self
            .proposals
            .insert(
                proposal_id,
                UpdateProposalState {
                    proposal: proposal.clone(),
                    proposal_date: cur_date,
                    votes: HashSet::new(),
                },
            )
            .is_some()
        {
            Err(Error::DuplicateProposal(proposal_id))
        } else {
            Ok(self)
        }
    }

    pub fn apply_vote(
        mut self,
        vote: &SignedUpdateVote,
        settings: &Settings,
    ) -> Result<Self, Error> {
        if vote.verify() == Verification::Failed {
            return Err(Error::BadVoteSignature(
                vote.vote.proposal_id,
                vote.vote.voter_id.clone(),
            ));
        }

        let vote = &vote.vote;

        if !settings.bft_leaders.contains(&vote.voter_id) {
            return Err(Error::BadVoter(vote.proposal_id, vote.voter_id.clone()));
        }

        if let Some(proposal) = self.proposals.get_mut(&vote.proposal_id) {
            if !proposal.votes.insert(vote.voter_id.clone()) {
                return Err(Error::DuplicateVote(
                    vote.proposal_id,
                    vote.voter_id.clone(),
                ));
            }

            Ok(self)
        } else {
            Err(Error::VoteForMissingProposal(vote.proposal_id))
        }
    }

    pub fn process_proposals(
        mut self,
        mut settings: Settings,
        prev_date: BlockDate,
        new_date: BlockDate,
    ) -> Result<(Self, Settings), Error> {
        let mut expired_ids = vec![];

        assert!(prev_date < new_date);

        let mut proposals: Vec<(&UpdateProposalId, &UpdateProposalState)> =
            self.proposals.iter().collect();

        // sort proposals by the date
        proposals.sort_by(|(a_id, a_state), (b_id, b_state)| {
            match a_state.proposal_date.cmp(&b_state.proposal_date) {
                std::cmp::Ordering::Equal => a_id.cmp(b_id),
                res => res,
            }
        });

        // If we entered a new epoch, then delete expired update
        // proposals and apply accepted update proposals.
        if prev_date.epoch < new_date.epoch {
            for (proposal_id, proposal_state) in proposals {
                // If a majority of BFT leaders voted for the
                // proposal, then apply it.
                if proposal_state.votes.len() > settings.bft_leaders.len() / 2 {
                    settings = settings.apply(&proposal_state.proposal.changes)?;
                    expired_ids.push(*proposal_id);
                } else if proposal_state.proposal_date.epoch + settings.proposal_expiration
                    < new_date.epoch
                {
                    expired_ids.push(*proposal_id);
                }
            }

            for proposal_id in expired_ids {
                self.proposals.remove(&proposal_id);
            }
        }

        Ok((self, settings))
    }
}

impl Default for UpdateState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateProposalState {
    pub proposal: UpdateProposal,
    pub proposal_date: BlockDate,
    pub votes: HashSet<UpdateVoterId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /*
    InvalidCurrentBlockId(Hash, Hash),
    UpdateIsInvalid,
     */
    BadProposalSignature(UpdateProposalId, UpdateVoterId),
    BadProposer(UpdateProposalId, UpdateVoterId),
    DuplicateProposal(UpdateProposalId),
    VoteForMissingProposal(UpdateProposalId),
    BadVoteSignature(UpdateProposalId, UpdateVoterId),
    BadVoter(UpdateProposalId, UpdateVoterId),
    DuplicateVote(UpdateProposalId, UpdateVoterId),
    ReadOnlySetting,
    BadBftSlotsRatio(crate::milli::Milli),
    BadConsensusGenesisPraosActiveSlotsCoeff(ActiveSlotsCoeffError),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            /*
            Error::InvalidCurrentBlockId(current_one, update_one) => {
                write!(f, "Cannot apply Setting Update. Update needs to be applied to from block {:?} but received {:?}", update_one, current_one)
            }
            Error::UpdateIsInvalid => write!(
                f,
                "Update does not apply to current state"
            ),
             */
            Error::BadProposalSignature(proposal_id, proposer_id) => write!(
                f,
                "Proposal {} from {:?} has an incorrect signature",
                proposal_id, proposer_id
            ),
            Error::BadProposer(proposal_id, proposer_id) => write!(
                f,
                "Proposer {:?} for proposal {} is not a BFT leader",
                proposer_id, proposal_id
            ),
            Error::DuplicateProposal(proposal_id) => {
                write!(f, "Received a duplicate proposal {}", proposal_id)
            }
            Error::VoteForMissingProposal(proposal_id) => write!(
                f,
                "Received a vote for a non-existent proposal {}",
                proposal_id
            ),
            Error::BadVoteSignature(proposal_id, voter_id) => write!(
                f,
                "Vote from {:?} for proposal {} has an incorrect signature",
                voter_id, proposal_id
            ),
            Error::BadVoter(proposal_id, voter_id) => write!(
                f,
                "Voter {:?} for proposal {} is not a BFT leader",
                voter_id, proposal_id
            ),
            Error::DuplicateVote(proposal_id, voter_id) => write!(
                f,
                "Received a duplicate vote from {:?} for proposal {}",
                voter_id, proposal_id
            ),
            Error::ReadOnlySetting => write!(
                f,
                "Received a proposal to modify a chain parameter that can only be set in block 0"
            ),
            Error::BadBftSlotsRatio(m) => {
                write!(f, "Cannot set BFT slots ratio to invalid value {}", m)
            }
            Error::BadConsensusGenesisPraosActiveSlotsCoeff(err) => write!(
                f,
                "Cannot set consensus genesis praos active slots coefficient: {}",
                err
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<ActiveSlotsCoeffError> for Error {
    fn from(err: ActiveSlotsCoeffError) -> Self {
        Error::BadConsensusGenesisPraosActiveSlotsCoeff(err)
    }
}

pub type UpdateProposalId = crate::fragment::FragmentId;
pub type UpdateVoterId = BftLeaderId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateProposal {
    changes: ConfigParams,
}

impl Default for UpdateProposal {
    fn default() -> Self {
        Self {
            changes: ConfigParams::new(),
        }
    }
}

impl UpdateProposal {
    pub fn new(changes: ConfigParams) -> Self {
        Self { changes }
    }

    pub fn id(&self) -> UpdateProposalId {
        UpdateProposalId::hash_bytes(
            &self
                .serialize_as_vec()
                .expect("memory serialize is expected to just work"),
        )
    }

    pub fn changes(&self) -> &ConfigParams {
        &self.changes
    }
}

impl property::Serialize for UpdateProposal {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.changes.serialize(writer)?;
        Ok(())
    }
}

impl property::Deserialize for UpdateProposal {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let changes = ConfigParams::deserialize(reader)?;
        Ok(UpdateProposal { changes })
    }
}

impl Readable for UpdateProposal {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        Ok(Self {
            changes: ConfigParams::read(buf)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct UpdateProposalWithProposer {
    proposal: UpdateProposal,
    proposer_id: UpdateVoterId,
}

impl UpdateProposalWithProposer {
    pub fn new(proposal: UpdateProposal, proposer_id: UpdateVoterId) -> Self {
        Self {
            proposal,
            proposer_id,
        }
    }

    pub fn proposal(&self) -> &UpdateProposal {
        &self.proposal
    }
}

impl property::Serialize for UpdateProposalWithProposer {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.proposal.serialize(&mut codec)?;
        self.proposer_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for UpdateProposalWithProposer {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        Ok(Self {
            proposal: Readable::read(buf)?,
            proposer_id: Readable::read(buf)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SignedUpdateProposal {
    sign: Signature<UpdateProposal, Ed25519>,
    proposal: UpdateProposalWithProposer,
}

impl SignedUpdateProposal {
    pub fn new(
        sign: Signature<UpdateProposal, Ed25519>,
        proposal: UpdateProposalWithProposer,
    ) -> Self {
        Self { sign, proposal }
    }

    pub fn verify(&self) -> Verification {
        verify_signature(
            &self.sign,
            self.proposal.proposer_id.as_public_key(),
            &self.proposal.proposal,
        )
    }

    pub fn proposal(&self) -> &UpdateProposalWithProposer {
        &self.proposal
    }
}

impl property::Serialize for SignedUpdateProposal {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        serialize_signature(&self.sign, &mut codec)?;
        self.proposal.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for SignedUpdateProposal {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        Ok(Self {
            sign: deserialize_signature(buf)?,
            proposal: Readable::read(buf)?,
        })
    }
}

// A positive vote for a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateVote {
    proposal_id: UpdateProposalId,
    voter_id: UpdateVoterId,
}

impl UpdateVote {
    pub fn new(proposal_id: UpdateProposalId, voter_id: UpdateVoterId) -> Self {
        Self {
            proposal_id,
            voter_id,
        }
    }
}

impl property::Serialize for UpdateVote {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.proposal_id.serialize(&mut codec)?;
        self.voter_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for UpdateVote {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let proposal_id = Readable::read(buf)?;
        let voter_id = Readable::read(buf)?;
        Ok(UpdateVote {
            proposal_id,
            voter_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SignedUpdateVote {
    sign: Signature<UpdateVote, Ed25519>,
    vote: UpdateVote,
}

impl SignedUpdateVote {
    pub fn new(sign: Signature<UpdateVote, Ed25519>, vote: UpdateVote) -> Self {
        Self { sign, vote }
    }

    pub fn verify(&self) -> Verification {
        verify_signature(&self.sign, self.vote.voter_id.as_public_key(), &self.vote)
    }
}

impl property::Serialize for SignedUpdateVote {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        serialize_signature(&self.sign, &mut codec)?;
        self.vote.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for SignedUpdateVote {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        Ok(SignedUpdateVote {
            sign: deserialize_signature(buf)?,
            vote: Readable::read(buf)?,
        })
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod tests {
    use super::*;
    use crate::key::make_signature;
    #[cfg(test)]
    use crate::testing::serialization::serialization_bijection;
    #[cfg(test)]
    use crate::{
        config::ConfigParam,
        testing::{
            builders::update_builder::{ProposalBuilder, SignedProposalBuilder, UpdateVoteBuilder},
            data::LeaderPair,
            TestGen,
        },
    };
    use crate::{
        fragment::config::ConfigParams,
        update::{
            SignedUpdateProposal, SignedUpdateVote, UpdateProposal, UpdateProposalWithProposer,
            UpdateVote,
        },
    };
    #[cfg(test)]
    use chain_addr::Discrimination;
    use chain_crypto::SecretKey;
    #[cfg(test)]
    use quickcheck::TestResult;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use std::iter;

    impl Arbitrary for UpdateProposal {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut changes = ConfigParams::new();
            for _ in 0..u8::arbitrary(g) % 10 {
                changes.push(Arbitrary::arbitrary(g));
            }
            Self { changes }
        }
    }

    impl Arbitrary for UpdateProposalWithProposer {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                proposal: Arbitrary::arbitrary(g),
                proposer_id: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedUpdateProposal {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let sk: SecretKey<Ed25519> = Arbitrary::arbitrary(g);
            let proposal: UpdateProposalWithProposer = Arbitrary::arbitrary(g);
            let sign = make_signature(&sk, &proposal.proposal);
            Self { sign, proposal }
        }
    }

    impl Arbitrary for UpdateVote {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                proposal_id: Arbitrary::arbitrary(g),
                voter_id: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedUpdateVote {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let sk: SecretKey<Ed25519> = Arbitrary::arbitrary(g);
            let vote: UpdateVote = Arbitrary::arbitrary(g);
            let sign = make_signature(&sk, &vote);
            Self { sign, vote }
        }
    }

    impl Arbitrary for UpdateProposalState {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let size = usize::arbitrary(g);
            Self {
                proposal: UpdateProposal::arbitrary(g),
                proposal_date: BlockDate::arbitrary(g),
                votes: iter::from_fn(|| Some(UpdateVoterId::arbitrary(g)))
                    .take(size)
                    .collect(),
            }
        }
    }

    #[cfg(test)]
    fn apply_update_proposal(
        update_state: UpdateState,
        proposal_id: UpdateProposalId,
        config_param: &ConfigParam,
        proposer: &LeaderPair,
        settings: &Settings,
        block_date: BlockDate,
    ) -> Result<UpdateState, Error> {
        let update_proposal = ProposalBuilder::new()
            .with_proposal_change(config_param.clone())
            .build();

        let signed_update_proposal = SignedProposalBuilder::new()
            .with_proposal_update(update_proposal)
            .with_proposer_secret_key(proposer.key())
            .build();

        update_state.apply_proposal(proposal_id, &signed_update_proposal, &settings, block_date)
    }

    #[cfg(test)]
    fn apply_update_vote(
        update_state: UpdateState,
        proposal_id: UpdateProposalId,
        proposer: &LeaderPair,
        settings: &Settings,
    ) -> Result<UpdateState, Error> {
        let signed_update_vote = UpdateVoteBuilder::new()
            .with_proposal_id(proposal_id)
            .with_voter_secret_key(proposer.key())
            .build();

        update_state.apply_vote(&signed_update_vote, &settings)
    }

    quickcheck! {
        fn update_proposal_serialize_deserialize_bijection(update_proposal: UpdateProposal) -> TestResult {
            serialization_bijection(update_proposal)
        }
    }

    #[test]
    fn apply_proposal_with_unknown_proposer_should_return_error() {
        // data
        let unknown_leader = TestGen::leader_pair();
        let block_date = BlockDate::first();
        let proposal_id = TestGen::hash();
        let config_param = ConfigParam::SlotsPerEpoch(100);
        //setup
        let update_state = UpdateState::new();
        let settings = Settings::new();

        assert_eq!(
            apply_update_proposal(
                update_state,
                proposal_id,
                &config_param,
                &unknown_leader,
                &settings,
                block_date,
            ),
            Err(Error::BadProposer(proposal_id, unknown_leader.id()))
        );
    }

    #[test]
    fn apply_invalid_signed_proposal_should_return_error() {
        // data
        let proposal_id = TestGen::hash();
        let block_date = BlockDate::first();
        let config_param = ConfigParam::SlotsPerEpoch(100);
        //setup
        let update_state = UpdateState::new();

        let leaders = TestGen::leaders_pairs()
            .take(5)
            .collect::<Vec<LeaderPair>>();
        let proposer1 = &leaders[0];
        let proposer2 = &leaders[1];
        let settings = TestGen::settings(leaders.clone());

        let update_proposal = ProposalBuilder::new()
            .with_proposal_change(config_param.clone())
            .build();

        let mut signed_update_proposal = SignedProposalBuilder::new()
            .with_proposal_update(update_proposal)
            .with_proposer_secret_key(proposer1.key())
            .build();

        // corrupt signed proposal
        signed_update_proposal.proposal.proposer_id = proposer2.id();

        assert_eq!(
            update_state.apply_proposal(
                proposal_id,
                &signed_update_proposal,
                &settings,
                block_date,
            ),
            Err(Error::BadProposalSignature(proposal_id, proposer2.id()))
        );
    }

    #[test]
    fn apply_duplicated_proposal_should_return_error() {
        // data
        let proposal_id = TestGen::hash();
        let block_date = BlockDate::first();
        let config_param = ConfigParam::SlotsPerEpoch(100);
        //setup
        let mut update_state = UpdateState::new();

        let leaders = TestGen::leaders_pairs()
            .take(5)
            .collect::<Vec<LeaderPair>>();
        let proposer = &leaders[0];
        let settings = TestGen::settings(leaders.clone());

        update_state = apply_update_proposal(
            update_state,
            proposal_id,
            &config_param,
            proposer,
            &settings,
            block_date,
        )
        .expect("failed while applying first proposal");

        assert_eq!(
            apply_update_proposal(
                update_state,
                proposal_id,
                &config_param,
                proposer,
                &settings,
                block_date
            ),
            Err(Error::DuplicateProposal(proposal_id))
        );
    }

    #[test]
    fn test_add_invalid_signed_vote_should_return_error() {
        let mut update_state = UpdateState::new();
        let proposal_id = TestGen::hash();
        let block_date = BlockDate::first();
        let config_param = ConfigParam::SlotsPerEpoch(100);

        let leaders = TestGen::leaders_pairs()
            .take(5)
            .collect::<Vec<LeaderPair>>();
        let proposer1 = &leaders[0];
        let proposer2 = &leaders[1];
        let settings = TestGen::settings(leaders.clone());

        update_state = apply_update_proposal(
            update_state,
            proposal_id,
            &config_param,
            proposer1,
            &settings,
            block_date,
        )
        .expect("failed while applying proposal");

        let mut signed_update_vote = UpdateVoteBuilder::new()
            .with_proposal_id(proposal_id)
            .with_voter_secret_key(proposer1.key())
            .build();

        // corrupt signed vote
        signed_update_vote.vote.voter_id = proposer2.id();

        assert_eq!(
            update_state.apply_vote(&signed_update_vote, &settings),
            Err(Error::BadVoteSignature(proposal_id, proposer2.id()))
        );
    }

    #[test]
    fn test_add_vote_for_non_existing_proposal_should_return_error() {
        let mut update_state = UpdateState::new();
        let proposal_id = TestGen::hash();
        let unknown_proposal_id = TestGen::hash();
        let block_date = BlockDate::first();
        let config_param = ConfigParam::SlotsPerEpoch(100);
        let leaders = TestGen::leaders_pairs()
            .take(5)
            .collect::<Vec<LeaderPair>>();
        let proposer = &leaders[0];
        let settings = TestGen::settings(leaders.clone());

        // Apply proposal
        update_state = apply_update_proposal(
            update_state,
            proposal_id,
            &config_param,
            proposer,
            &settings,
            block_date,
        )
        .expect("failed while applying first proposal");

        // Apply vote for unknown proposal
        assert_eq!(
            apply_update_vote(update_state, unknown_proposal_id, proposer, &settings),
            Err(Error::VoteForMissingProposal(unknown_proposal_id))
        );
    }

    #[test]
    fn test_add_duplicated_vote_should_return_error() {
        let mut update_state = UpdateState::new();
        let proposal_id = TestGen::hash();
        let block_date = BlockDate::first();
        let config_param = ConfigParam::SlotsPerEpoch(100);

        let leaders = TestGen::leaders_pairs()
            .take(5)
            .collect::<Vec<LeaderPair>>();
        let proposer = &leaders[0];
        let settings = TestGen::settings(leaders.clone());

        update_state = apply_update_proposal(
            update_state,
            proposal_id,
            &config_param,
            proposer,
            &settings,
            block_date,
        )
        .expect("failed while applying proposal");

        // Apply vote
        update_state = apply_update_vote(update_state, proposal_id, proposer, &settings)
            .expect("failed while applying first vote");

        // Apply duplicated vote
        assert_eq!(
            apply_update_vote(update_state, proposal_id, proposer, &settings),
            Err(Error::DuplicateVote(proposal_id, proposer.id()))
        );
    }

    #[test]
    fn test_add_vote_from_unknown_voter_should_return_error() {
        let mut update_state = UpdateState::new();
        let proposal_id = TestGen::hash();
        let unknown_leader = TestGen::leader_pair();
        let block_date = BlockDate::first();
        let config_param = ConfigParam::SlotsPerEpoch(100);

        let leaders = TestGen::leaders_pairs()
            .take(5)
            .collect::<Vec<LeaderPair>>();
        let proposer = &leaders[0];
        let settings = TestGen::settings(leaders.clone());

        update_state = apply_update_proposal(
            update_state,
            proposal_id,
            &config_param,
            proposer,
            &settings,
            block_date,
        )
        .expect("failed while applying proposal");

        // Apply vote for unknown leader
        assert_eq!(
            apply_update_vote(update_state, proposal_id, &unknown_leader, &settings),
            Err(Error::BadVoter(proposal_id, unknown_leader.id()))
        );
    }

    #[test]
    fn process_proposals_for_readonly_setting_should_return_error() {
        let mut update_state = UpdateState::new();
        let proposal_id = TestGen::hash();
        let proposer = TestGen::leader_pair();
        let block_date = BlockDate::first();
        let readonly_setting = ConfigParam::Discrimination(Discrimination::Test);

        let settings = TestGen::settings(vec![proposer.clone()]);

        update_state = apply_update_proposal(
            update_state,
            proposal_id,
            &readonly_setting,
            &proposer,
            &settings,
            block_date,
        )
        .expect("failed while applying proposal");

        // Apply vote
        update_state = apply_update_vote(update_state, proposal_id, &proposer, &settings)
            .expect("failed while applying vote");

        assert_eq!(
            update_state.process_proposals(settings, block_date, block_date.next_epoch()),
            Err(Error::ReadOnlySetting)
        );
    }

    #[test]
    pub fn process_proposal_is_by_id_ordered() {
        let mut update_state = UpdateState::new();
        let first_proposal_id = TestGen::hash();
        let second_proposal_id = TestGen::hash();
        let first_proposer = TestGen::leader_pair();
        let second_proposer = TestGen::leader_pair();
        let block_date = BlockDate::first();
        let first_update = ConfigParam::SlotsPerEpoch(100);
        let second_update = ConfigParam::SlotsPerEpoch(200);

        let settings = TestGen::settings(vec![first_proposer.clone(), second_proposer.clone()]);

        // Apply proposal
        update_state = apply_update_proposal(
            update_state,
            first_proposal_id,
            &first_update,
            &first_proposer,
            &settings,
            block_date,
        )
        .expect("failed while applying proposal");

        // Apply vote
        update_state =
            apply_update_vote(update_state, first_proposal_id, &first_proposer, &settings)
                .expect("failed while applying vote");

        // Apply vote
        update_state =
            apply_update_vote(update_state, first_proposal_id, &second_proposer, &settings)
                .expect("failed while applying vote");

        // Apply proposal
        update_state = apply_update_proposal(
            update_state,
            second_proposal_id,
            &second_update,
            &second_proposer,
            &settings,
            block_date,
        )
        .expect("failed while applying proposal");

        // Apply vote
        update_state =
            apply_update_vote(update_state, second_proposal_id, &first_proposer, &settings)
                .expect("failed while applying vote");

        // Apply vote
        update_state = apply_update_vote(
            update_state,
            second_proposal_id,
            &second_proposer,
            &settings,
        )
        .expect("failed while applying vote");

        let (update_state, settings) = update_state
            .process_proposals(settings, block_date, block_date.next_epoch())
            .expect("error while processing proposal");

        match first_proposal_id.cmp(&second_proposal_id) {
            std::cmp::Ordering::Less => assert_eq!(settings.slots_per_epoch, 200),
            std::cmp::Ordering::Greater => assert_eq!(settings.slots_per_epoch, 100),
            _ => {}
        }

        assert_eq!(update_state.proposals.len(), 0);
    }

    #[test]
    pub fn process_proposal_is_by_time_ordered() {
        let mut update_state = UpdateState::new();
        let first_proposal_id = TestGen::hash();
        let second_proposal_id = TestGen::hash();
        let first_proposer = TestGen::leader_pair();
        let second_proposer = TestGen::leader_pair();
        let first_proposal_block_date = BlockDate::first();
        let second_proposal_block_date = BlockDate {
            slot_id: first_proposal_block_date.slot_id + 1,
            ..first_proposal_block_date
        };
        let first_update = ConfigParam::SlotsPerEpoch(100);
        let second_update = ConfigParam::SlotsPerEpoch(200);

        let settings = TestGen::settings(vec![first_proposer.clone(), second_proposer.clone()]);

        // Apply proposal
        update_state = apply_update_proposal(
            update_state,
            first_proposal_id,
            &first_update,
            &first_proposer,
            &settings,
            first_proposal_block_date,
        )
        .expect("failed while applying proposal");

        // Apply vote
        update_state =
            apply_update_vote(update_state, first_proposal_id, &first_proposer, &settings)
                .expect("failed while applying vote");

        // Apply vote
        update_state =
            apply_update_vote(update_state, first_proposal_id, &second_proposer, &settings)
                .expect("failed while applying vote");

        // Apply proposal
        update_state = apply_update_proposal(
            update_state,
            second_proposal_id,
            &second_update,
            &second_proposer,
            &settings,
            second_proposal_block_date,
        )
        .expect("failed while applying proposal");

        // Apply vote
        update_state =
            apply_update_vote(update_state, second_proposal_id, &first_proposer, &settings)
                .expect("failed while applying vote");

        // Apply vote
        update_state = apply_update_vote(
            update_state,
            second_proposal_id,
            &second_proposer,
            &settings,
        )
        .expect("failed while applying vote");

        let (update_state, settings) = update_state
            .process_proposals(
                settings,
                first_proposal_block_date,
                first_proposal_block_date.next_epoch(),
            )
            .expect("error while processing proposal");

        assert_eq!(settings.slots_per_epoch, 200);

        assert_eq!(update_state.proposals.len(), 0);
    }

    #[derive(Debug, Copy, Clone)]
    struct ExpiryBlockDate {
        pub block_date: BlockDate,
        pub proposal_expiration: u32,
    }

    #[cfg(test)]
    impl ExpiryBlockDate {
        pub fn block_date(&self) -> BlockDate {
            self.block_date
        }

        pub fn proposal_expiration(&self) -> u32 {
            self.proposal_expiration
        }

        pub fn get_last_epoch(&self) -> u32 {
            self.block_date().epoch + self.proposal_expiration() + 1
        }
    }

    impl Arbitrary for ExpiryBlockDate {
        fn arbitrary<G: Gen>(gen: &mut G) -> Self {
            let mut block_date = BlockDate::arbitrary(gen);
            block_date.epoch %= 10;
            let proposal_expiration = u32::arbitrary(gen) % 10;
            ExpiryBlockDate {
                block_date,
                proposal_expiration,
            }
        }
    }

    #[quickcheck]
    fn rejected_proposals_are_removed_after_expiration_period(
        expiry_block_data: ExpiryBlockDate,
    ) -> TestResult {
        let proposal_date = expiry_block_data.block_date();
        let proposal_expiration = expiry_block_data.proposal_expiration();

        let mut update_state = UpdateState::new();
        let proposal_id = TestGen::hash();
        let proposer = TestGen::leader_pair();
        let update = ConfigParam::SlotsPerEpoch(100);

        let mut settings = TestGen::settings(vec![proposer.clone()]);
        settings.proposal_expiration = proposal_expiration;

        // Apply proposal
        update_state = apply_update_proposal(
            update_state,
            proposal_id,
            &update,
            &proposer,
            &settings,
            proposal_date,
        )
        .expect("failed while applying proposal");

        let mut current_block_date = BlockDate::first();

        // Traverse through epoch and check if proposal is still in queue
        // if proposal expiration period is not exceeded after that
        // proposal should be removed from proposal collection
        for _i in 0..expiry_block_data.get_last_epoch() {
            let (update_state, _settings) = update_state
                .clone()
                .process_proposals(
                    settings.clone(),
                    current_block_date,
                    current_block_date.next_epoch(),
                )
                .expect("error while processing proposal");

            if proposal_date.epoch + proposal_expiration <= current_block_date.epoch {
                assert_eq!(update_state.proposals.len(), 0);
            } else {
                assert_eq!(update_state.proposals.len(), 1);
            }
            current_block_date = current_block_date.next_epoch()
        }

        TestResult::passed()
    }
}
