//! block builder tooling and more
//!

use crate::block::{
    BftProof, Block, BlockContentHash, BlockContents, BlockDate, BlockId, BlockVersion,
    ChainLength, Common, Fragment, GenesisPraosProof, Header, KESSignature, Proof,
};
use crate::certificate::PoolId;
use crate::key::make_signature;
use crate::leadership;
use crate::transaction::{AuthenticatedTransaction, NoExtra};
use chain_addr::Address;
use chain_crypto::{
    Curve25519_2HashDH, Ed25519, SecretKey, SumEd25519_12, VerifiableRandomFunction,
};

pub struct BlockBuilder {
    pub common: Common,
    pub contents: BlockContents,
}

impl From<Block> for BlockBuilder {
    fn from(block: Block) -> BlockBuilder {
        BlockBuilder {
            common: block.header.common,
            contents: block.contents,
        }
    }
}

/// block builder to build and finalize the construction of a block
impl BlockBuilder {
    /// default setting, equivalent to writing a genesis block (the empty block)
    pub fn new() -> BlockBuilder {
        use chain_core::property::BlockId;
        BlockBuilder {
            common: Common {
                block_content_size: 0,
                block_content_hash: BlockContentHash::zero(),
                any_block_version: BlockVersion::Genesis.into(),
                block_parent_hash: BlockId::zero(),
                block_date: BlockDate::first(),
                chain_length: ChainLength(0),
            },
            contents: BlockContents::new(Vec::new()),
        }
    }

    /// set the block date
    pub fn date(&mut self, block_date: BlockDate) -> &mut Self {
        self.common.block_date = block_date;
        self
    }

    /// set the chain_length
    pub fn chain_length(&mut self, chain_length: ChainLength) -> &mut Self {
        self.common.chain_length = chain_length;
        self
    }

    /// set the parent hash
    pub fn parent(&mut self, block_parent_hash: BlockId) -> &mut Self {
        self.common.block_parent_hash = block_parent_hash;
        self
    }

    /// set a transaction in the block to build
    ///
    /// Equivalent to call `block_builder.message(Fragment::Transaction(transaction))`
    pub fn transaction(
        &mut self,
        signed_transaction: AuthenticatedTransaction<Address, NoExtra>,
    ) -> &mut Self {
        self.message(Fragment::Transaction(signed_transaction))
    }

    /// add a message in the block to build
    pub fn message(&mut self, message: Fragment) -> &mut Self {
        self.contents.0.push(message);
        self
    }

    /// set multiple messages in the block to build
    pub fn messages<I>(&mut self, messages: I) -> &mut Self
    where
        I: IntoIterator<Item = Fragment>,
    {
        self.contents.0.extend(messages);
        self
    }

    fn make_block(self, proof: Proof) -> Block {
        Block {
            header: Header {
                common: self.common,
                proof: proof,
            },
            contents: self.contents,
        }
    }

    fn finalize_common(&mut self, block_version: BlockVersion) -> &mut Self {
        let (content_hash, content_size) = self.contents.compute_hash_size();
        self.common.block_content_hash = content_hash;
        self.common.block_content_size = content_size as u32;
        self.common.any_block_version = block_version.into();
        self
    }

    /// create a genesis block (i.e. no signature)
    ///
    /// This is the first ever block of the blockchain and it is expected
    /// the data to be `0.0` and the hash to be `00000000000000...`.
    pub fn make_genesis_block(mut self) -> Block {
        use chain_core::property::BlockId as _;
        assert!(self.common.block_parent_hash == BlockId::zero());
        assert!(self.common.block_date == BlockDate::first());
        assert_eq!(self.common.chain_length, ChainLength(0));
        self.finalize_common(BlockVersion::Genesis);
        self.make_block(Proof::None)
    }

    /// create a BFT Block. this block will be signed with the given private key
    pub fn make_bft_block(mut self, bft_signing_key: &SecretKey<Ed25519>) -> Block {
        assert_ne!(self.common.chain_length, ChainLength(0));
        self.finalize_common(BlockVersion::Ed25519Signed);
        let bft_proof = BftProof {
            leader_id: leadership::bft::LeaderId(bft_signing_key.to_public()),
            signature: super::BftSignature(make_signature(bft_signing_key, &self.common)),
        };
        self.make_block(Proof::Bft(bft_proof))
    }

    /// create a Praos/Genesis block, this block will be signed with the
    /// given KES key.
    pub fn make_genesis_praos_block(
        mut self,
        node_id: &PoolId,
        kes_signing_key: &SecretKey<SumEd25519_12>,
        vrf_proof: <Curve25519_2HashDH as VerifiableRandomFunction>::VerifiedRandomOutput,
    ) -> Block {
        assert_ne!(self.common.chain_length, ChainLength(0));
        self.finalize_common(BlockVersion::KesVrfproof);

        let genesis_praos_proof = GenesisPraosProof {
            node_id: node_id.clone(),
            vrf_proof: vrf_proof,
            // ! SECURITY FIXME ! : also include id and vrf proof.
            kes_proof: KESSignature(make_signature(kes_signing_key, &self.common)),
        };
        self.make_block(Proof::GenesisPraos(genesis_praos_proof))
    }
}

#[cfg(test)]
mod tests {

    use super::{BlockBuilder, BlockContents, BlockDate, BlockId, BlockVersion, ChainLength};
    use crate::block::{
        header::{Common, GenesisPraosProof, Header},
        version::BlockVersion::{Ed25519Signed, KesVrfproof},
        AnyBlockVersion, Block,
    };
    use crate::testing::arbitrary::utils::Verify;
    use chain_core::property::BlockId as BlockIdProperty;
    use chain_crypto::{testing::TestCryptoGen, Ed25519, SumEd25519_12};
    use quickcheck::TestResult;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    pub fn make_genesis_block(block_content: BlockContents) -> TestResult {
        let mut builder = BlockBuilder::new();
        builder.messages(block_content.iter().cloned());
        let block = builder.make_genesis_block();

        let (content_hash, content_size) = block_content.compute_hash_size();

        let expected_common = Common {
            any_block_version: AnyBlockVersion::Supported(BlockVersion::Genesis),
            block_date: BlockDate::first(),
            block_content_size: content_size as u32,
            block_content_hash: content_hash,
            block_parent_hash: BlockId::zero(),
            chain_length: ChainLength(0),
        };

        verify_block(block, expected_common, block_content)
    }

    pub fn verify_block(
        block: Block,
        expected_common: Common,
        expected_block_content: BlockContents,
    ) -> TestResult {
        let mut verify = Verify::new();
        verify.verify_eq(
            block.contents.clone(),
            expected_block_content.clone(),
            "block contents",
        );
        verify.verify_eq(
            block.header.common.clone(),
            expected_common.clone(),
            "block header common",
        );
        verify.get_result()
    }

    #[quickcheck]
    pub fn make_genesis_praos_block(
        key_gen: TestCryptoGen,
        parent_header: Header,
        genesis_praos_proof: GenesisPraosProof,
        block_content: BlockContents,
    ) -> TestResult {
        let (content_hash, content_size) = block_content.compute_hash_size();
        let expected_common = Common {
            any_block_version: AnyBlockVersion::Supported(KesVrfproof),
            block_date: *parent_header.block_date(),
            block_content_size: content_size as u32,
            block_content_hash: content_hash,
            block_parent_hash: parent_header.hash(),
            chain_length: ChainLength(parent_header.chain_length().0 + 1),
        };

        let kes_signing_key = key_gen.secret_key::<SumEd25519_12>(0);

        let mut builder = BlockBuilder::new();
        builder.messages(block_content.iter().cloned());
        builder.date(expected_common.block_date);
        builder.chain_length(expected_common.chain_length);
        builder.parent(parent_header.hash());
        let block = builder.make_genesis_praos_block(
            &genesis_praos_proof.node_id,
            &kes_signing_key,
            genesis_praos_proof.vrf_proof,
        );

        verify_block(block, expected_common, block_content)
    }

    #[quickcheck]
    pub fn make_bft_block(
        key_gen: TestCryptoGen,
        parent_header: Header,
        block_content: BlockContents,
    ) -> TestResult {
        let (content_hash, content_size) = block_content.compute_hash_size();
        let expected_common = Common {
            any_block_version: AnyBlockVersion::Supported(Ed25519Signed),
            block_date: *parent_header.block_date(),
            block_content_size: content_size as u32,
            block_content_hash: content_hash,
            block_parent_hash: parent_header.hash(),
            chain_length: ChainLength(parent_header.chain_length().0 + 1),
        };

        let bft_signing_key = key_gen.secret_key::<Ed25519>(0);

        let mut builder = BlockBuilder::new();
        builder.messages(block_content.iter().cloned());
        builder.date(expected_common.block_date);
        builder.chain_length(expected_common.chain_length);
        builder.parent(parent_header.hash());
        let block = builder.make_bft_block(&bft_signing_key);

        verify_block(block, expected_common, block_content)
    }
}
