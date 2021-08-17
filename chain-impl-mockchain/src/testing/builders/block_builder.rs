use crate::{
    block::Block,
    chaintypes::ChainLength,
    date::BlockDate,
    fragment::{Contents, ContentsBuilder, Fragment},
    header::{BlockVersion, Header, HeaderBuilderNew},
    key::Hash,
    testing::{data::StakePool, TestGen},
};
use chain_time::TimeEra;

pub struct GenesisPraosBlockBuilder {
    date: Option<BlockDate>,
    chain_length: Option<ChainLength>,
    parent_id: Option<Hash>,
    contents_builder: ContentsBuilder,
}

impl Default for GenesisPraosBlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GenesisPraosBlockBuilder {
    pub fn new() -> Self {
        GenesisPraosBlockBuilder {
            date: None,
            chain_length: None,
            parent_id: None,
            contents_builder: ContentsBuilder::new(),
        }
    }

    pub fn with_parent(self, parent: &Header) -> Self {
        self.with_parent_id(parent.hash())
            .with_date(parent.block_date())
            .with_chain_length(parent.chain_length())
    }

    pub fn with_parent_id(mut self, parent_id: Hash) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_date(mut self, date: BlockDate) -> Self {
        self.date = Some(date);
        self
    }

    pub fn with_chain_length(mut self, chain_length: ChainLength) -> Self {
        self.chain_length = Some(chain_length);
        self
    }

    pub fn with_fragment(mut self, fragment: Fragment) -> Self {
        self.contents_builder.push(fragment);
        self
    }

    pub fn with_fragments(self, fragments: Vec<Fragment>) -> Self {
        fragments
            .into_iter()
            .fold(self, |builder, fragment| builder.with_fragment(fragment))
    }

    pub fn build(self, stake_pool: &StakePool, time_era: &TimeEra) -> Block {
        if self.date.is_none() || self.chain_length.is_none() || self.parent_id.is_none() {
            panic!("date,chain_length or hash is not set");
        }
        let vrf_proof = TestGen::vrf_proof(&stake_pool);
        let contents: Contents = self.contents_builder.clone().into();
        let header = HeaderBuilderNew::new(BlockVersion::KesVrfproof, &contents)
            .set_parent(
                &self.parent_id.unwrap(),
                self.chain_length.unwrap().increase(),
            )
            .set_date(self.date.unwrap().next(time_era))
            .into_genesis_praos_builder()
            .unwrap()
            .set_consensus_data(&stake_pool.id(), &vrf_proof)
            .sign_using(stake_pool.kes().private_key())
            .generalize();

        Block { header, contents }
    }
}
