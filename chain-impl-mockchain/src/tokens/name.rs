use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, WriteError},
};
use std::convert::TryFrom;
use thiserror::Error;

pub const TOKEN_NAME_MAX_SIZE: usize = 32;

/// A sequence of bytes serving as a token name. Tokens that share the same name but have different
/// voting policies hashes are different tokens. A name can be empty. The maximum length of a token
/// name is 32 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenName(Vec<u8>);

#[derive(Debug, Error)]
#[error("Token name can be no more that {} bytes long; got {} bytes", TOKEN_NAME_MAX_SIZE, .actual)]
pub struct TokenNameTooLong {
    actual: usize,
}

impl AsRef<[u8]> for TokenName {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl TryFrom<Vec<u8>> for TokenName {
    type Error = TokenNameTooLong;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        if value.len() > TOKEN_NAME_MAX_SIZE {
            return Err(TokenNameTooLong {
                actual: value.len(),
            });
        }
        Ok(Self(value))
    }
}

impl Serialize for TokenName {
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), WriteError> {
        let mut codec = Codec::new(writer);
        codec.put_u8(self.0.len() as u8)?;
        codec.put_bytes(self.0.as_slice())
    }
}

impl Deserialize for TokenName {
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, ReadError> {
        let mut codec = Codec::new(reader);
        let name_length = codec.get_u8()? as usize;
        if name_length > TOKEN_NAME_MAX_SIZE {
            return Err(ReadError::SizeTooBig(TOKEN_NAME_MAX_SIZE, name_length));
        }
        let bytes = codec.get_bytes(name_length)?;
        Ok(Self(bytes))
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod tests {
    use super::*;

    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for TokenName {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let len = usize::arbitrary(g) % (TOKEN_NAME_MAX_SIZE + 1);
            let mut bytes = Vec::with_capacity(len);
            for _ in 0..len {
                bytes.push(Arbitrary::arbitrary(g));
            }
            Self(bytes)
        }
    }
}
