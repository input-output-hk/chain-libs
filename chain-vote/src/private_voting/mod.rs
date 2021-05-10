//! Implementation of the Unit Vector ZK argument presented by
//! Zhang, Oliynykov and Balogum in "A Treasury System for Cryptocurrencies:
//! Enabling Better Collaborative Intelligence"
//! (https://www.ndss-symposium.org/wp-content/uploads/2019/02/ndss2019_02A-2_Zhang_paper.pdf)
//!
//! Given a common reference string formed by a pedersen commitment key,
//! the prover generates a proof that a tuple of encryptions corresponds to
//! the element-wise encryption of some unit vector, without disclosing the latter.
//! The proof communication complexity is logarithmic with respect to the size of
//! the encrypted tuple.


pub(crate) mod challenge_context;
pub mod unit_vector_zkp;

pub(crate) use self::{
    challenge_context::ChallengeContext, unit_vector_zkp::Announcement,
    unit_vector_zkp::Proof,
};
