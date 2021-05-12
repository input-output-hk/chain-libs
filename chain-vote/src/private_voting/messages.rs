//! Structures sent by the prover to the verifier.
//! todo: describe
//!

use crate::commitment::{Commitment, CommitmentKey};
use crate::math::Polynomial;
use crate::unit_vector::binrep;
use crate::Scalar;
use rand_core::{CryptoRng, RngCore};

/// Randomness generated in the proof, used for the hiding property.
pub struct BlindingRandomness {
    alpha: Scalar,
    beta: Scalar,
    gamma: Scalar,
    delta: Scalar,
}

impl BlindingRandomness {
    /// Generate randomness
    pub fn random<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        let alpha = Scalar::random(rng);
        let beta = Scalar::random(rng);
        let gamma = Scalar::random(rng);
        let delta = Scalar::random(rng);
        BlindingRandomness {
            alpha,
            beta,
            gamma,
            delta,
        }
    }

    /// Generate a response randomness from the `BlindingRandomness`, and a `challenge` and `index` given as
    /// input.
    pub(crate) fn gen_response(&self, challenge: &Scalar, index: &bool) -> ResponseRandomness {
        let z = Scalar::from(*index) * challenge + &self.beta;
        let w = &self.alpha * challenge + &self.gamma;
        let v = &self.alpha * (challenge - &z) + &self.delta;
        ResponseRandomness { z, w, v }
    }
}

/// First announcement, formed by I, B, A commitments. These commitments
/// contain the binary representation of the unit vector index.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Announcement {
    pub(crate) i: Commitment,
    pub(crate) b: Commitment,
    pub(crate) a: Commitment,
}

impl Announcement {
    pub const BYTES_LEN: usize = Commitment::BYTES_LEN * 3;

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::BYTES_LEN {
            return None;
        }
        Some(Self {
            i: Commitment::from_bytes(&bytes[0..Commitment::BYTES_LEN])?,
            b: Commitment::from_bytes(&bytes[Commitment::BYTES_LEN..Commitment::BYTES_LEN * 2])?,
            a: Commitment::from_bytes(&bytes[Commitment::BYTES_LEN * 2..])?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::BYTES_LEN);
        for component in [&self.i, &self.b, &self.a].iter() {
            buf.extend_from_slice(&component.to_bytes());
        }
        debug_assert_eq!(buf.len(), Self::BYTES_LEN);
        buf
    }

    pub(crate) fn new(
        ck: &CommitmentKey,
        blinding_randomness: &BlindingRandomness,
        index: &Scalar,
    ) -> Self {
        assert!(index == &Scalar::zero() || index == &Scalar::one());

        // commit index bit: 0 or 1
        let i = ck.commit(&index, &blinding_randomness.alpha);
        // commit beta
        let b = ck.commit(&blinding_randomness.beta, &blinding_randomness.gamma);
        // commit i * B => 0 * B = 0 or 1 * B = B
        let a = if index == &Scalar::one() {
            ck.commit(
                &blinding_randomness.beta.clone(),
                &blinding_randomness.delta,
            )
        } else {
            ck.commit(&Scalar::zero(), &blinding_randomness.delta)
        };

        Announcement { i, b, a }
    }
}

/// Response encoding the bits of the private vector, and the randomness of `BlindingRandomness`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ResponseRandomness {
    pub(crate) z: Scalar,
    pub(crate) w: Scalar,
    pub(crate) v: Scalar,
}

impl ResponseRandomness {
    pub const BYTES_LEN: usize = Scalar::BYTES_LEN * 3;

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::BYTES_LEN {
            return None;
        }
        Some(Self {
            z: Scalar::from_bytes(&bytes[0..Scalar::BYTES_LEN])?,
            w: Scalar::from_bytes(&bytes[Scalar::BYTES_LEN..Scalar::BYTES_LEN * 2])?,
            v: Scalar::from_bytes(&bytes[Scalar::BYTES_LEN * 2..])?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(Self::BYTES_LEN);
        for component in [&self.z, &self.w, &self.v].iter() {
            buf.extend_from_slice(&component.to_bytes());
        }
        debug_assert_eq!(buf.len(), Self::BYTES_LEN);
        buf
    }
}

pub(crate) fn generate_polys(
    ciphers_len: usize,
    idx_binary_rep: &[bool],
    bits: usize,
    blinding_randomness_vec: &[BlindingRandomness],
) -> Vec<Polynomial> {
    // Compute polynomials pj(x)
    let polys = idx_binary_rep
        .iter()
        .zip(blinding_randomness_vec.iter())
        .map(|(ix, abcd)| {
            let z1 = Polynomial::new(bits).set2(abcd.beta.clone(), (*ix).into());
            let z0 = Polynomial::new(bits).set2(abcd.beta.negate(), (!ix).into());
            (z0, z1)
        })
        .collect::<Vec<_>>();

    let mut pjs = Vec::new();
    for i in 0..ciphers_len {
        let j = binrep(i, bits as u32);

        let mut acc = if j[0] {
            polys[0].1.clone()
        } else {
            polys[0].0.clone()
        };
        for k in 1..bits {
            let t = if j[k] {
                polys[k].1.clone()
            } else {
                polys[k].0.clone()
            };
            acc = acc * t;
        }
        pjs.push(acc)
    }

    assert_eq!(pjs.len(), ciphers_len);
    pjs
}
