use crate::{
    certificate::{UpdateProposal, UpdateProposalId, UpdateVote},
    config::ConfigParam,
    fee::LinearFee,
    fragment::config::ConfigParams,
    key::BftLeaderId,
    testing::arbitrary::utils as arbitrary_utils,
    testing::data::LeaderPair,
};
use chain_crypto::{Ed25519, SecretKey};
use quickcheck::{Arbitrary, Gen};
use std::fmt::Debug;
use std::{collections::HashMap, iter};

#[derive(Clone, Debug)]
pub struct UpdateProposalData {
    pub leaders: HashMap<BftLeaderId, SecretKey<Ed25519>>,
    pub voters: HashMap<BftLeaderId, SecretKey<Ed25519>>,
    pub proposal: UpdateProposal,
    pub block_signing_key: SecretKey<Ed25519>,
}

impl UpdateProposalData {
    pub fn leaders_ids(&self) -> Vec<BftLeaderId> {
        self.leaders.keys().cloned().collect()
    }

    pub fn leaders_pairs(&self) -> Vec<LeaderPair> {
        self.leaders
            .values()
            .cloned()
            .map(LeaderPair::new)
            .collect()
    }

    pub fn proposal_settings(&self) -> ConfigParams {
        self.proposal.changes().clone()
    }

    pub fn gen_votes(&self, proposal_id: UpdateProposalId) -> Vec<UpdateVote> {
        self.voters
            .iter()
            .map(|(id, _)| UpdateVote::new(proposal_id, id.clone()))
            .collect()
    }
}

mod pt {
    use std::collections::HashMap;

    use chain_crypto::{Ed25519, SecretKey};
    use proptest::{collection::vec, prelude::*};

    use crate::{
        certificate::UpdateProposal, config::ConfigParam, fee::LinearFee, fragment::ConfigParams,
        key::BftLeaderId, testing::utils::proptest::random_subset,
    };

    use super::UpdateProposalData;

    prop_compose! {
        fn settings()(
            slots_per_epoch in any::<u32>(),
            slot_duration in any::<u8>(),
            epoch_stability_depth in any::<u32>(),
            block_content_max_size in any::<u32>(),
            linear_fee in any::<LinearFee>(),
            proposal_expiration in any::<u32>(),
        ) -> Vec<ConfigParam> {
            use ConfigParam::*;
            vec![
                SlotsPerEpoch(slots_per_epoch),
                SlotDuration(slot_duration),
                EpochStabilityDepth(epoch_stability_depth),
                BlockContentMaxSize(block_content_max_size),
                LinearFee(linear_fee),
                ProposalExpiration(proposal_expiration),
            ]
        }
    }

    fn leaders() -> impl Strategy<Value = HashMap<BftLeaderId, SecretKey<Ed25519>>> {
        let pair = any::<SecretKey<Ed25519>>().prop_map(|key| {
            let leader_id = BftLeaderId(key.to_public());
            (leader_id, key)
        });
        vec(pair, 1..2).prop_map(|vec| vec.into_iter().collect())
    }

    prop_compose! {
        fn proposal_leader_key()(
            settings in random_subset(settings()),
            leaders in leaders(),
            sk in any::<SecretKey<Ed25519>>(),
        ) -> (UpdateProposal, HashMap<BftLeaderId, SecretKey<Ed25519>>, SecretKey<Ed25519>) {
            let proposal = UpdateProposal::new(
                ConfigParams(settings),
                leaders.iter().next().unwrap().0.clone(),
            );

            (proposal, leaders, sk)
        }
    }

    impl Arbitrary for UpdateProposalData {
        type Strategy = BoxedStrategy<Self>;
        type Parameters = ();

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            proposal_leader_key()
                .prop_flat_map(|(proposal, leaders, key)| {
                    random_subset(Just(leaders.clone())).prop_map(move |voters| {
                        UpdateProposalData {
                            leaders: leaders.clone().into_iter().collect(),
                            voters: voters.into_iter().collect(),
                            proposal: proposal.clone(),
                            block_signing_key: key.clone(),
                        }
                    })
                })
                .boxed()
        }
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

        //create proposal
        let unique_arbitrary_settings: Vec<ConfigParam> = vec![
            ConfigParam::SlotsPerEpoch(u32::arbitrary(gen)),
            ConfigParam::SlotDuration(u8::arbitrary(gen)),
            ConfigParam::EpochStabilityDepth(u32::arbitrary(gen)),
            ConfigParam::BlockContentMaxSize(u32::arbitrary(gen)),
            ConfigParam::LinearFee(LinearFee::arbitrary(gen)),
            ConfigParam::ProposalExpiration(u32::arbitrary(gen)),
        ];

        let proposal = UpdateProposal::new(
            ConfigParams(arbitrary_utils::choose_random_vec_subset(
                &unique_arbitrary_settings,
                gen,
            )),
            leaders.iter().next().unwrap().0.clone(),
        );

        let sk: chain_crypto::SecretKey<Ed25519> = Arbitrary::arbitrary(gen);

        UpdateProposalData {
            leaders: leaders.into_iter().collect(),
            voters: voters.into_iter().collect(),
            proposal,
            block_signing_key: sk,
        }
    }
}
