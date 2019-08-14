use crate::{
    config::ConfigParam,
    fragment::config::ConfigParams,
    leadership::bft::LeaderId,
    update::{
        SignedUpdateProposal, SignedUpdateVote, UpdateProposal, UpdateProposalId,
        UpdateProposalWithProposer, UpdateVote,
    },
};
use chain_crypto::{Ed25519Extended, SecretKey};

pub struct ProposalBuilder {
    proposer_id: Option<LeaderId>,
    config_params: ConfigParams,
    signature_key: Option<SecretKey<Ed25519Extended>>,
}

impl ProposalBuilder {
    pub fn new() -> Self {
        ProposalBuilder {
            proposer_id: None,
            config_params: ConfigParams::new(),
            signature_key: None,
        }
    }

    pub fn with_proposer_id(&mut self, proposer_id: LeaderId) -> &mut Self {
        self.proposer_id = Some(proposer_id);
        self
    }

    pub fn with_proposal_changes(&mut self, changes: Vec<ConfigParam>) -> &mut Self {
        for change in changes {
            self.with_proposal_change(change);
        }
        self
    }

    pub fn with_proposal_change(&mut self, change: ConfigParam) -> &mut Self {
        self.config_params.push(change);
        self
    }

    pub fn with_signature_key(&mut self, signature_key: SecretKey<Ed25519Extended>) -> &mut Self {
        self.signature_key = Some(signature_key);
        self
    }

    pub fn build(&self) -> SignedUpdateProposal {
        let mut update_proposal = UpdateProposal::new();
        for config_param in self.config_params.iter().cloned() {
            update_proposal.changes.push(config_param);
        }

        //add proposer
        let proposal_signature =
            update_proposal.make_certificate(&self.signature_key.clone().unwrap());
        let update_proposal_with_proposer = UpdateProposalWithProposer {
            proposal: update_proposal,
            proposer_id: self.proposer_id.clone().unwrap(),
        };

        //sign proposal
        SignedUpdateProposal {
            proposal: update_proposal_with_proposer,
            signature: proposal_signature,
        }
    }
}

pub struct UpdateVoteBuilder {
    proposal_id: Option<UpdateProposalId>,
    voter_id: Option<LeaderId>,
    signature_key: Option<SecretKey<Ed25519Extended>>,
}

impl UpdateVoteBuilder {
    pub fn new() -> Self {
        UpdateVoteBuilder {
            proposal_id: None,
            voter_id: None,
            signature_key: None,
        }
    }

    pub fn with_proposal_id(&mut self, proposal_id: UpdateProposalId) -> &mut Self {
        self.proposal_id = Some(proposal_id);
        self
    }

    pub fn with_voter_id(&mut self, voter_id: LeaderId) -> &mut Self {
        self.voter_id = Some(voter_id);
        self
    }

    pub fn with_signature_key(&mut self, signature_key: SecretKey<Ed25519Extended>) -> &mut Self {
        self.signature_key = Some(signature_key);
        self
    }

    pub fn build(&self) -> SignedUpdateVote {
        let update_vote = UpdateVote {
            proposal_id: self.proposal_id.unwrap().clone(),
            voter_id: self.voter_id.clone().unwrap(),
        };
        let vote_signature = update_vote.make_certificate(&self.signature_key.clone().unwrap());
        SignedUpdateVote {
            vote: update_vote,
            signature: vote_signature,
        }
    }
}
