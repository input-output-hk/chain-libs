//! Implementation of the Unit Vector ZK argument presented by
//! Zhang, Oliynykov and Balogum in "A Treasury System for Cryptocurrencies:
//! Enabling Better Collaborative Intelligence"
//! (https://www.ndss-symposium.org/wp-content/uploads/2019/02/ndss2019_02A-2_Zhang_paper.pdf)
//!
//! Given a common reference string formed by a pedersen commitment key,
//! the prover generates a logarithmic proof that a tuple of encryptions
//! corresponds to the element-wise encryption of some unit vector, without
//! disclosing the latter.


use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;
use rand_core::{CryptoRng, RngCore};
use merlin::Transcript;

use crate::commitment::{Commitment, CommitmentKey};
use crate::encrypted::{EncryptingVote, PTP};
use crate::gang::Scalar;
use crate::gargamel::{encrypt, Ciphertext, PublicKey};
use crate::math::Polynomial;
use crate::unit_vector::binrep;
use crate::CRS;

/// Randomness generated in the proof, used for the hiding property.
struct BlindingRandomness {
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
}

/// First announcement, formed by I, B, A commitments. These commitments
/// contain the binary representation of the unit vector index.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Announcement {
    i: Commitment,
    b: Commitment,
    a: Commitment,
}

/// Response encoding the bits of the private vector, and the randomness of `BlindingRandomness`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ResponseRandomness {
    z: Scalar,
    w: Scalar,
    v: Scalar,
}

/// Unit vector proof. In this proof, a prover encrypts each entry of a vector e, and proves
/// that the vector is a unit vector. In particular, it proves that it is the ith unit
/// vector without disclosing i.
/// Common Reference String: Pedersen Commitment Key
/// Statement: group generator g, public key pk, and ciphertexts
/// C_0=Enc_pk(r_0; v_0), ..., C_{m-1}=Enc_pk(r_{m-1}; v_{m-1})
/// Witness: the unit vector e, and randomness r_i for i in [0, m-1]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Proof {
    ibas: Vec<Announcement>,
    ds: Vec<Ciphertext>,
    zwvs: Vec<ResponseRandomness>,
    r: Scalar,
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

#[allow(clippy::len_without_is_empty)]
impl Proof {
    /// Constructs the proof structure from constituent parts.
    ///
    /// # Panics
    ///
    /// The `ibas`, `ds`, and `zwvs` must have the same length, otherwise the function will panic.
    pub fn from_parts(ibas: Vec<Announcement>, ds: Vec<Ciphertext>, zwvs: Vec<ResponseRandomness>, r: Scalar) -> Self {
        assert_eq!(ibas.len(), ds.len());
        assert_eq!(ibas.len(), zwvs.len());
        Proof { ibas, ds, zwvs, r }
    }

    /// Returns the length of the size of the witness vector
    pub fn len(&self) -> usize {
        self.ibas.len()
    }

    /// Return an iterator of the announcement commitments
    pub fn ibas(&self) -> impl Iterator<Item = &Announcement> {
        self.ibas.iter()
    }

    /// Return an iterator of the encryptions of the polynomial coefficients
    pub fn ds(&self) -> impl Iterator<Item = &Ciphertext> {
        self.ds.iter()
    }

    /// Return an iterator of the response related to the randomness
    pub fn zwvs(&self) -> impl Iterator<Item = &ResponseRandomness> {
        self.zwvs.iter()
    }

    /// Return R
    pub fn r(&self) -> &Scalar {
        &self.r
    }
}

impl Announcement {
    fn new(ck: &CommitmentKey, blinding_randomness: &BlindingRandomness, index: &Scalar) -> Self {
        assert!(index == &Scalar::zero() || index == &Scalar::one());

        // commit index bit: 0 or 1
        let i = Commitment::new(ck, &index, &blinding_randomness.alpha);
        // commit beta
        let b = Commitment::new(ck, &blinding_randomness.beta, &blinding_randomness.gamma);
        // commit i * B => 0 * B = 0 or 1 * B = B
        let acommited = if index == &Scalar::one() {
            blinding_randomness.beta.clone()
        } else {
            Scalar::zero()
        };
        let a = Commitment::new(ck, &acommited, &blinding_randomness.delta);

        Announcement { i, b, a }
    }
}

struct ChallengeContext(Blake2b);

fn hash_to_scalar(b: &Blake2b) -> Scalar {
    let mut h = [0u8; 32];
    b.clone().result(&mut h);
    Scalar::from_bytes(&h).unwrap()
}

impl ChallengeContext {
    fn new(public_key: &PublicKey, ciphers: &[Ciphertext], ibas: &[Announcement]) -> Self {
        let mut ctx = Blake2b::new(32);
        ctx.input(&public_key.to_bytes());
        for c in ciphers {
            ctx.input(&c.to_bytes());
        }
        for iba in ibas {
            ctx.input(&iba.i.to_bytes());
            ctx.input(&iba.b.to_bytes());
            ctx.input(&iba.a.to_bytes());
        }
        ChallengeContext(ctx)
    }

    fn first_challenge(&self) -> Scalar {
        hash_to_scalar(&self.0)
    }

    fn second_challenge(&self, ds: &[Ciphertext]) -> Scalar {
        let mut x = self.0.clone();
        for d in ds {
            x.input(&d.to_bytes())
        }
        hash_to_scalar(&x)
    }
}

fn generate_polys() -> Vec<Polynomial> {
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
    for i in 0..ciphers.len() {
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

    assert_eq!(pjs.len(), ciphers.len());
    pjs
}

pub(crate) fn prove<R: RngCore + CryptoRng>(
    rng: &mut R,
    crs: &CRS,
    public_key: &PublicKey,
    encrypting_vote: EncryptingVote,
) -> Proof {
    let ck = CommitmentKey { h: crs.clone() };
    let ciphers = PTP::new(encrypting_vote.ciphertexts, Ciphertext::zero);
    let cipher_randoms = PTP::new(encrypting_vote.random_elements, Scalar::zero);

    assert_eq!(ciphers.bits(), cipher_randoms.bits());

    let bits = ciphers.bits();

    let mut blinding_randomness_vec = Vec::with_capacity(bits);
    for _ in 0..bits {
        blinding_randomness_vec.push(BlindingRandomness::random(rng))
    }

    let unit_vector = &encrypting_vote.unit_vector;
    let idx_binary_rep = binrep(unit_vector.ith(), bits as u32);
    assert_eq!(idx_binary_rep.len(), bits);

    // Generate I, B, A commitments
    let first_announcement_vec: Vec<Announcement> = blinding_randomness_vec
        .iter()
        .zip(idx_binary_rep.iter())
        .map(|(abcd, index)| Announcement::new(&ck, abcd, &(*index).into()))
        .collect();
    debug_assert_eq!(first_announcement_vec.len(), bits);

    // Generate First verifier challenge
    let cc = ChallengeContext::new(public_key, ciphers.as_ref(), &first_announcement_vec);
    let cy = cc.first_challenge();

    let (ds, rs) = {
        let pjs = generate_polys();

        // Generate new Rs for Ds
        let mut rs = Vec::with_capacity(bits);
        for _ in 0..bits {
            let r = Scalar::random(rng);
            rs.push(r);
        }

        // Compute Ds
        let ds = rs
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let mut sum = Scalar::zero();
                #[allow(clippy::needless_range_loop)]
                for j in 0..ciphers.len() {
                    sum = sum + (cy.power(j) * pjs[j].get_coefficient_at(i))
                }

                encrypt(public_key, &sum, r)
            })
            .collect::<Vec<_>>();

        (ds, rs)
    };
    debug_assert_eq!(ds.len(), bits);

    // Generate second verifier challenge
    let cx = cc.second_challenge(&ds);

    // Compute ZWVs
    let zwvs = blinding_randomness_vec
        .iter()
        .zip(idx_binary_rep.iter())
        .map(|(abcd, index)| {
            let z = Scalar::from(*index) * &cx + &abcd.beta;
            let w = &abcd.alpha * &cx + &abcd.gamma;
            let v = &abcd.alpha * (&cx - &z) + &abcd.delta;
            ResponseRandomness { z, w, v }
        })
        .collect::<Vec<_>>();
    debug_assert_eq!(zwvs.len(), bits);

    // Compute R
    let r = {
        let cx_pow = cx.power(bits);
        let p1 = cipher_randoms
            .iter()
            .enumerate()
            .fold(Scalar::zero(), |acc, (i, r)| {
                let el = r * &cx_pow * cy.power(i);
                el + acc
            });
        let p2 = rs.iter().enumerate().fold(Scalar::zero(), |acc, (l, r)| {
            let el = r * cx.power(l);
            el + acc
        });
        p1 + p2
    };

    Proof { ibas: first_announcement_vec, ds, zwvs, r }
}

pub(crate) fn verify(
    crs: &CRS,
    public_key: &PublicKey,
    ciphertexts: &[Ciphertext],
    proof: &Proof,
) -> bool {
    let ck = CommitmentKey { h: crs.clone() };
    let ciphertexts = PTP::new(ciphertexts.to_vec(), Ciphertext::zero);
    let bits = ciphertexts.bits();
    let cc = ChallengeContext::new(public_key, ciphertexts.as_ref(), &proof.ibas);
    let cy = cc.first_challenge();
    let cx = cc.second_challenge(&proof.ds);

    if proof.ibas.len() != bits {
        return false;
    }

    if proof.zwvs.len() != bits {
        return false;
    }

    // check commitments are 0 / 1
    for (iba, zwv) in proof.ibas.iter().zip(proof.zwvs.iter()) {
        let com1 = Commitment::new(&ck, &zwv.z, &zwv.w);
        let lhs = &iba.i * &cx + &iba.b;
        if lhs != com1 {
            return false;
        }

        let com2 = Commitment::new(&ck, &Scalar::zero(), &zwv.v);
        let lhs = &iba.i * (&cx - &zwv.z) + &iba.a;
        if lhs != com2 {
            return false;
        }
    }

    // check product
    {
        let bits = ciphertexts.bits();
        let cx_pow = cx.power(bits);

        let p1 = ciphertexts
            .as_ref()
            .iter()
            .enumerate()
            .fold(Ciphertext::zero(), |acc, (i, c)| {
                let idx = binrep(i, bits as u32);
                let multz = proof
                    .zwvs
                    .iter()
                    .enumerate()
                    .fold(Scalar::one(), |acc, (j, zwv)| {
                        let m = if idx[j] { zwv.z.clone() } else { &cx - &zwv.z };
                        &acc * m
                    });
                let enc = encrypt(public_key, &multz.negate(), &Scalar::zero());
                let mult_c = c * &cx_pow;
                let y_pow_i = cy.power(i);
                let t = (&mult_c + &enc) * y_pow_i;
                &acc + &t
            });

        let dsum = proof
            .ds
            .iter()
            .enumerate()
            .fold(Ciphertext::zero(), |acc, (l, d)| &acc + &(d * cx.power(l)));

        let zero = encrypt(public_key, &Scalar::zero(), &proof.r);
        if &p1 + &dsum != zero {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypted::EncryptingVote;
    use crate::gargamel;
    use crate::unit_vector::UnitVector;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    #[test]
    fn prove_verify1() {
        let mut r = ChaCha20Rng::from_seed([0u8; 32]);
        let public_key = gargamel::generate(&mut r).public_key;
        let unit_vector = UnitVector::new(2, 0);
        let ev = EncryptingVote::prepare(&mut r, &public_key, &unit_vector);

        let mut shared_string =
            b"Example of a shared string. This could be the latest block hash".to_owned();
        let crs = CRS::from_hash(&mut shared_string);

        let proof = prove(&mut r, &crs, &public_key, ev.clone());
        assert!(verify(&crs, &public_key, &ev.ciphertexts, &proof))
    }

    #[test]
    fn prove_verify() {
        let mut r = ChaCha20Rng::from_seed([0u8; 32]);
        let public_key = gargamel::generate(&mut r).public_key;
        let unit_vector = UnitVector::new(5, 1);
        let ev = EncryptingVote::prepare(&mut r, &public_key, &unit_vector);

        let mut shared_string =
            b"Example of a shared string. This could be the latest block hash".to_owned();
        let crs = CRS::from_hash(&mut shared_string);

        let proof = prove(&mut r, &crs, &public_key, ev.clone());
        assert!(verify(&crs, &public_key, &ev.ciphertexts, &proof))
    }
}
