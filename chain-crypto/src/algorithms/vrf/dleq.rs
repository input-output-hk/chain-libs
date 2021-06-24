use crate::ec::{GroupElement, Scalar};
use cryptoxide::blake2b::Blake2b;
use cryptoxide::digest::Digest;

/// Proof of discrete logarithm equivalence
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    c: Challenge,
    z: Scalar,
}

impl Proof {
    pub const PROOF_SIZE: usize = Scalar::BYTES_LEN * 2;

    pub fn to_bytes(&self, output: &mut [u8]) {
        assert_eq!(output.len(), Self::PROOF_SIZE);
        output[0..32].copy_from_slice(&self.c.0.to_bytes());
        output[32..64].copy_from_slice(&self.z.to_bytes());
    }

    pub fn from_bytes(slice: &[u8]) -> Option<Self> {
        if slice.len() != Self::PROOF_SIZE {
            return None;
        }
        let mut c_array = [0u8; 32];
        c_array.copy_from_slice(&slice[0..32]);
        let c = Scalar::from_bytes(&c_array)?;

        let mut z_array = [0u8; 32];
        z_array.copy_from_slice(&slice[32..64]);
        let z = Scalar::from_bytes(&z_array)?;

        let proof = Proof { c: Challenge(c), z };
        Some(proof)
    }
}

/// Parameters for DLEQ where g1^a = h1, h2^a = h2
pub struct Dleq<'a> {
    pub g1: &'a GroupElement,
    pub h1: &'a GroupElement,
    pub g2: &'a GroupElement,
    pub h2: &'a GroupElement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Challenge(Scalar);

fn challenge(
    h1: &GroupElement,
    h2: &GroupElement,
    a1: &GroupElement,
    a2: &GroupElement,
) -> Challenge {
    let mut d = Blake2b::new(64);
    d.input(&h1.to_bytes());
    d.input(&h2.to_bytes());
    d.input(&a1.to_bytes());
    d.input(&a2.to_bytes());
    Challenge(Scalar::hash_to_scalar(&d))
}

/// Generate a zero knowledge of discrete log equivalence
///
/// where:
/// * g1^a = h1
/// * g2^a = h2
pub fn generate(w: &Scalar, a: &Scalar, dleq: &Dleq) -> Proof {
    let a1 = dleq.g1 * w;
    let a2 = dleq.g2 * w;
    let c = challenge(&dleq.h1, &dleq.h2, &a1, &a2);
    let z = w + a * &c.0;
    Proof { c, z }
}

/// Verify a zero knowledge proof of discrete log equivalence
pub fn verify(dleq: &Dleq, proof: &Proof) -> bool {
    let r1 = dleq.g1 * &proof.z;
    let r2 = dleq.g2 * &proof.z;
    let a1 = r1 - (dleq.h1 * &proof.c.0);
    let a2 = r2 - (dleq.h2 * &proof.c.0);
    // no need for constant time equality because of the hash in challenge()
    challenge(&dleq.h1, &dleq.h2, &a1, &a2) == proof.c
}

#[cfg(test)]
mod tests {
    use rand_core::OsRng;

    use super::*;

    #[test]
    #[allow(non_snake_case)]
    pub fn it_works() {
        let G = &GroupElement::generator();
        let H = GroupElement::from_hash(&G.to_bytes());
        let mut csprng: OsRng = OsRng;

        let a = Scalar::random(&mut csprng);
        let w = Scalar::random(&mut csprng);

        let dleq = Dleq {
            g1: G,
            h1: &(G * &a),
            g2: &H,
            h2: &(&H * &a),
        };
        let proof = generate(&w, &a, &dleq);
        assert_eq!(verify(&dleq, &proof), true);

        let dleq_bad = Dleq {
            g1: G,
            h1: &(G * a),
            g2: &H,
            h2: &(&H * w),
        };

        assert_eq!(verify(&dleq_bad, &proof), false);
    }
}
