use crate::block::Header;
#[cfg(test)]
use crate::testing::serialization::serialization_bijection_prop;
#[cfg(test)]
use proptest::prelude::ProptestConfig;
use crate::{
    block::{Block, BlockVersion},
    fragment::{Contents, ContentsBuilder, Fragment},
    header::{BftProof, GenesisPraosProof, HeaderBuilderNew},
};
#[cfg(test)]
use chain_core::{
    packer::Codec,
    property::{Block as _, Deserialize, Serialize},
};
#[cfg(test)]
use proptest::prop_assert_eq;
use quickcheck::{Arbitrary, Gen};
use test_strategy::proptest;

#[proptest]
fn header_serialization_bijection(#[allow(dead_code)] b: Header) {
    serialization_bijection_prop(b);
}

#[proptest]
fn block_serialization_bijection(#[allow(dead_code)] b: Block) {
    serialization_bijection_prop(b);
}

#[proptest(ProptestConfig {
    max_flat_map_regens: 10,
    cases: 10,
    ..Default::default()
})]
fn header_properties(#[allow(dead_code)] block: Block) {
    use chain_core::property::Header as Prop;
    let header = block.header.clone();

    prop_assert_eq!(header.hash(), block.id());
    prop_assert_eq!(header.id(), block.id());
    prop_assert_eq!(header.parent_id(), block.parent_id());
    prop_assert_eq!(header.date(), block.date());
    prop_assert_eq!(header.version(), block.version());
    prop_assert_eq!(header.get_bft_leader_id(), block.header.get_bft_leader_id());
    prop_assert_eq!(header.get_stakepool_id(), block.header.get_stakepool_id());
    prop_assert_eq!(header.common(), block.header.common());
    prop_assert_eq!(header.to_raw(), block.header.to_raw());
    prop_assert_eq!(header.as_auth_slice(), block.header.as_auth_slice());
    prop_assert_eq!(header.description().id, block.header.description().id);
    prop_assert_eq!(header.size(), block.header.size());

    prop_assert_eq!(header.chain_length(), block.chain_length());
}

// TODO: add a separate test with headers with correct content size to stress hash
// checking when tests are migrated to proptest
#[proptest(ProptestConfig {
    max_flat_map_regens: 10,
    cases: 10,
    ..Default::default()
})]
fn inconsistent_block_deserialization(
    #[allow(dead_code)] header: Header,
    #[allow(dead_code)] contents: Contents,
) {
    let (content_hash, content_size) = contents.compute_hash_size();

    let maybe_block = Block {
        header: header.clone(),
        contents,
    };

    let block = Block::deserialize(&mut Codec::new(
        maybe_block.serialize_as_vec().unwrap().as_slice(),
    ));
    let should_err =
        content_hash != header.block_content_hash() || content_size != header.block_content_size();
    prop_assert_eq!(should_err, block.is_err());
}

impl Arbitrary for Contents {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let len = u8::arbitrary(g) % 12;
        let fragments: Vec<Fragment> = std::iter::repeat_with(|| Arbitrary::arbitrary(g))
            .take(len as usize)
            .collect();
        let mut content = ContentsBuilder::new();
        content.push_many(fragments);
        content.into()
    }
}

impl Arbitrary for Block {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let content = Contents::arbitrary(g);
        let ver = BlockVersion::arbitrary(g);
        let parent_hash = Arbitrary::arbitrary(g);
        let chain_length = Arbitrary::arbitrary(g);
        let date = Arbitrary::arbitrary(g);
        let hdrbuilder = HeaderBuilderNew::new(ver, &content)
            .set_parent(&parent_hash, chain_length)
            .set_date(date);
        let header = match ver {
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
        };
        Block {
            header,
            contents: content,
        }
    }
}

mod prop_impl {
    use proptest::arbitrary::StrategyFor;
    use proptest::collection::{vec, VecStrategy};
    use proptest::prelude::*;
    use proptest::strategy::Map;

    use crate::block::{
        BftProof, Block, BlockDate, BlockVersion, ChainLength, Contents, ContentsBuilder,
        GenesisPraosProof,
    };
    use crate::fragment::Fragment;
    use crate::header::HeaderBuilderNew;
    use crate::key::Hash;

    prop_compose! {
        fn block_strategy()(
            content in any::<Contents>(),
            ver in any::<BlockVersion>(),
            parent_hash in any::<Hash>(),
            chain_length in any::<ChainLength>(),
            date in any::<BlockDate>(),
            bft_proof in any::<BftProof>(),
            gp_proof in any::<GenesisPraosProof>(),
        ) -> Block {
            let hdrbuilder = HeaderBuilderNew::new(ver, &content)
                .set_parent(&parent_hash, chain_length)
                .set_date(date);
            let header = match ver {
                BlockVersion::Genesis => hdrbuilder.into_unsigned_header().unwrap().generalize(),
                BlockVersion::Ed25519Signed => {
                    hdrbuilder
                        .into_bft_builder()
                        .unwrap()
                        .set_consensus_data(&bft_proof.leader_id)
                        .set_signature(bft_proof.signature)
                        .generalize()
                }
                BlockVersion::KesVrfproof => {
                    hdrbuilder
                        .into_genesis_praos_builder()
                        .unwrap()
                        .set_consensus_data(&gp_proof.node_id, &gp_proof.vrf_proof)
                        .set_signature(gp_proof.kes_proof)
                        .generalize()
                }
            };
            Block {
                header,
                contents: content,
            }
        }
    }

    impl Arbitrary for Block {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            block_strategy().boxed()
        }
    }

    impl Arbitrary for Contents {
        type Parameters = ();
        type Strategy = Map<VecStrategy<StrategyFor<Fragment>>, fn(Vec<Fragment>) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            vec(any::<Fragment>(), 0..12).prop_map(|fragments| {
                let mut content = ContentsBuilder::new();
                content.push_many(fragments);
                content.into()
            })
        }
    }
}
