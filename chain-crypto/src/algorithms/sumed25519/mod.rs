#[cfg(test)]
mod sumrec;

#[cfg(not(feature = "with-bench"))]
mod common;
#[cfg(not(feature = "with-bench"))]
mod sum;

#[cfg(feature = "with-bench")]
pub mod common;
#[cfg(feature = "with-bench")]
pub mod sum;

use crate::evolving::{EvolvingStatus, KeyEvolvingAlgorithm};
use crate::kes::KeyEvolvingSignatureAlgorithm;
use crate::key::{AsymmetricKey, AsymmetricPublicKey, PublicKeyError, SecretKeyError};
use crate::sign::{SignatureError, SigningAlgorithm, Verification, VerificationAlgorithm};
use rand_core::{CryptoRng, RngCore};

// MMM sum scheme instanciated over the Ed25519 signature system
// and a specific depth of 12
pub struct SumEd25519_12;

const DEPTH: common::Depth = common::Depth(12);

impl AsymmetricPublicKey for SumEd25519_12 {
    type Public = sum::PublicKey;
    const PUBLIC_BECH32_HRP: &'static str = "kes25519-12-pk";
    const PUBLIC_KEY_SIZE: usize = 32;
    fn public_from_binary(data: &[u8]) -> Result<Self::Public, PublicKeyError> {
        sum::PublicKey::from_bytes(data).map_err(|e| match e {
            sum::Error::InvalidPublicKeySize(_) => PublicKeyError::SizeInvalid,
            _ => PublicKeyError::StructureInvalid,
        })
    }
}

impl AsymmetricKey for SumEd25519_12 {
    type Secret = sum::SecretKey;
    type PubAlg = SumEd25519_12;

    const SECRET_BECH32_HRP: &'static str = "kes25519-12-sk";
    fn generate<T: RngCore + CryptoRng>(mut rng: T) -> Self::Secret {
        let mut priv_bytes = [0u8; common::Seed::SIZE];
        rng.fill_bytes(&mut priv_bytes);

        let seed = common::Seed::from_bytes(priv_bytes);

        let (sk, _) = sum::keygen(DEPTH, &seed);
        sk
    }

    fn compute_public(key: &Self::Secret) -> sum::PublicKey {
        key.compute_public()
    }

    fn secret_from_binary(data: &[u8]) -> Result<Self::Secret, SecretKeyError> {
        sum::SecretKey::from_bytes(DEPTH, data).map_err(|e| match e {
            sum::Error::InvalidSecretKeySize(_) => SecretKeyError::SizeInvalid,
            _ => SecretKeyError::StructureInvalid,
        })
    }
}

impl VerificationAlgorithm for SumEd25519_12 {
    type Signature = sum::Signature;

    const SIGNATURE_SIZE: usize = sum::signature_size(DEPTH);
    const SIGNATURE_BECH32_HRP: &'static str = "kes25519-12-sig";

    fn signature_from_bytes(data: &[u8]) -> Result<Self::Signature, SignatureError> {
        sum::Signature::from_bytes(DEPTH, data).map_err(|e| match e {
            sum::Error::InvalidSignatureSize(_) => SignatureError::SizeInvalid {
                expected: Self::SIGNATURE_SIZE,
                got: data.len(),
            },
            _ => SignatureError::StructureInvalid,
        })
    }

    fn verify_bytes(
        pubkey: &Self::Public,
        signature: &Self::Signature,
        msg: &[u8],
    ) -> Verification {
        if sum::verify(pubkey, msg, signature) {
            Verification::Success
        } else {
            Verification::Failed
        }
    }
}

impl SigningAlgorithm for SumEd25519_12 {
    fn sign(key: &Self::Secret, msg: &[u8]) -> sum::Signature {
        sum::sign(key, msg)
    }
}

impl KeyEvolvingAlgorithm for SumEd25519_12 {
    fn get_period(sec: &Self::Secret) -> u32 {
        sec.t() as u32
    }
    fn update(key: &mut Self::Secret) -> EvolvingStatus {
        if sum::update(key).is_ok() {
            EvolvingStatus::Success
        } else {
            EvolvingStatus::Failed
        }
    }
}

impl KeyEvolvingSignatureAlgorithm for SumEd25519_12 {
    fn get_period(sig: &Self::Signature) -> u32 {
        sig.t() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[quickcheck]
    /// `public_from_binary`should fail if the provided byte array does not match the public key size
    fn public_from_binary_size_check(n: usize) {
        let public_key = SumEd25519_12::public_from_binary(&vec![0; n]);

        assert_eq!(
            n != SumEd25519_12::PUBLIC_KEY_SIZE,
            public_key == Err(PublicKeyError::SizeInvalid)
        );
    }

    #[quickcheck]
    /// `signature_from_bytes` should fail if the provided byte array does not match the public key size
    fn signature_from_bytes_size_check(n: usize) {
        let verification_algorithm = SumEd25519_12::signature_from_bytes(&vec![0; n]);

        assert_eq!(
            n != SumEd25519_12::SIGNATURE_SIZE,
            matches!(
                verification_algorithm,
                Err(SignatureError::SizeInvalid { .. })
            )
        );
    }
}
