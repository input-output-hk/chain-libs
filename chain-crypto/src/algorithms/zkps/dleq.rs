//! Non-interactive Zero Knowledge proof for correct ElGamal
//! decryption. We use the notation and scheme presented in
//! Figure 5 of the Treasury voting protocol spec.
//!
//! The proof is the following:
//!
//! `NIZK{(pk, C, M), (sk): M = Dec_sk(C) AND pk = g^sk}`
//!
//! which makes the statement, the public key, `pk`, the ciphertext
//! `(e1, e2)`, and the message, `m`. The witness, on the other hand
//! is the secret key, `sk`.
#![allow(clippy::many_single_char_names)]
use super::challenge_context::ChallengeContext;
use crate::ec::{GroupElement, Scalar};
use rand_core::{CryptoRng, RngCore};

/// Proof of correct decryption.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Zkp {
    challenge: Scalar,
    response: Scalar,
}

impl Zkp {
    pub(crate) const PROOF_SIZE: usize = 2 * Scalar::BYTES_LEN;
    /// Generate a decryption zero knowledge proof
    pub fn generate<R>(base_1: &GroupElement, base_2: &GroupElement, point_1: &GroupElement, point_2: &GroupElement, dlog: &Scalar, rng: &mut R) -> Self
        where
            R: CryptoRng + RngCore,
    {
        let w = Scalar::random(rng);
        let announcement_1 = base_1 * &w;
        let announcement_2 = base_2 * &w;
        let mut challenge_context = ChallengeContext::new(base_1, base_2, point_1, point_2);
        let challenge = challenge_context.first_challenge(&announcement_1, &announcement_2);
        let response = dlog * &challenge + &w;

        Zkp { challenge, response }
    }

    /// Verify a DLEQ proof is valid
    pub fn verify(&self, base_1: &GroupElement, base_2: &GroupElement, point_1: &GroupElement, point_2: &GroupElement) -> bool {
        let r1 = base_1 * &self.response;
        let r2 = base_2 * &self.response;
        let announcement_1 = r1 - (point_1 * &self.challenge);
        let announcement_2 = r2 - (point_2 * &self.challenge);
        // no need for constant time equality because of the hash in challenge()
        let mut challenge_context = ChallengeContext::new(base_1, base_2, point_1, point_2);
        let challenge = challenge_context.first_challenge(&announcement_1, &announcement_2);
        challenge == self.challenge
    }

    pub fn to_bytes(&self) -> [u8; Self::PROOF_SIZE] {
        let mut output = [0u8; Self::PROOF_SIZE];
        self.to_mut_slice(&mut output);
        output
    }

    pub fn to_mut_slice(&self, output: &mut [u8]) {
        assert_eq!(output.len(), Self::PROOF_SIZE);
        output[0..Scalar::BYTES_LEN].copy_from_slice(&self.challenge.to_bytes());
        output[Scalar::BYTES_LEN..]
            .copy_from_slice(&self.response.to_bytes());
    }

    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != Self::PROOF_SIZE {
            return None;
        }
        let challenge = Scalar::from_bytes(&slice[..Scalar::BYTES_LEN])?;
        let response = Scalar::from_bytes(
            &slice
                [Scalar::BYTES_LEN..],
        )?;

        let proof = Zkp { challenge, response };
        Some(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    #[test]
    pub fn it_works() {
        let mut r: OsRng = OsRng;

        let dlog = Scalar::random(&mut r);
        let base_1 = GroupElement::from_hash(&[0u8]);
        let base_2 = GroupElement::from_hash(&[0u8]);
        let point_1 = &base_1 * &dlog;
        let point_2 = &base_2 * &dlog;

        let proof = Zkp::generate(
            &base_1,
            &base_2,
            &point_1,
            &point_2,
            &dlog,
            &mut r,
        );

        assert!(proof.verify(&base_1, &base_2, &point_1, &point_2));
    }

    #[test]
    fn serialisation() {
        let mut r: OsRng = OsRng;

        let dlog = Scalar::random(&mut r);
        let base_1 = GroupElement::from_hash(&[0u8]);
        let base_2 = GroupElement::from_hash(&[0u8]);
        let point_1 = &base_1 * &dlog;
        let point_2 = &base_2 * &dlog;

        let proof = Zkp::generate(
            &base_1,
            &base_2,
            &point_1,
            &point_2,
            &dlog,
            &mut r,
        );

        let serialised_proof = proof.to_bytes();
        let deserialised_proof = Zkp::from_slice(&serialised_proof);

        assert!(deserialised_proof.is_some());

        assert!(deserialised_proof.unwrap().verify(&base_1, &base_2, &point_1, &point_2));
    }
}
