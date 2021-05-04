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
    pub fn generate_from_seed(buffer: &mut [u8]) -> Self {
        CommitmentKey {
            h: GroupElement::from_hash(buffer),
        }
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

    pub fn new_open(ck: &CommitmentKey, o: &Open) -> Self {
        let c = GroupElement::generator() * &o.m + &ck.h * &o.r;
        Commitment { c }
    }

    pub fn new(ck: &CommitmentKey, m: &Scalar, r: &Scalar) -> Self {
        let c = GroupElement::generator() * m + &ck.h * r;
        Commitment { c }
    }

    pub fn verify(&self, ck: &CommitmentKey, o: &Open) -> Validity {
        let other = GroupElement::generator() * &o.m + &ck.h * &o.r;
        if self.c == other {
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
