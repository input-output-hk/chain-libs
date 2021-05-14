use crate::gang::{GroupElement, Scalar};
use crate::Crs;

/// Pedersen Commitment key
#[derive(Clone)]
pub struct CommitmentKey {
    pub h: GroupElement,
}

impl CommitmentKey {
    pub fn to_bytes(&self) -> [u8; GroupElement::BYTES_LEN] {
        self.h.to_bytes()
    }

    /// Return a commitment with the given opening, `o`
    pub fn commit_with_open(&self, o: &Open) -> GroupElement {
        self.commit(&o.m, &o.r)
    }

    /// Return a commitment with the given message, `m`,  and opening key, `r`
    pub fn commit(&self, m: &Scalar, r: &Scalar) -> GroupElement {
        GroupElement::generator() * m + &self.h * r

    }

    /// Verify that a given `commitment` opens to `o` under commitment key `self`
    #[allow(dead_code)]
    pub fn verify(&self, commitment: &GroupElement, o: &Open) -> Validity {
        let other = self.commit_with_open(o);
        if commitment == &other {
            Validity::Valid
        } else {
            Validity::Invalid
        }
    }
}

impl From<Crs> for CommitmentKey {
    fn from(crs: Crs) -> Self {
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
