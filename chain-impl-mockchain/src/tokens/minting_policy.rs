use crate::account::AccountAlg;
use crate::key::{deserialize_public_key, serialize_public_key};
use crate::tokens::policy_hash::{PolicyHash, POLICY_HASH_SIZE};
use crate::transaction::{Input, InputEnum};
#[cfg(any(test, feature = "property-test-api"))]
use proptest::prelude::*;

use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, WriteError},
};
use chain_crypto::PublicKey;
use cryptoxide::{blake2b::Blake2b, digest::Digest};
use thiserror::Error;
use typed_bytes::ByteBuilder;

/// A minting policy consists of multiple entries defining different
/// constraints on the minting process. An empty policy means that new tokens
/// cannot be minted during the chain run.
///
/// Policies are OR-composed: a minting transaction is authorized if at least
/// one entry is satisfied.
///
/// Minting policies are meant to be ignored in block0 fragments.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "property-test-api"),
    derive(test_strategy::Arbitrary)
)]
pub struct MintingPolicy(
    #[cfg_attr(
        any(test, feature = "property-test-api"),
        strategy(Just(vec![]))
    )]
    Vec<MintingPolicyEntry>,
);

const AUTHORITY_KEY_TAG: u8 = 1;

/// An entry of a minting policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MintingPolicyEntry {
    /// The wrapping transaction must have at least one account input whose
    /// identifier matches the carried public key.
    AuthorityKey(PublicKey<AccountAlg>),
}

/// Error while checking a minting transaction against the current system state.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MintingPolicyViolation {
    #[error("the policy of this token does not allow minting")]
    AdditionalMintingNotAllowed,
    #[error("no policy entry was satisfied by the minting transaction inputs")]
    UnauthorizedMinter,
}

impl MintingPolicy {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Construct a policy that authorizes minting by holders of the given
    /// account key. The wrapping transaction must carry an Account input
    /// whose identifier matches this key.
    pub fn with_authority(key: PublicKey<AccountAlg>) -> Self {
        Self(vec![MintingPolicyEntry::AuthorityKey(key)])
    }

    /// Check whether a minting transaction satisfies this policy.
    ///
    /// Inputs are scanned for account identifiers matching any of the
    /// policy's `AuthorityKey` entries (OR semantics across entries).
    /// Witness verification for those inputs is assumed to have already
    /// been performed by the surrounding transaction-application code.
    pub fn check_minting_tx<I>(&self, inputs: I) -> Result<(), MintingPolicyViolation>
    where
        I: IntoIterator<Item = Input>,
    {
        if self.0.is_empty() {
            return Err(MintingPolicyViolation::AdditionalMintingNotAllowed);
        }

        let input_account_keys: Vec<PublicKey<AccountAlg>> = inputs
            .into_iter()
            .filter_map(|input| match input.to_enum() {
                InputEnum::AccountInput(uid, _) => uid.to_single_account().map(|id| id.into()),
                InputEnum::UtxoInput(_) => None,
            })
            .collect();

        for entry in &self.0 {
            match entry {
                MintingPolicyEntry::AuthorityKey(pk) => {
                    if input_account_keys.contains(pk) {
                        return Ok(());
                    }
                }
            }
        }

        Err(MintingPolicyViolation::UnauthorizedMinter)
    }

    pub fn entries(&self) -> &[MintingPolicyEntry] {
        &self.0
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut bb: ByteBuilder<Self> = ByteBuilder::new();
        bb = bb.u8(self.0.len() as u8);
        for entry in &self.0 {
            match entry {
                MintingPolicyEntry::AuthorityKey(pk) => {
                    bb = bb.u8(AUTHORITY_KEY_TAG).bytes(pk.as_ref());
                }
            }
        }
        bb.finalize_as_vec()
    }

    pub fn hash(&self) -> PolicyHash {
        let mut result = [0u8; POLICY_HASH_SIZE];
        if !self.0.is_empty() {
            let mut hasher = Blake2b::new(POLICY_HASH_SIZE);
            hasher.input(&self.bytes());
            hasher.result(&mut result);
        }
        result.into()
    }
}

impl Default for MintingPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for MintingPolicy {
    fn serialized_size(&self) -> usize {
        let mut size = Codec::u8_size();
        for entry in &self.0 {
            size += Codec::u8_size();
            match entry {
                MintingPolicyEntry::AuthorityKey(pk) => {
                    size += pk.as_ref().len();
                }
            }
        }
        size
    }

    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), WriteError> {
        codec.put_u8(self.0.len() as u8)?;
        for entry in &self.0 {
            match entry {
                MintingPolicyEntry::AuthorityKey(pk) => {
                    codec.put_u8(AUTHORITY_KEY_TAG)?;
                    serialize_public_key(pk, codec)?;
                }
            }
        }
        Ok(())
    }
}

impl Deserialize for MintingPolicy {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let no_entries = codec.get_u8()?;
        let mut entries = Vec::with_capacity(no_entries as usize);
        for _ in 0..no_entries {
            let tag = codec.get_u8()?;
            match tag {
                AUTHORITY_KEY_TAG => {
                    let bytes = codec.get_bytes(<AccountAlg as chain_crypto::AsymmetricPublicKey>::PUBLIC_KEY_SIZE)?;
                    let mut slice_codec = Codec::new(bytes.as_slice());
                    let pk = deserialize_public_key::<AccountAlg>(&mut slice_codec)?;
                    entries.push(MintingPolicyEntry::AuthorityKey(pk));
                }
                other => {
                    return Err(ReadError::InvalidData(format!(
                        "unknown minting policy entry tag: {}",
                        other
                    )));
                }
            }
        }
        Ok(Self(entries))
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod tests {
    use super::*;
    #[cfg(test)]
    use crate::testing::serialization::serialization_bijection;
    use chain_crypto::{Ed25519, KeyPair};
    #[cfg(test)]
    use quickcheck::TestResult;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for MintingPolicy {
        fn arbitrary<G: Gen>(_g: &mut G) -> Self {
            Self::new()
        }
    }

    quickcheck! {
        fn minting_policy_serialization_bijection(policy: MintingPolicy) -> TestResult {
            serialization_bijection(policy)
        }
    }

    #[cfg(test)]
    fn fresh_pubkey(seed: u64) -> PublicKey<AccountAlg> {
        use rand::SeedableRng;
        let mut rng = rand_chacha::ChaChaRng::seed_from_u64(seed);
        let kp: KeyPair<Ed25519> = KeyPair::generate(&mut rng);
        kp.into_keys().1
    }

    #[cfg(test)]
    fn account_input(pk: &PublicKey<AccountAlg>) -> Input {
        Input::from_account_single(pk.clone().into(), crate::value::Value(1))
    }

    #[test]
    fn empty_policy_rejects_runtime_mint() {
        let policy = MintingPolicy::new();
        let pk = fresh_pubkey(1);
        let inputs = vec![account_input(&pk)];
        assert_eq!(
            policy.check_minting_tx(inputs),
            Err(MintingPolicyViolation::AdditionalMintingNotAllowed)
        );
    }

    #[test]
    fn authority_key_match_authorizes_mint() {
        let pk = fresh_pubkey(2);
        let policy = MintingPolicy::with_authority(pk.clone());
        let inputs = vec![account_input(&pk)];
        assert_eq!(policy.check_minting_tx(inputs), Ok(()));
    }

    #[test]
    fn authority_key_mismatch_rejects_mint() {
        let authorized = fresh_pubkey(3);
        let intruder = fresh_pubkey(4);
        let policy = MintingPolicy::with_authority(authorized);
        let inputs = vec![account_input(&intruder)];
        assert_eq!(
            policy.check_minting_tx(inputs),
            Err(MintingPolicyViolation::UnauthorizedMinter)
        );
    }

    #[test]
    fn authority_key_match_among_multiple_inputs() {
        let authorized = fresh_pubkey(5);
        let other = fresh_pubkey(6);
        let policy = MintingPolicy::with_authority(authorized.clone());
        let inputs = vec![account_input(&other), account_input(&authorized)];
        assert_eq!(policy.check_minting_tx(inputs), Ok(()));
    }

    #[test]
    fn or_semantics_across_entries() {
        let a = fresh_pubkey(7);
        let b = fresh_pubkey(8);
        let policy = MintingPolicy(vec![
            MintingPolicyEntry::AuthorityKey(a),
            MintingPolicyEntry::AuthorityKey(b.clone()),
        ]);
        let inputs = vec![account_input(&b)];
        assert_eq!(policy.check_minting_tx(inputs), Ok(()));
    }

    #[test]
    fn empty_policy_hash_is_zero() {
        let policy = MintingPolicy::new();
        assert_eq!(policy.hash().as_ref(), &[0u8; POLICY_HASH_SIZE]);
    }

    #[test]
    fn authority_policy_hash_is_nonzero_and_stable() {
        let pk = fresh_pubkey(9);
        let policy = MintingPolicy::with_authority(pk);
        let h1 = policy.hash();
        let h2 = policy.hash();
        assert_eq!(h1, h2);
        assert_ne!(h1.as_ref(), &[0u8; POLICY_HASH_SIZE]);
    }

    #[test]
    fn codec_roundtrip_authority_policy() {
        let pk = fresh_pubkey(10);
        let policy = MintingPolicy::with_authority(pk);
        let mut buf = Vec::new();
        let mut wcodec = Codec::new(&mut buf);
        policy.serialize(&mut wcodec).unwrap();
        let mut rcodec = Codec::new(buf.as_slice());
        let decoded = MintingPolicy::deserialize(&mut rcodec).unwrap();
        assert_eq!(policy, decoded);
    }

    #[test]
    fn codec_roundtrip_empty_policy_unchanged() {
        let policy = MintingPolicy::new();
        let mut buf = Vec::new();
        let mut wcodec = Codec::new(&mut buf);
        policy.serialize(&mut wcodec).unwrap();
        assert_eq!(buf, vec![0u8]);
    }
}
