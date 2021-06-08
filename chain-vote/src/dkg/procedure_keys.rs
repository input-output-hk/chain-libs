use crate::encryption::{PublicKey, SecretKey, HybridCiphertext};
use crate::gang::{GroupElement, Scalar};
use rand_core::{CryptoRng, RngCore};
use super::committee::IndexedEncryptedShares;

/// Committee member election secret key
#[derive(Clone)]
pub struct MemberSecretKey(pub(crate) SecretKey);

/// Committee member election public key
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MemberPublicKey(pub(crate) PublicKey);

/// Committee member communication private key
#[derive(Clone)]
pub struct MemberCommunicationKey(SecretKey);

/// Committee Member communication public key (with other committee members)
#[derive(Clone)]
pub struct MemberCommunicationPublicKey(PublicKey);

/// The overall committee public key used for everyone to encrypt their vote to.
#[derive(Clone)]
pub struct ElectionPublicKey(pub(crate) PublicKey);

impl MemberSecretKey {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.sk.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let sk = Scalar::from_bytes(bytes)?;
        Some(Self(SecretKey { sk }))
    }
}

impl MemberPublicKey {
    pub const BYTES_LEN: usize = PublicKey::BYTES_LEN;

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        Some(Self(PublicKey::from_bytes(buf)?))
    }
}

impl From<PublicKey> for MemberPublicKey {
    fn from(pk: PublicKey) -> MemberPublicKey {
        MemberPublicKey(pk)
    }
}

impl MemberCommunicationKey {
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let sk = SecretKey::generate(rng);
        MemberCommunicationKey(sk)
    }

    pub fn to_public(&self) -> MemberCommunicationPublicKey {
        MemberCommunicationPublicKey(PublicKey {
            pk: &GroupElement::generator() * &self.0.sk,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<MemberCommunicationKey> {
        SecretKey::from_bytes(bytes).map(MemberCommunicationKey)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.sk.to_bytes()
    }

    pub fn hybrid_decrypt(&self, ciphertext: &HybridCiphertext) -> Vec<u8> {
        self.0.hybrid_decrypt(ciphertext)
    }

    pub(crate) fn decrypt_shares(&self, shares: IndexedEncryptedShares) -> (Option<Scalar>, Option<Scalar>) {
        let comm_scalar = Scalar::from_bytes(
            &self.hybrid_decrypt(&shares.1));
        let shek_scalar = Scalar::from_bytes(
            &self.hybrid_decrypt(&shares.2));

        (comm_scalar, shek_scalar)
    }
}

impl MemberCommunicationPublicKey {
    pub fn from_public_key(pk: PublicKey) -> Self {
        Self(pk)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        PublicKey::from_bytes(bytes).map(Self)
    }

    pub fn hybrid_encrypt<R>(&self, message: &[u8], rng: &mut R) -> HybridCiphertext
        where
            R: RngCore + CryptoRng,
    {
        self.0.hybrid_encrypt(message, rng)
    }
}

impl ElectionPublicKey {
    /// Create an election public key from all the participants of this committee
    pub fn from_participants(pks: &[MemberPublicKey]) -> Self {
        let mut k = pks[0].0.pk.clone();
        for pk in &pks[1..] {
            k = k + &pk.0.pk;
        }
        ElectionPublicKey(PublicKey { pk: k })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        PublicKey::from_bytes(buf).map(ElectionPublicKey)
    }

    #[doc(hidden)]
    pub fn as_raw(&self) -> &PublicKey {
        &self.0
    }
}
