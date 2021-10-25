use crate::key::make_signature;
use crate::{
    config::ConfigParam,
    fee::LinearFee,
    fragment::config::ConfigParams,
    key::BftLeaderId,
    testing::builders::SignedProposalBuilder,
    testing::{arbitrary::utils as arbitrary_utils, builders::update_builder::ProposalBuilder},
    update::{SignedUpdateProposal, SignedUpdateVote, UpdateVote},
};
use chain_crypto::{Ed25519, SecretKey};
use quickcheck::{Arbitrary, Gen};
use std::fmt::{self, Debug};
use std::{collections::HashMap, iter};

#[derive(Clone)]
pub struct UpdateProposalData {
    pub leaders: HashMap<BftLeaderId, SecretKey<Ed25519>>,
    pub proposal: SignedUpdateProposal,
    pub votes: Vec<SignedUpdateVote>,
    pub block_signing_key: SecretKey<Ed25519>,
    pub update_successful: bool,
}

impl Debug for UpdateProposalData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let leaders: Vec<BftLeaderId> = self.leaders.keys().cloned().collect();
        f.debug_struct("UpdateProposalData")
            .field("leaders", &leaders)
            .field("proposal", &self.proposal)
            .field("proposal_id", &self.proposal.proposal().proposal().id())
            .field("votes", &self.votes)
            .finish()
    }
}

impl UpdateProposalData {
    pub fn leaders_ids(&self) -> Vec<BftLeaderId> {
        self.leaders.keys().cloned().collect()
    }

    pub fn proposal_settings(&self) -> ConfigParams {
        self.proposal.proposal().proposal().changes().clone()
    }
}

impl Arbitrary for UpdateProposalData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let leader_size = 1; //usize::arbitrary(gen) % 20 + 1;
        let leaders: HashMap<BftLeaderId, SecretKey<Ed25519>> = iter::from_fn(|| {
            let sk: SecretKey<Ed25519> = Arbitrary::arbitrary(gen);
            let leader_id = BftLeaderId(sk.to_public());
            Some((leader_id, sk))
        })
        .take(leader_size)
        .collect();

        let voters: HashMap<BftLeaderId, SecretKey<Ed25519>> =
            arbitrary_utils::choose_random_map_subset(&leaders, gen);
        let leaders_ids: Vec<BftLeaderId> = leaders.keys().cloned().collect();
        let proposer_id = arbitrary_utils::choose_random_item(&leaders_ids, gen);
        let proposer_secret_key = leaders.get(&proposer_id).unwrap();

        //create proposal
        let unique_arbitrary_settings: Vec<ConfigParam> = vec![
            ConfigParam::SlotsPerEpoch(u32::arbitrary(gen)),
            ConfigParam::SlotDuration(u8::arbitrary(gen)),
            ConfigParam::EpochStabilityDepth(u32::arbitrary(gen)),
            ConfigParam::BlockContentMaxSize(u32::arbitrary(gen)),
            ConfigParam::LinearFee(LinearFee::arbitrary(gen)),
            ConfigParam::ProposalExpiration(u32::arbitrary(gen)),
        ];

        let update_proposal = ProposalBuilder::new()
            .with_proposal_changes(arbitrary_utils::choose_random_vec_subset(
                &unique_arbitrary_settings,
                gen,
            ))
            .build();

        let proposal_id = update_proposal.id();

        let signed_update_proposal = SignedProposalBuilder::new()
            .with_proposal_update(update_proposal)
            .with_proposer_secret_key(proposer_secret_key.clone())
            .build();

        // create signed votes
        let signed_votes: Vec<SignedUpdateVote> = voters
            .iter()
            .map(|(id, _)| {
                let update_vote = UpdateVote::new(proposal_id, id.clone());
                SignedUpdateVote::new(
                    make_signature(proposer_secret_key, &update_vote),
                    update_vote,
                )
            })
            .collect();

        let sk: chain_crypto::SecretKey<Ed25519> = Arbitrary::arbitrary(gen);
        let update_successful = signed_votes.len() > (leaders.len() / 2);

        UpdateProposalData {
            leaders,
            proposal: signed_update_proposal,
            votes: signed_votes,
            block_signing_key: sk,
            update_successful,
        }
    }
}
