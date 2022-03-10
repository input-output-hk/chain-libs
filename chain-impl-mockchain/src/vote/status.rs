use crate::{
    account,
    certificate::{ExternalProposalId, VotePlanId},
    date::BlockDate,
    tokens::identifier::TokenIdentifier,
    vote::{Options, PayloadType, Tally},
};
use chain_vote::MemberPublicKey;
use imhamt::Trie;

pub struct VotePlanStatus {
    pub id: VotePlanId,
    pub payload: PayloadType,
    pub vote_start: BlockDate,
    pub vote_end: BlockDate,
    pub committee_end: BlockDate,
    pub committee_public_keys: Vec<MemberPublicKey>,
    pub proposals: Vec<VoteProposalStatus>,
    pub voting_token: TokenIdentifier,
}

pub struct VoteProposalStatus {
    pub index: u8,
    pub proposal_id: ExternalProposalId,
    pub options: Options,
    pub tally: Tally,
    pub votes: Trie<account::Identifier, ()>,
}
