//! Implementation of the Unit Vector ZK argument presented by
//! Zhang, Oliynykov and Balogum in "A Treasury System for Cryptocurrencies:
//! Enabling Better Collaborative Intelligence"
//! (https://www.ndss-symposium.org/wp-content/uploads/2019/02/ndss2019_02A-2_Zhang_paper.pdf)
//!
//! Given a common reference string formed by a pedersen commitment key,
//! the prover generates a logarithmic proof that a tuple of encryptions
//! corresponds to the element-wise encryption of some unit vector, without
//! disclosing the latter.

use rand_core::{CryptoRng, RngCore};

use crate::commitment::{Commitment, CommitmentKey};
use crate::encrypted::{EncryptingVote, PTP};
use crate::encryption::{Ciphertext, PublicKey};
use crate::gang::Scalar;
use crate::math::Polynomial;
use crate::private_voting::ChallengeContext;
use crate::unit_vector::binrep;
use crate::CRS;

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

#[allow(clippy::len_without_is_empty)]
impl Proof {

    pub(crate) fn prove<R: RngCore + CryptoRng>(
        rng: &mut R,
        crs: &CRS,
        public_key: &PublicKey,
        encrypting_vote: EncryptingVote,
    ) -> Self {
        let ck = CommitmentKey::from(crs.clone());
        let ciphers = PTP::new(encrypting_vote.ciphertexts, Ciphertext::zero);
        let cipher_randoms = PTP::new(encrypting_vote.random_elements, Scalar::zero);

        assert_eq!(ciphers.bits(), cipher_randoms.bits());

        let bits = ciphers.bits();

        let mut blinding_randomness_vec = Vec::with_capacity(bits);
        for _ in 0..bits {
            blinding_randomness_vec.push(BlindingRandomness::random(rng))
        }

        let idx_binary_rep = binrep(encrypting_vote.unit_vector.ith(), bits as u32);

        // Generate I, B, A commitments
        let first_announcement_vec: Vec<Announcement> = blinding_randomness_vec
            .iter()
            .zip(idx_binary_rep.iter())
            .map(|(abcd, index)| Announcement::new(&ck, abcd, &(*index).into()))
            .collect();

        // Generate First verifier challenge
        let mut cc = ChallengeContext::new(&ck, public_key, ciphers.as_ref());
        let cy = cc.first_challenge(&first_announcement_vec);

        let (ds, rs) = {
            let pjs = generate_polys(
                ciphers.len(),
                &idx_binary_rep,
                bits,
                &blinding_randomness_vec,
            );

            // Generate new Rs for Ds
            let mut rs = Vec::with_capacity(bits);
            let mut ds = Vec::with_capacity(bits);

            for i in 0..bits {
                let mut sum = Scalar::zero();
                #[allow(clippy::needless_range_loop)]
                for j in 0..ciphers.len() {
                    sum = sum + (cy.power(j) * pjs[j].get_coefficient_at(i))
                }

                let (d, r) = public_key.encrypt_return_r(&sum, rng);
                ds.push(d);
                rs.push(r);
            }
            (ds, rs)
        };

        // Generate second verifier challenge
        let cx = cc.second_challenge(&ds);

        // Compute ZWVs
        let randomness_response_vec = blinding_randomness_vec
            .iter()
            .zip(idx_binary_rep.iter())
            .map(|(abcd, index)| abcd.gen_response(&cx, index))
            .collect::<Vec<_>>();

        // Compute R
        let r = Self::compute_response(cx, cy, &rs, cipher_randoms);

        Proof {
            ibas: first_announcement_vec,
            ds,
            zwvs: randomness_response_vec,
            r,
        }
    }

    /// Computes the final response
    /// todo: detail
    fn compute_response(
        first_challenge: Scalar,
        second_challenge: Scalar,
        rs: &[Scalar],
        cipher_randoms: PTP<Scalar>,
    ) -> Scalar {
        let cx_pow = first_challenge.power(cipher_randoms.bits());
        let p1 = cipher_randoms
            .iter()
            .enumerate()
            .fold(Scalar::zero(), |acc, (i, r)| {
                let el = r * &cx_pow * second_challenge.power(i);
                el + acc
            });
        let p2 = rs.iter().enumerate().fold(Scalar::zero(), |acc, (l, r)| {
            let el = r * first_challenge.power(l);
            el + acc
        });
        p1 + p2
    }

    pub(crate) fn verify(
        &self,
        crs: &CRS,
        public_key: &PublicKey,
        ciphertexts: &[Ciphertext],
    ) -> bool {
        let ck = CommitmentKey::from(crs.clone());
        let ciphertexts = PTP::new(ciphertexts.to_vec(), Ciphertext::zero);
        let bits = ciphertexts.bits();
        let mut cc = ChallengeContext::new(&ck, public_key, ciphertexts.as_ref());
        let cy = cc.first_challenge(&self.ibas);
        let cx = cc.second_challenge(&self.ds);

        if self.ibas.len() != bits {
            return false;
        }

        if self.zwvs.len() != bits {
            return false;
        }

        // check commitments are 0 / 1
        for (iba, zwv) in self.ibas.iter().zip(self.zwvs.iter()) {
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
                    let multz = self
                        .zwvs
                        .iter()
                        .enumerate()
                        .fold(Scalar::one(), |acc, (j, zwv)| {
                            let m = if idx[j] { zwv.z.clone() } else { &cx - &zwv.z };
                            &acc * m
                        });
                    let enc = public_key.encrypt_with_r(&multz.negate(), &Scalar::zero());
                    let mult_c = c * &cx_pow;
                    let y_pow_i = cy.power(i);
                    let t = (&mult_c + &enc) * y_pow_i;
                    &acc + &t
                });

            let dsum = self
                .ds
                .iter()
                .enumerate()
                .fold(Ciphertext::zero(), |acc, (l, d)| &acc + &(d * cx.power(l)));

            let zero = public_key.encrypt_with_r(&Scalar::zero(), &self.r);
            if &p1 + &dsum != zero {
                return false;
            }
        }

        true
    }

    /// Constructs the proof structure from constituent parts.
    ///
    /// # Panics
    ///
    /// The `ibas`, `ds`, and `zwvs` must have the same length, otherwise the function will panic.
    pub fn from_parts(
        ibas: Vec<Announcement>,
        ds: Vec<Ciphertext>,
        zwvs: Vec<ResponseRandomness>,
        r: Scalar,
    ) -> Self {
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

    /// Generate a response randomness from the `BlindingRandomness`, and a `challenge` and `index` given as
    /// input.
    fn gen_response(
        &self,
        challenge: &Scalar,
        index: &bool,
    ) -> ResponseRandomness {
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

    fn new(ck: &CommitmentKey, blinding_randomness: &BlindingRandomness, index: &Scalar) -> Self {
        assert!(index == &Scalar::zero() || index == &Scalar::one());

        // commit index bit: 0 or 1
        let i = Commitment::new(ck, &index, &blinding_randomness.alpha);
        // commit beta
        let b = Commitment::new(ck, &blinding_randomness.beta, &blinding_randomness.gamma);
        // commit i * B => 0 * B = 0 or 1 * B = B
        let a = if index == &Scalar::one() {
            Commitment::new(
                ck,
                &blinding_randomness.beta.clone(),
                &blinding_randomness.delta,
            )
        } else {
            Commitment::new(ck, &Scalar::zero(), &blinding_randomness.delta)
        };

        Announcement { i, b, a }
    }
}

/// Response encoding the bits of the private vector, and the randomness of `BlindingRandomness`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ResponseRandomness {
    z: Scalar,
    w: Scalar,
    v: Scalar,
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

fn generate_polys(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypted::EncryptingVote;
    use crate::encryption::Keypair;
    use crate::unit_vector::UnitVector;
    use rand_chacha::ChaCha20Rng;
    use rand_core::SeedableRng;

    #[test]
    fn prove_verify1() {
        let mut r = ChaCha20Rng::from_seed([0u8; 32]);
        let public_key = Keypair::generate(&mut r).public_key;
        let unit_vector = UnitVector::new(2, 0);
        let ev = EncryptingVote::prepare(&mut r, &public_key, &unit_vector);

        let mut shared_string =
            b"Example of a shared string. This could be the latest block hash".to_owned();
        let crs = CRS::from_hash(&mut shared_string);

        let proof = Proof::prove(&mut r, &crs, &public_key, ev.clone());
        assert!(proof.verify(&crs, &public_key, &ev.ciphertexts))
    }

    #[test]
    fn prove_verify() {
        let mut r = ChaCha20Rng::from_seed([0u8; 32]);
        let public_key = Keypair::generate(&mut r).public_key;
        let unit_vector = UnitVector::new(5, 1);
        let ev = EncryptingVote::prepare(&mut r, &public_key, &unit_vector);

        let mut shared_string =
            b"Example of a shared string. This could be the latest block hash".to_owned();
        let crs = CRS::from_hash(&mut shared_string);

        let proof = Proof::prove(&mut r, &crs, &public_key, ev.clone());
        assert!(proof.verify(&crs, &public_key, &ev.ciphertexts))
    }
}
