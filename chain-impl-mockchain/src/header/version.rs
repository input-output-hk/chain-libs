use super::cstruct;
use crate::chaintypes::ConsensusType;
use std::num::NonZeroUsize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnyBlockVersion {
    Supported(BlockVersion),
    Unsupported(u8),
}

impl AnyBlockVersion {
    pub fn try_into_block_version(self) -> Option<BlockVersion> {
        match self {
            AnyBlockVersion::Supported(version) => Some(version),
            AnyBlockVersion::Unsupported(_) => None,
        }
    }
}

impl PartialEq<BlockVersion> for AnyBlockVersion {
    fn eq(&self, other: &BlockVersion) -> bool {
        match self {
            AnyBlockVersion::Supported(version) => version == other,
            AnyBlockVersion::Unsupported(_) => false,
        }
    }
}

impl From<u8> for AnyBlockVersion {
    fn from(n: u8) -> Self {
        match BlockVersion::from_u8(n) {
            Some(supported) => AnyBlockVersion::Supported(supported),
            None => AnyBlockVersion::Unsupported(n),
        }
    }
}

impl From<AnyBlockVersion> for u8 {
    fn from(block_version: AnyBlockVersion) -> u8 {
        match block_version {
            AnyBlockVersion::Supported(version) => version as u8,
            AnyBlockVersion::Unsupported(n) => n,
        }
    }
}

impl From<BlockVersion> for AnyBlockVersion {
    fn from(version: BlockVersion) -> Self {
        AnyBlockVersion::Supported(version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "property-test-api"),
    derive(test_strategy::Arbitrary)
)]
pub enum BlockVersion {
    Genesis,
    Ed25519Signed,
    KesVrfproof,
}

impl BlockVersion {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            cstruct::VERSION_UNSIGNED => Some(BlockVersion::Genesis),
            cstruct::VERSION_BFT => Some(BlockVersion::Ed25519Signed),
            cstruct::VERSION_GP => Some(BlockVersion::KesVrfproof),
            _ => None,
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            BlockVersion::Genesis => cstruct::VERSION_UNSIGNED,
            BlockVersion::Ed25519Signed => cstruct::VERSION_BFT,
            BlockVersion::KesVrfproof => cstruct::VERSION_GP,
        }
    }

    pub const fn get_size(self) -> NonZeroUsize {
        const SIZE: [NonZeroUsize; 3] = [
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_COMMON_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_BFT_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_GP_SIZE) },
        ];
        SIZE[self as usize]
    }

    pub const fn get_auth_size(self) -> NonZeroUsize {
        const SIZE: [NonZeroUsize; 3] = [
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_COMMON_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_BFT_AUTHED_SIZE) },
            unsafe { NonZeroUsize::new_unchecked(cstruct::HEADER_GP_AUTHED_SIZE) },
        ];
        SIZE[self as usize]
    }

    pub fn to_consensus_type(self) -> Option<ConsensusType> {
        match self {
            BlockVersion::Genesis => None,
            BlockVersion::Ed25519Signed => Some(ConsensusType::Bft),
            BlockVersion::KesVrfproof => Some(ConsensusType::GenesisPraos),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::chaintypes::ConsensusType;
    use crate::header::{AnyBlockVersion, BlockVersion};
    use proptest::prop_assert_eq;
    use test_strategy::proptest;

    #[test]
    pub fn try_into_block_version() {
        assert_eq!(
            AnyBlockVersion::Supported(BlockVersion::Genesis).try_into_block_version(),
            Some(BlockVersion::Genesis)
        );
        assert_eq!(
            AnyBlockVersion::Supported(BlockVersion::Ed25519Signed).try_into_block_version(),
            Some(BlockVersion::Ed25519Signed)
        );
        assert_eq!(
            AnyBlockVersion::Supported(BlockVersion::KesVrfproof).try_into_block_version(),
            Some(BlockVersion::KesVrfproof)
        );
        assert_eq!(
            AnyBlockVersion::Unsupported(0).try_into_block_version(),
            None
        );
    }

    #[test]
    pub fn equality() {
        assert_eq!(
            AnyBlockVersion::Supported(BlockVersion::Genesis),
            BlockVersion::Genesis
        );
        assert_eq!(
            AnyBlockVersion::Supported(BlockVersion::Ed25519Signed),
            BlockVersion::Ed25519Signed
        );
        assert_eq!(
            AnyBlockVersion::Supported(BlockVersion::KesVrfproof),
            BlockVersion::KesVrfproof
        );
        assert!(AnyBlockVersion::Unsupported(0) != BlockVersion::KesVrfproof);
        assert!(AnyBlockVersion::Unsupported(0) != BlockVersion::Ed25519Signed);
        assert!(AnyBlockVersion::Unsupported(0) != BlockVersion::KesVrfproof);
    }

    #[proptest]
    fn conversion_u8(block_version: AnyBlockVersion) {
        let bytes: u8 = block_version.into();
        let new_block_version = AnyBlockVersion::from(bytes);
        println!("{:?}, {:?}", bytes, new_block_version);
        prop_assert_eq!(block_version, new_block_version);
    }

    #[proptest]
    fn from_block_version(block_version: BlockVersion) {
        let right_version = AnyBlockVersion::Supported(block_version);
        let left_version: AnyBlockVersion = block_version.into();
        prop_assert_eq!(left_version, right_version);
    }

    #[test]
    pub fn to_consensus_type() {
        assert_eq!(BlockVersion::Genesis.to_consensus_type(), None);
        assert_eq!(
            BlockVersion::Ed25519Signed.to_consensus_type(),
            Some(ConsensusType::Bft)
        );
        assert_eq!(
            BlockVersion::KesVrfproof.to_consensus_type(),
            Some(ConsensusType::GenesisPraos)
        );
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod prop_impls {
    use proptest::{arbitrary::StrategyFor, prelude::*, strategy::Map};

    use super::AnyBlockVersion;

    impl Arbitrary for AnyBlockVersion {
        type Parameters = ();
        type Strategy = Map<StrategyFor<u8>, fn(u8) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<u8>().prop_map(From::from)
        }
    }
}
