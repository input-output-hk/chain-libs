use chain_core::mempack::{ReadBuf, ReadError, Readable};
use cryptoxide::{blake2b::Blake2b, digest::Digest};
use thiserror::Error;
use typed_bytes::ByteBuilder;

use std::{convert::TryInto, fmt, str::FromStr};

pub const POLICY_HASH_SIZE: usize = 28;
pub const TOKEN_NAME_MAX_SIZE: usize = 32;

/// The unique identifier of a token.
///
/// It is represented either as two hex strings separated by a dot or just a hex string when the
/// name is empty.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TokenIdentifier {
    pub policy_hash: PolicyHash,
    pub token_name: TokenName,
}

/// blake2b_224 hash of a serialized minting policy
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyHash([u8; POLICY_HASH_SIZE]);

/// A sequence of bytes serving as a token name. Tokens that share the same name but have different
/// voting policies hashes are different tokens. A name can be empty. The maximum length of a token
/// name is 32 bytes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenName(Vec<u8>);

/// A minting policy consists of multiple entries defining different
/// constraints on the minting process. An empty policy means that new tokens
/// cannot be minted during the chain run.
///
/// Minting policies are meant to be ignored in block0 fragments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintingPolicy(Vec<MintingPolicyEntry>);

/// An entry of a minting policy. Currently there are no entries available.
/// This is reserved for the future use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MintingPolicyEntry {}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MintingPolicyViolation {
    #[error("the policy of this token does not allow minting")]
    AdditionalMintingNotAllowed,
}

#[derive(Debug, Error)]
#[error("Token name can be no more that {} bytes long; got {} bytes", TOKEN_NAME_MAX_SIZE, .actual)]
pub struct TokenNameTooLong {
    actual: usize,
}

#[derive(Debug, Error)]
pub enum TokenIdentifierParseError {
    #[error("got an empty str")]
    EmptyStr,

    #[error(transparent)]
    Hex(#[from] hex::FromHexError),

    #[error(transparent)]
    PolicyHash(#[from] ReadError),

    #[error("expected a token name after the `.`")]
    ExpectedTokenName,

    #[error(transparent)]
    TokenName(#[from] TokenNameTooLong),

    #[error("unexpected data after the token name")]
    UnexpectedData,
}

impl MintingPolicy {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn check_minting_tx(&self) -> Result<(), MintingPolicyViolation> {
        if self.0.is_empty() {
            return Err(MintingPolicyViolation::AdditionalMintingNotAllowed);
        }

        for _entry in &self.0 {
            unreachable!("implement this when we have actual minting policies");
        }

        Ok(())
    }

    pub fn entries(&self) -> &[MintingPolicyEntry] {
        &self.0
    }

    pub fn bytes(&self) -> Vec<u8> {
        let bb: ByteBuilder<Self> = ByteBuilder::new();
        bb.u8(0).finalize_as_vec()
    }

    pub fn hash(&self) -> PolicyHash {
        let mut result = [0u8; POLICY_HASH_SIZE];
        if !self.0.is_empty() {
            let mut hasher = Blake2b::new(POLICY_HASH_SIZE);
            hasher.input(&self.bytes());
            hasher.result(&mut result);
        }
        PolicyHash(result)
    }
}

impl Default for MintingPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl Readable for MintingPolicy {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let no_entries = buf.get_u8()?;
        if no_entries != 0 {
            return Err(ReadError::InvalidData(
                "non-zero number of minting policy entries, but they are currently unimplemented"
                    .to_string(),
            ));
        }
        Ok(Self::new())
    }
}

impl PolicyHash {
    pub fn bytes(&self) -> &[u8; POLICY_HASH_SIZE] {
        &self.0
    }
}

impl Readable for PolicyHash {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let bytes = buf
            .get_slice(POLICY_HASH_SIZE)?
            .try_into()
            .expect(&format!("already read {} bytes", POLICY_HASH_SIZE));
        Ok(Self(bytes))
    }
}

impl TokenName {
    pub fn try_from_bytes(b: Vec<u8>) -> Result<Self, TokenNameTooLong> {
        if b.len() > TOKEN_NAME_MAX_SIZE {
            return Err(TokenNameTooLong { actual: b.len() });
        }
        Ok(Self(b))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Readable for TokenName {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let name_length = buf.get_u8()? as usize;
        if name_length > TOKEN_NAME_MAX_SIZE {
            return Err(ReadError::SizeTooBig(TOKEN_NAME_MAX_SIZE, name_length));
        }
        let bytes = buf.get_slice(name_length)?.into();
        Ok(Self(bytes))
    }
}

impl TokenIdentifier {
    pub fn bytes(&self) -> Vec<u8> {
        let bb: ByteBuilder<Self> = ByteBuilder::new();
        let token_name = self.token_name.bytes();
        bb.bytes(self.policy_hash.bytes())
            .u8(token_name.len() as u8)
            .bytes(token_name)
            .finalize_as_vec()
    }
}

impl Readable for TokenIdentifier {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let policy_hash = PolicyHash::read(buf)?;
        let token_name = TokenName::read(buf)?;
        Ok(Self {
            policy_hash,
            token_name,
        })
    }
}

impl fmt::Display for TokenIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.policy_hash.bytes()))?;
        let token_name = self.token_name.bytes();
        if !token_name.is_empty() {
            write!(f, ".{}", hex::encode(token_name))?;
        }
        Ok(())
    }
}

impl FromStr for TokenIdentifier {
    type Err = TokenIdentifierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(".");

        let policy_hash = {
            let hex = parts.next().ok_or(TokenIdentifierParseError::EmptyStr)?;
            let bytes = hex::decode(hex)?;
            PolicyHash::read(&mut ReadBuf::from(&bytes))?
        };

        let token_name = {
            let bytes = if let Some(hex) = parts.next() {
                hex::decode(hex)?
            } else {
                Vec::new()
            };
            TokenName::try_from_bytes(bytes)?
        };

        if parts.next().is_some() {
            return Err(TokenIdentifierParseError::UnexpectedData);
        }

        Ok(TokenIdentifier {
            policy_hash,
            token_name,
        })
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

    impl Arbitrary for TokenName {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let len = usize::arbitrary(g) % 33;
            let mut bytes = Vec::with_capacity(len);
            for _ in 0..len {
                bytes.push(Arbitrary::arbitrary(g));
            }
            Self(bytes)
        }
    }

    impl Arbitrary for TokenIdentifier {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let policy_hash = Arbitrary::arbitrary(g);
            let token_name = Arbitrary::arbitrary(g);
            Self {
                policy_hash,
                token_name,
            }
        }
    }

    impl Arbitrary for MintingPolicy {
        fn arbitrary<G: Gen>(_g: &mut G) -> Self {
            Self::new()
        }
    }

    #[quickcheck_macros::quickcheck]
    fn token_identifier_display_sanity(id: TokenIdentifier) {
        let s = id.to_string();
        let id_: TokenIdentifier = s.parse().unwrap();
        assert_eq!(id, id_);
    }
}
