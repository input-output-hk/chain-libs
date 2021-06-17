use crate::cryptography::{Ciphertext, PublicKey};
use crate::gang::Scalar;
use rand_core::{CryptoRng, RngCore};

// Power of Two Padded vector structure
#[derive(Clone)]
pub struct Ptp<A> {
    pub elements: Vec<A>,
    pub orig_len: usize,
}

impl<A: Clone> Ptp<A> {
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn bits(&self) -> usize {
        let len = self.elements.len();
        assert!(len.is_power_of_two());
        len.trailing_zeros() as usize
    }

    pub fn new<F>(mut vec: Vec<A>, extended_value: F) -> Ptp<A>
    where
        A: Clone,
        F: Fn() -> A,
    {
        let orig_len = vec.len();

        let expected_len = orig_len.next_power_of_two();
        if orig_len < expected_len {
            let a = extended_value();
            while vec.len() < expected_len {
                vec.push(a.clone());
            }
        }
        Ptp {
            orig_len,
            elements: vec,
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, A> {
        self.elements.iter()
    }
}

impl<A> AsRef<[A]> for Ptp<A> {
    fn as_ref(&self) -> &[A] {
        &self.elements
    }
}

#[derive(Clone)]
pub struct EncryptedVote(Vec<Ciphertext>);

/// Encrypted vote is a unit vector where each element is encrypted with ElGamal Ciphertext to
/// the tally opener.
#[derive(Clone)]
pub struct EncryptingVote {
    pub(crate) unit_vector: UnitVector,
    pub ciphertexts: Vec<Ciphertext>,
    pub random_elements: Vec<Scalar>,
}

impl EncryptingVote {
    pub fn prepare<R: RngCore + CryptoRng>(
        rng: &mut R,
        public_key: &PublicKey,
        vote: &UnitVector,
    ) -> Self {
        let mut rs = Vec::new();
        let mut ciphers = Vec::new();
        for vote_element in vote.iter() {
            let (cipher, r) = public_key.encrypt_return_r(&vote_element.into(), rng);
            rs.push(r);
            ciphers.push(cipher);
        }
        Self {
            unit_vector: *vote,
            ciphertexts: ciphers,
            random_elements: rs,
        }
    }

    /*
    pub fn pad<F>(mut self, extended_value: F) -> PtpEncryptingVote
    where
        F: Fn() -> (Scalar, Ciphertext),
    {
        let orig_len = self.ciphertexts.len();

        let expected_len = orig_len.next_power_of_two();
        if orig_len < expected_len {
            let (field_element, zero_cipher) = extended_value();
            while self.ciphertexts.len() < expected_len {
                self.ciphertexts.push(zero_cipher.clone());
                self.random_elements.push(field_element);
            }
        }
        PtpEncryptingVote {
            actual_length: orig_len,
            encrypting_vote: self,
        }
    }
    */
}

#[derive(Clone, Copy)]
/// Represent a Unit vector which size is @size and the @ith element (0-indexed) is enabled
pub struct UnitVector {
    ith: usize,
    size: usize,
}

impl std::fmt::Debug for UnitVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "e_{}({})", self.ith, self.size)
    }
}

impl std::fmt::Display for UnitVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "e_{}({})", self.ith, self.size)
    }
}

// `is_empty` cannot ever be useful in the case of UnitVector,
// as the size will always be > 0 as enforced in new()
#[allow(clippy::len_without_is_empty)]
impl UnitVector {
    /// Create a new
    pub fn new(size: usize, ith: usize) -> Self {
        assert!(size > 0);
        assert!(ith < size);
        UnitVector { ith, size }
    }

    pub fn iter(&self) -> UnitVectorIter {
        UnitVectorIter(0, *self)
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn ith(&self) -> usize {
        self.ith
    }

    pub fn is_jth(&self, j: usize) -> bool {
        if j >= self.size {
            panic!(
                "out of bounds: unit vector {} accessing index {}",
                self.size, j
            );
        }
        j == self.ith
    }

    pub fn jth(&self, j: usize) -> Scalar {
        if j >= self.size {
            panic!(
                "out of bounds: unit vector {} accessing index {}",
                self.size, j
            );
        } else if j == self.ith {
            Scalar::one()
        } else {
            Scalar::zero()
        }
    }
}

pub fn binrep(n: usize, digits: u32) -> Vec<bool> {
    assert!(n < 2usize.pow(digits));
    (0..digits)
        .rev()
        .map(|i: u32| (n & (1 << i)) != 0)
        .collect::<Vec<bool>>()
}

#[derive(Clone, Copy)]
pub struct UnitVectorIter(usize, UnitVector);

impl Iterator for UnitVectorIter {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        let i = self.0;
        if i == self.1.size {
            None
        } else {
            self.0 += 1;
            Some(i == self.1.ith)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_vector() {
        let uv = UnitVector::new(5, 0);
        assert_eq!(
            &uv.iter().collect::<Vec<_>>()[..],
            [true, false, false, false, false]
        );
        assert_eq!(
            &uv.iter().collect::<Vec<_>>()[..],
            &(0usize..5).map(|i| uv.is_jth(i)).collect::<Vec<_>>()[..]
        );

        let uv = UnitVector::new(5, 4);
        assert_eq!(
            &uv.iter().collect::<Vec<_>>()[..],
            [false, false, false, false, true]
        );

        assert_eq!(
            &uv.iter().collect::<Vec<_>>()[..],
            &(0usize..5).map(|i| uv.is_jth(i)).collect::<Vec<_>>()[..]
        );
    }

    #[test]
    fn unit_binrep() {
        assert_eq!(binrep(3, 5), &[false, false, false, true, true])
    }
}
