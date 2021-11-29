use std::convert::{TryFrom, TryInto};

use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError},
};

pub const POLICY_HASH_SIZE: usize = 28;

/// blake2b_224 hash of a serialized minting policy
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyHash([u8; POLICY_HASH_SIZE]);

impl AsRef<[u8]> for PolicyHash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<[u8; POLICY_HASH_SIZE]> for PolicyHash {
    fn from(bytes: [u8; POLICY_HASH_SIZE]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<&[u8]> for PolicyHash {
    type Error = ReadError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::deserialize(value)
    }
}

impl Deserialize for PolicyHash {
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, ReadError> {
        let mut codec = Codec::new(reader);

        let bytes = codec
            .get_bytes(POLICY_HASH_SIZE)?
            .try_into()
            .unwrap_or_else(|_| panic!("already read {} bytes", POLICY_HASH_SIZE));
        Ok(Self(bytes))
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for PolicyHash {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; POLICY_HASH_SIZE];
            for i in &mut bytes {
                *i = Arbitrary::arbitrary(g);
            }
            Self(bytes)
        }
    }
}
