use crate::key::Hash;
use crate::{
    config::ConfigParam,
    fee::LinearFee,
    fragment::config::ConfigParams,
    key::BftLeaderId,
    testing::builders::SignedProposalBuilder,
    testing::{arbitrary::utils as arbitrary_utils, builders::update_builder::ProposalBuilder},
    update::{SignedUpdateProposal, SignedUpdateVote, UpdateVote},
};
use chain_crypto::{Ed25519, Ed25519Extended, SecretKey};
use proptest::prelude::*;
use proptest::sample::select;
use quickcheck::{Arbitrary, Gen};
use std::fmt::{self, Debug};
use std::{collections::HashMap, iter};

#[derive(Clone)]
pub struct UpdateProposalData {
    pub leaders: HashMap<BftLeaderId, SecretKey<Ed25519Extended>>,
    pub proposal: SignedUpdateProposal,
    pub proposal_id: Hash,
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
            .field("proposal_id", &self.proposal_id)
            .field("votes", &self.votes)
            .finish()
    }
}

impl UpdateProposalData {
    pub fn leaders_ids(&self) -> Vec<BftLeaderId> {
        self.leaders.keys().cloned().collect()
    }

    pub fn proposal_settings(&self) -> ConfigParams {
        self.proposal.proposal.proposal.changes.clone()
    }
}

impl Arbitrary for UpdateProposalData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let leader_size = 1; //usize::arbitrary(gen) % 20 + 1;
        let leaders: HashMap<BftLeaderId, SecretKey<Ed25519Extended>> = iter::from_fn(|| {
            let sk: SecretKey<Ed25519Extended> = Arbitrary::arbitrary(gen);
            let leader_id = BftLeaderId(sk.to_public());
            Some((leader_id, sk))
        })
        .take(leader_size)
        .collect();

        let voters: HashMap<BftLeaderId, SecretKey<Ed25519Extended>> =
            arbitrary_utils::choose_random_map_subset(&leaders, gen);
        let leaders_ids: Vec<BftLeaderId> = leaders.keys().cloned().collect();
        let proposer_id = arbitrary_utils::choose_random_item(&leaders_ids, gen);

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

        let signed_update_proposal = SignedProposalBuilder::new()
            .with_proposal_update(update_proposal)
            .with_proposer_id(proposer_id)
            .build();

        //generate proposal header
        let proposal_id = Hash::arbitrary(gen);

        // create signed votes
        let signed_votes: Vec<SignedUpdateVote> = voters
            .iter()
            .map(|(id, _)| {
                let update_vote = UpdateVote {
                    proposal_id,
                    voter_id: id.clone(),
                };
                SignedUpdateVote { vote: update_vote }
            })
            .collect();

        let sk: chain_crypto::SecretKey<Ed25519> = Arbitrary::arbitrary(gen);
        let update_successful = signed_votes.len() > (leaders.len() / 2);

        UpdateProposalData {
            leaders,
            proposal: signed_update_proposal,
            proposal_id,
            votes: signed_votes,
            block_signing_key: sk,
            update_successful,
        }
    }
}

impl proptest::arbitrary::Arbitrary for UpdateProposalData {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::collection::vec;

        let leaders_strategy = vec(any::<SecretKey<Ed25519Extended>>(), 1..=20).prop_map(|sks| {
            sks.into_iter()
                .map(|sk| {
                    let leader_id = BftLeaderId(sk.to_public());
                    (leader_id, sk)
                })
                .collect::<Vec<_>>()
        });

        let voters_leaders_proposer_strategy = leaders_strategy.prop_flat_map(|leaders| {
            let voters = vec(select(leaders.clone()), 0..leaders.len())
                .prop_map(|entries| entries.into_iter().collect::<HashMap<_, _>>());
            let leaders = leaders.into_iter().collect::<HashMap<_, _>>();
            let leaders_ids = leaders.keys().cloned().collect::<Vec<_>>();
            let proposer = select(leaders_ids);
            (Just(leaders), voters, proposer)
        });

        let settings_strategy = any::<(u32, u8, u32, u32, LinearFee, u32)>()
            .prop_map(
                |(
                    slots_per_epoch,
                    slot_duration,
                    epoch_stablility_depth,
                    block_content_max_size,
                    linear_fee,
                    proposal_expiration,
                )| {
                    vec![
                        ConfigParam::SlotsPerEpoch(slots_per_epoch),
                        ConfigParam::SlotDuration(slot_duration),
                        ConfigParam::EpochStabilityDepth(epoch_stablility_depth),
                        ConfigParam::BlockContentMaxSize(block_content_max_size),
                        ConfigParam::LinearFee(linear_fee),
                        ConfigParam::ProposalExpiration(proposal_expiration),
                    ]
                },
            )
            .prop_flat_map(|settings| {
                let settings_len = settings.len();
                vec(select(settings), 0..settings_len)
            });

        let proposal_id_strategy = any::<Hash>();

        let sk_strategy = any::<chain_crypto::SecretKey<Ed25519>>();

        (
            voters_leaders_proposer_strategy,
            settings_strategy,
            proposal_id_strategy,
            sk_strategy,
        )
            .prop_map(
                |((leaders, voters, proposer_id), settings, proposal_id, sk)| {
                    let update_proposal = ProposalBuilder::new()
                        .with_proposal_changes(settings)
                        .build();
                    let signed_update_proposal = SignedProposalBuilder::new()
                        .with_proposal_update(update_proposal)
                        .with_proposer_id(proposer_id)
                        .build();
                    let signed_votes: Vec<SignedUpdateVote> = voters
                        .iter()
                        .map(|(id, _)| {
                            let update_vote = UpdateVote {
                                proposal_id,
                                voter_id: id.clone(),
                            };
                            SignedUpdateVote { vote: update_vote }
                        })
                        .collect();
                    let update_successful = signed_votes.len() > (leaders.len() / 2);
                    UpdateProposalData {
                        leaders,
                        proposal: signed_update_proposal,
                        proposal_id,
                        votes: signed_votes,
                        block_signing_key: sk,
                        update_successful,
                    }
                },
            )
            .boxed()
    }
}
