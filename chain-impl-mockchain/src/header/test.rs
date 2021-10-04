use super::*;
use crate::block::BlockContentHash;
use crate::chaintypes::ChainLength;
use crate::header::{BftProof, BftSignature, Common, GenesisPraosProof, KesSignature};
use crate::key::BftLeaderId;
#[cfg(test)]
use crate::testing::serialization::serialization_bijection_r;
use chain_crypto::{
    self, AsymmetricKey, Ed25519, RistrettoGroup2HashDh, SecretKey, SumEd25519_12,
    VerifiableRandomFunction,
};
use lazy_static::lazy_static;
use proptest::prelude::*;
#[cfg(test)]
use quickcheck::TestResult;
use quickcheck::{Arbitrary, Gen};
use test_strategy::proptest;

#[proptest]
fn header_serialization_bijection(b: Header) {
    serialization_bijection_r(b)
}

impl Arbitrary for BlockVersion {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        BlockVersion::from_u16(u16::arbitrary(g) % 3).unwrap()
    }
}

impl Arbitrary for AnyBlockVersion {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        u16::arbitrary(g).into()
    }
}

impl Arbitrary for Common {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Common {
            block_version: Arbitrary::arbitrary(g),
            block_date: Arbitrary::arbitrary(g),
            block_content_size: Arbitrary::arbitrary(g),
            block_content_hash: Arbitrary::arbitrary(g),
            block_parent_hash: Arbitrary::arbitrary(g),
            chain_length: ChainLength(Arbitrary::arbitrary(g)),
        }
    }
}

impl Arbitrary for BftProof {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let sk: chain_crypto::SecretKey<Ed25519> = Arbitrary::arbitrary(g);
        let pk = sk.to_public();
        let signature = sk.sign(&[0u8, 1, 2, 3]);
        BftProof {
            leader_id: BftLeaderId(pk),
            signature: BftSignature(signature.coerce()),
        }
    }
}

impl proptest::arbitrary::Arbitrary for BftProof {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        any::<chain_crypto::SecretKey<Ed25519>>()
            .prop_map(|sk| {
                let pk = sk.to_public();
                let signature = sk.sign(&[0u8, 1, 2, 3]);
                BftProof {
                    leader_id: BftLeaderId(pk),
                    signature: BftSignature(signature.coerce()),
                }
            })
            .boxed()
    }
}

impl Arbitrary for GenesisPraosProof {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        use chain_crypto::testing;
        let tcg = testing::TestCryptoGen::arbitrary(g);

        let node_id = Arbitrary::arbitrary(g);

        let vrf_proof = {
            let sk = RistrettoGroup2HashDh::generate(&mut tcg.get_rng(0));
            RistrettoGroup2HashDh::evaluate_and_prove(&sk, &[0, 1, 2, 3], &mut tcg.get_rng(1))
        };

        let kes_proof = {
            lazy_static! {
                static ref SK_FIRST: SecretKey<SumEd25519_12> = testing::static_secret_key();
            }
            let sk = SK_FIRST.clone();
            let signature = sk.sign(&[0u8, 1, 2, 3]);
            KesSignature(signature.coerce())
        };
        GenesisPraosProof {
            node_id,
            vrf_proof: vrf_proof.into(),
            kes_proof,
        }
    }
}

impl proptest::arbitrary::Arbitrary for GenesisPraosProof {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use crate::certificate::PoolId;

        (
            any::<chain_crypto::testing::TestCryptoGen>(),
            any::<PoolId>(),
        )
            .prop_map(|(tcg, node_id)| {
                let vrf_proof = {
                    let sk = RistrettoGroup2HashDh::generate(&mut tcg.get_rng(0));
                    RistrettoGroup2HashDh::evaluate_and_prove(
                        &sk,
                        &[0, 1, 2, 3],
                        &mut tcg.get_rng(1),
                    )
                };

                let kes_proof = {
                    let sk = SecretKey::<SumEd25519_12>::generate(&mut tcg.get_rng(3));
                    let signature = sk.sign(&[0u8, 1, 2, 3]);
                    KesSignature(signature.coerce())
                };

                Self {
                    node_id,
                    vrf_proof: vrf_proof.into(),
                    kes_proof,
                }
            })
            .boxed()
    }
}

impl Arbitrary for Header {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let common = Common::arbitrary(g);
        let hdrbuilder = HeaderBuilderNew::new_raw(
            common.block_version,
            &common.block_content_hash,
            common.block_content_size,
        )
        .set_parent(&common.block_parent_hash, common.chain_length)
        .set_date(common.block_date);
        match common.block_version {
            BlockVersion::Genesis => hdrbuilder.into_unsigned_header().unwrap().generalize(),
            BlockVersion::Ed25519Signed => {
                let bft_proof: BftProof = Arbitrary::arbitrary(g);
                hdrbuilder
                    .into_bft_builder()
                    .unwrap()
                    .set_consensus_data(&bft_proof.leader_id)
                    .set_signature(bft_proof.signature)
                    .generalize()
            }
            BlockVersion::KesVrfproof => {
                let gp_proof: GenesisPraosProof = Arbitrary::arbitrary(g);
                hdrbuilder
                    .into_genesis_praos_builder()
                    .unwrap()
                    .set_consensus_data(&gp_proof.node_id, &gp_proof.vrf_proof)
                    .set_signature(gp_proof.kes_proof)
                    .generalize()
            }
        }
    }
}

impl proptest::arbitrary::Arbitrary for Header {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        any::<(BlockContentHash, u32)>()
            .prop_flat_map(|(content_hash, content_size)| {
                header_strategy(content_hash, content_size)
            })
            .boxed()
    }
}

pub fn header_strategy(
    content_hash: BlockContentHash,
    content_size: u32,
) -> impl Strategy<Value = Header> {
    #[derive(Debug, Clone)]
    enum Proof {
        Genesis,
        Ed25519Signed(BftProof),
        KesVrfproof(GenesisPraosProof),
    }

    (
        any::<BlockVersion>(),
        any::<BlockContentHash>(),
        any::<ChainLength>(),
        any::<BlockDate>(),
    )
        .prop_flat_map(move |(ver, parent_hash, chain_length, date)| {
            let proof_strategy = match ver {
                BlockVersion::Genesis => Just(Proof::Genesis).boxed(),
                BlockVersion::Ed25519Signed => {
                    any::<BftProof>().prop_map(Proof::Ed25519Signed).boxed()
                }
                BlockVersion::KesVrfproof => any::<GenesisPraosProof>()
                    .prop_map(Proof::KesVrfproof)
                    .boxed(),
            };

            proof_strategy.prop_map(move |proof| {
                let hdrbuilder = HeaderBuilderNew::new_raw(ver, &content_hash, content_size)
                    .set_parent(&parent_hash, chain_length)
                    .set_date(date);

                match proof {
                    Proof::Genesis => hdrbuilder.into_unsigned_header().unwrap().generalize(),
                    Proof::Ed25519Signed(bft_proof) => hdrbuilder
                        .into_bft_builder()
                        .unwrap()
                        .set_consensus_data(&bft_proof.leader_id)
                        .set_signature(bft_proof.signature)
                        .generalize(),
                    Proof::KesVrfproof(gp_proof) => hdrbuilder
                        .into_genesis_praos_builder()
                        .unwrap()
                        .set_consensus_data(&gp_proof.node_id, &gp_proof.vrf_proof)
                        .set_signature(gp_proof.kes_proof)
                        .generalize(),
                }
            })
        })
}
