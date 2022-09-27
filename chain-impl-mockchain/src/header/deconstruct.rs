use super::components::{BftSignature, KesSignature, VrfProof};
use super::version::BlockVersion;
use crate::certificate::PoolId;
use crate::chaintypes::{ChainLength, HeaderId};
use crate::date::BlockDate;
use crate::fragment::{BlockContentHash, BlockContentSize};
use crate::key::BftLeaderId;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "property-test-api"),
    derive(test_strategy::Arbitrary)
)]
pub struct Common {
    pub block_version: BlockVersion,
    pub block_date: BlockDate,
    pub block_content_size: BlockContentSize,
    pub block_content_hash: BlockContentHash,
    pub block_parent_hash: HeaderId,
    pub chain_length: ChainLength,
}

#[derive(Debug, Clone)]
pub enum Proof {
    /// In case there is no need for consensus layer and no need for proof of the
    /// block. This may apply to the genesis block for example.
    None,
    Bft(BftProof),
    GenesisPraos(GenesisPraosProof),
}

#[derive(Debug, Clone)]
pub struct BftProof {
    pub(crate) leader_id: BftLeaderId,
    pub(crate) signature: BftSignature,
}

#[derive(Debug, Clone)]
pub struct GenesisPraosProof {
    pub(crate) node_id: PoolId,
    pub(crate) vrf_proof: VrfProof,
    pub(crate) kes_proof: KesSignature,
}
