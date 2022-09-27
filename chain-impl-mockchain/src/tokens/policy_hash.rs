use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError},
};

pub const POLICY_HASH_SIZE: usize = 28;

/// blake2b_224 hash of a serialized minting policy
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    any(test, feature = "property-test-api"),
    derive(test_strategy::Arbitrary)
)]
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
        Self::deserialize(&mut Codec::new(value))
    }
}

impl Deserialize for PolicyHash {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
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
    // proptest macro bug
    #[allow(unused_imports)]
    use proptest::prop_assert_eq;
    #[allow(unused_imports)]
    use quickcheck::TestResult;
    use quickcheck::{Arbitrary, Gen};
    use test_strategy::proptest;

    impl Arbitrary for PolicyHash {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; POLICY_HASH_SIZE];
            for i in &mut bytes {
                *i = Arbitrary::arbitrary(g);
            }
            Self(bytes)
        }
    }

    #[proptest]
    // `proptest` attr macro doesn't keep span info properly, so rustc can't see that `ph` is
    // actually used
    fn policy_hash_serialization_bijection(#[allow(dead_code)] ph: PolicyHash) {
        let ph_got = ph.as_ref();
        let mut codec = Codec::new(ph_got);
        let result = PolicyHash::deserialize(&mut codec).unwrap();
        prop_assert_eq!(ph, result);
    }
}
