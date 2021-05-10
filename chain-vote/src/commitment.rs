use crate::CRS;
use crate::gang::{GroupElement, Scalar};
use std::ops::{Add, Mul};

/// Pedersen commitment
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Commitment {
    c: GroupElement,
}

#[derive(Clone)]
pub struct CommitmentKey {
    pub h: GroupElement,
}

impl CommitmentKey {
    pub fn to_bytes(&self) -> [u8; GroupElement::BYTES_LEN] {
        self.h.to_bytes()
    }

    /// Generate a commitment key from a seed. This function hashes the
    /// input `buffer`, and creates a group element out of the hash.
    pub fn generate_from_seed(buffer: &mut [u8]) -> Self {
        CommitmentKey {
            h: GroupElement::from_hash(buffer),
        }
    }

    /// Return a commitment with the given opening, `o`
    pub fn commit_with_open(&self, o: &Open) -> Commitment {
        self.commit(&o.m, &o.r)
    }

    /// Return a commitment with the given message, `m`,  and opening key, `r`
    pub fn commit(&self, m: &Scalar, r: &Scalar) -> Commitment {
        let c = GroupElement::generator() * m + &self.h * r;
        Commitment { c }
    }
}

impl From<CRS> for CommitmentKey {
    fn from(crs: CRS) -> Self {
        CommitmentKey { h: crs }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Validity {
    Valid,
    Invalid,
}

#[derive(Clone)]
pub struct Open {
    m: Scalar,
    r: Scalar,
}

impl Commitment {
    pub const BYTES_LEN: usize = GroupElement::BYTES_LEN;

    /// Verify that a given opening, `o`,  corresponds to the commitment under a
    /// given commitment key `ck`
    pub fn verify(&self, ck: &CommitmentKey, o: &Open) -> Validity {
        let other = ck.commit_with_open(o);
        if self == &other {
            Validity::Valid
        } else {
            Validity::Invalid
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::BYTES_LEN] {
        self.c.to_bytes()
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        Some(Self {
            c: GroupElement::from_bytes(buf)?,
        })
    }
}

impl<'a, 'b> Add<&'b Commitment> for &'a Commitment {
    type Output = Commitment;
    fn add(self, rhs: &'b Commitment) -> Self::Output {
        let c = &self.c + &rhs.c;
        Commitment { c }
    }
}

std_ops_gen!(Commitment, Add, Commitment, Commitment, add);

impl<'a, 'b> Mul<&'b Scalar> for &'a Commitment {
    type Output = Commitment;
    fn mul(self, rhs: &'b Scalar) -> Self::Output {
        Commitment { c: &self.c * rhs }
    }
}

std_ops_gen!(Commitment, Mul, Scalar, Commitment, mul);
