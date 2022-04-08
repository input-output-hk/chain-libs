use super::*;
use crate::block::BlockDate;
use crate::fragment::ConfigParams;
use crate::ledger::governance::TreasuryGovernanceAction;
use crate::rewards::TaxType;
use crate::testing::data::CommitteeMembersManager;
use crate::vote;
use crate::{accounting::account::DelegationType, tokens::identifier::TokenIdentifier};
#[cfg(test)]
use chain_core::{packer::Codec, property::DeserializeFromSlice};
use chain_crypto::{testing, Ed25519};
use chain_time::DurationSeconds;
use chain_vote::{Crs, EncryptedTally};
#[allow(unused_imports)] // proptest macro bug
use proptest::prop_assert_eq;
use quickcheck::{Arbitrary, Gen};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use std::num::NonZeroU8;
use test_strategy::proptest;

impl Arbitrary for PoolRetirement {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let retirement_time = DurationSeconds::from(u64::arbitrary(g)).into();
        PoolRetirement {
            pool_id: Arbitrary::arbitrary(g),
            retirement_time,
        }
    }
}

impl Arbitrary for PoolUpdate {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let pool_id = Arbitrary::arbitrary(g);
        let last_pool_reg_hash = Arbitrary::arbitrary(g);
        let new_pool_reg = Arbitrary::arbitrary(g);

        PoolUpdate {
            pool_id,
            last_pool_reg_hash,
            new_pool_reg,
        }
    }
}

impl Arbitrary for PoolOwnersSigned {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut signatoree = u8::arbitrary(g) % 32;
        if signatoree == 0 {
            signatoree = 1;
        }

        let mut signatures = Vec::new();
        for i in 0..signatoree {
            let s = Arbitrary::arbitrary(g);
            signatures.push((i, s));
        }
        PoolOwnersSigned { signatures }
    }
}

impl Arbitrary for PoolSignature {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if bool::arbitrary(g) {
            PoolSignature::Operator(Arbitrary::arbitrary(g))
        } else {
            PoolSignature::Owners(Arbitrary::arbitrary(g))
        }
    }
}

impl Arbitrary for PoolPermissions {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        PoolPermissions::new(u8::arbitrary(g))
    }
}

impl Arbitrary for DelegationType {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        DelegationType::Full(Arbitrary::arbitrary(g))
    }
}

impl Arbitrary for StakeDelegation {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        StakeDelegation {
            account_id: Arbitrary::arbitrary(g),
            delegation: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for OwnerStakeDelegation {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Self {
            delegation: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for UpdateProposal {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut changes = ConfigParams::new();
        for _ in 0..u8::arbitrary(g) % 10 {
            changes.push(Arbitrary::arbitrary(g));
        }
        let proposer_id = UpdateProposerId::arbitrary(g);
        Self::new(changes, proposer_id)
    }
}

impl Arbitrary for UpdateVote {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let proposal_id = UpdateProposalId::arbitrary(g);
        let voter_id = UpdateVoterId::arbitrary(g);
        Self::new(proposal_id, voter_id)
    }
}

impl Arbitrary for PoolRegistration {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let start_validity: DurationSeconds = u64::arbitrary(g).into();
        let keys = Arbitrary::arbitrary(g);

        let nb_owners = usize::arbitrary(g) % 32;
        let nb_operators = usize::arbitrary(g) % 4;

        let mut owners = Vec::new();
        for _ in 0..nb_owners {
            let pk = testing::arbitrary_public_key::<Ed25519, G>(g);
            owners.push(pk)
        }

        let mut operators = Vec::new();
        for _ in 0..nb_operators {
            let pk = testing::arbitrary_public_key::<Ed25519, G>(g);
            operators.push(pk)
        }

        PoolRegistration {
            serial: Arbitrary::arbitrary(g),
            permissions: PoolPermissions::new(1),
            start_validity: start_validity.into(),
            owners,
            operators: operators.into(),
            rewards: TaxType::zero(),
            reward_account: None,
            keys,
        }
    }
}

impl Arbitrary for TreasuryGovernanceAction {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        TreasuryGovernanceAction::TransferToRewards {
            value: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for VoteAction {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if let Some(action) = Arbitrary::arbitrary(g) {
            VoteAction::Treasury { action }
        } else {
            VoteAction::OffChain
        }
    }
}

impl Arbitrary for Proposal {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let external_id = ExternalProposalId::arbitrary(g);
        let funding_plan = vote::Options::arbitrary(g);
        let action = VoteAction::arbitrary(g);

        Self::new(external_id, funding_plan, action)
    }
}

impl Arbitrary for Proposals {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let len = usize::arbitrary(g) % Proposals::MAX_LEN;
        let mut proposals = Proposals::new();
        for _ in 0..len {
            if let PushProposal::Success = proposals.push(Proposal::arbitrary(g)) {
                // pushed successfully
            } else {
                unreachable!("only generates what is needed")
            }
        }

        proposals
    }
}

impl Arbitrary for VotePlan {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let vote_start = BlockDate::arbitrary(g);
        let vote_end = BlockDate::arbitrary(g);
        let committee_end = BlockDate::arbitrary(g);
        let proposals = Proposals::arbitrary(g);
        let payload_type = vote::PayloadType::arbitrary(g);

        let mut keys = Vec::new();
        // it should have been 256 but is limited for the sake of adequate test times
        let keys_n = g.next_u32() % 15 + 1;
        let mut seed = [0u8; 32];
        g.fill_bytes(&mut seed);
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(seed);
        let h = chain_vote::Crs::from_hash(&seed);
        for _i in 0..keys_n {
            let mc = chain_vote::MemberCommunicationKey::new(&mut rng);
            let threshold = 1;
            let m1 = chain_vote::MemberState::new(&mut rng, threshold, &h, &[mc.to_public()], 0);
            keys.push(m1.public_key());
        }

        let voting_token = TokenIdentifier::arbitrary(g);

        Self::new(
            vote_start,
            vote_end,
            committee_end,
            proposals,
            payload_type,
            keys,
            voting_token,
        )
    }
}

impl Arbitrary for VotePlanProof {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Self {
            id: Arbitrary::arbitrary(g),
            signature: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for VoteCast {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let vote_plan = VotePlanId::arbitrary(g);
        let proposal_index = u8::arbitrary(g);
        let payload = vote::Payload::arbitrary(g);

        VoteCast::new(vote_plan, proposal_index, payload)
    }
}

fn arbitrary_decrypted_private_tally<G: Gen>(g: &mut G) -> DecryptedPrivateTally {
    let proposals_n = u8::arbitrary(g);
    let mut inner = Vec::new();
    let mut rng = ChaChaRng::seed_from_u64(u64::arbitrary(g));
    let crs_seed = String::arbitrary(g).into_bytes();
    let committee_size = (g.next_u32() % 2 + 1) as usize; // very time consuming
    let committee_manager =
        CommitteeMembersManager::new(&mut rng, &crs_seed, committee_size, committee_size);

    for _ in 0..proposals_n {
        let n_options = NonZeroU8::arbitrary(g);

        let encrypted_tally = EncryptedTally::new(
            n_options.get() as usize,
            committee_manager.election_pk(),
            Crs::from_hash(&crs_seed),
        );

        let mut decrypte_shares = Vec::new();
        for i in 0..committee_size {
            decrypte_shares.push(
                encrypted_tally
                    .partial_decrypt(&mut rng, committee_manager.members()[i].secret_key()),
            );
        }

        inner.push(DecryptedPrivateTallyProposal {
            tally_result: (0..n_options.get())
                .map(|_| u64::arbitrary(g))
                .collect::<Box<[_]>>(),
            decrypt_shares: decrypte_shares.into_boxed_slice(),
        });
    }
    DecryptedPrivateTally::new(inner).unwrap()
}

impl Arbitrary for VoteTally {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let vote_plan_id = VotePlanId::arbitrary(g);

        let private = bool::arbitrary(g);

        if private {
            Self::new_private(vote_plan_id, arbitrary_decrypted_private_tally(g))
        } else {
            Self::new_public(vote_plan_id)
        }
    }
}

impl Arbitrary for TallyProof {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Self::Public {
            id: Arbitrary::arbitrary(g),
            signature: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for Certificate {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let option = u8::arbitrary(g) % 11;
        match option {
            0 => Certificate::StakeDelegation(Arbitrary::arbitrary(g)),
            1 => Certificate::OwnerStakeDelegation(Arbitrary::arbitrary(g)),
            2 => Certificate::PoolRegistration(Arbitrary::arbitrary(g)),
            3 => Certificate::PoolRetirement(Arbitrary::arbitrary(g)),
            4 => Certificate::PoolUpdate(Arbitrary::arbitrary(g)),
            5 => Certificate::VotePlan(Arbitrary::arbitrary(g)),
            6 => Certificate::VoteCast(Arbitrary::arbitrary(g)),
            7 => Certificate::VoteTally(Arbitrary::arbitrary(g)),
            8 => Certificate::UpdateProposal(Arbitrary::arbitrary(g)),
            9 => Certificate::UpdateVote(Arbitrary::arbitrary(g)),
            10 => Certificate::MintToken(Arbitrary::arbitrary(g)),
            _ => panic!("unimplemented"),
        }
    }
}

// proptest macro doesn't track spans properly, so this triggers a "dead code" warning
#[proptest]
fn pool_reg_serialization_bijection(#[allow(dead_code)] b: PoolRegistration) {
    let b_got = b.serialize();
    let result = PoolRegistration::deserialize_from_slice(&mut Codec::new(b_got.as_ref())).unwrap();
    prop_assert_eq!(b, result);
}

mod pt {

    use std::num::NonZeroU8;

    use chain_vote::{Crs, EncryptedTally};
    use proptest::{
        arbitrary::StrategyFor,
        collection::{vec, VecStrategy},
        prelude::*,
        strategy::Map,
    };
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;

    use crate::{
        account::DelegationType,
        certificate::{
            DecryptedPrivateTally, DecryptedPrivateTallyProposal, ExternalProposalId, PoolId,
            Proposal, UpdateProposal, UpdateProposerId, VoteAction, VotePlanId, VoteTally,
        },
        config::ConfigParam,
        fragment::ConfigParams,
        testing::data::CommitteeMembersManager,
        tokens::identifier::TokenIdentifier,
        vote::{Options, PayloadType},
    };

    use super::{BlockDate, Proposals, VotePlan};

    prop_compose! {
        fn arb_vote_plan()(
            vote_start in any::<BlockDate>(),
            vote_end in any::<BlockDate>(),
            committee_end in any::<BlockDate>(),
            proposals in any::<Proposals>(),
            payload_type in any::<PayloadType>(),
            keys_n in 1usize..16,
            seed in any::<[u8; 32]>(),
            voting_token in any::<TokenIdentifier>(),
        ) -> VotePlan {
            let mut keys = Vec::with_capacity(keys_n);
            let mut rng = rand_chacha::ChaCha20Rng::from_seed(seed);
            let h = chain_vote::Crs::from_hash(&seed);

            for _i in 0..keys_n {
                let mc = chain_vote::MemberCommunicationKey::new(&mut rng);
                let threshold = 1;
                let m1 = chain_vote::MemberState::new(&mut rng, threshold, &h, &[mc.to_public()], 0);
                keys.push(m1.public_key());
            }

            VotePlan::new(
                vote_start,
                vote_end,
                committee_end,
                proposals,
                payload_type,
                keys,
                voting_token,
            )
        }
    }

    impl Arbitrary for VotePlan {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            arb_vote_plan().boxed()
        }
    }

    impl Arbitrary for Proposal {
        type Parameters = ();
        type Strategy = Map<
            StrategyFor<(ExternalProposalId, Options, VoteAction)>,
            fn((ExternalProposalId, Options, VoteAction)) -> Self,
        >;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<(ExternalProposalId, Options, VoteAction)>()
                .prop_map(|(id, plan, action)| Self::new(id, plan, action))
        }
    }

    impl Arbitrary for Proposals {
        type Parameters = ();
        type Strategy = Map<VecStrategy<StrategyFor<Proposal>>, fn(Vec<Proposal>) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            vec(any::<Proposal>(), 0usize..Proposals::MAX_LEN).prop_map(|proposals| {
                let mut result = Proposals::new();
                for proposal in proposals {
                    result.push(proposal).unwrap();
                }
                result
            })
        }
    }

    impl Arbitrary for DelegationType {
        type Parameters = ();
        type Strategy = Map<StrategyFor<PoolId>, fn(PoolId) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<PoolId>().prop_map(DelegationType::Full)
        }
    }

    impl Arbitrary for UpdateProposal {
        type Parameters = ();
        type Strategy = Map<
            (
                VecStrategy<StrategyFor<ConfigParam>>,
                StrategyFor<UpdateProposerId>,
            ),
            fn((Vec<ConfigParam>, UpdateProposerId)) -> Self,
        >;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            let params = vec(any::<ConfigParam>(), 0..10);
            let id = any::<UpdateProposerId>();
            (params, id).prop_map(|(params, id)| {
                let mut changes = ConfigParams::new();
                for param in params {
                    changes.push(param);
                }
                Self::new(changes, id)
            })
        }
    }

    fn arbitrary_decrypted_private_tally() -> impl Strategy<Value = DecryptedPrivateTally> {
        (any::<(String, u64)>(), 1usize..=2).prop_flat_map(|((s, seed), committee_size)| {
            let mut rng = ChaChaRng::seed_from_u64(seed);
            let crs_seed = s.into_bytes();
            let committee_manager =
                CommitteeMembersManager::new(&mut rng, &crs_seed, committee_size, committee_size);

            let single_element =
                any::<(NonZeroU8, u64, u64)>().prop_map(move |(n_options, n, seed)| {
                    let mut rng = ChaChaRng::seed_from_u64(seed);
                    let encrypted_tally = EncryptedTally::new(
                        n_options.get() as usize,
                        committee_manager.election_pk(),
                        Crs::from_hash(&crs_seed),
                    );

                    let mut decrypte_shares = Vec::new();
                    for i in 0..committee_size {
                        decrypte_shares.push(encrypted_tally.partial_decrypt(
                            &mut rng,
                            committee_manager.members()[i].secret_key(),
                        ));
                    }

                    DecryptedPrivateTallyProposal {
                        tally_result: (0..n_options.get()).map(|_| n).collect::<Box<[_]>>(),
                        decrypt_shares: decrypte_shares.into_boxed_slice(),
                    }
                });

            vec(single_element, 0..(u8::MAX as usize))
                .prop_map(|inner| DecryptedPrivateTally::new(inner).unwrap())
        })
    }

    impl Arbitrary for VoteTally {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            let private = (any::<VotePlanId>(), arbitrary_decrypted_private_tally())
                .prop_map(|(id, tally)| Self::new_private(id, tally));
            let public = any::<VotePlanId>().prop_map(Self::new_public);

            prop_oneof![private, public].boxed()
        }
    }
}
