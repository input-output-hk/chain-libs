//! Errors related to chain-vote.
use thiserror::Error;
use chain_core::mempack::ReadError;
use crate::error::CryptoError::InvalidBuffer;


#[derive(Error, Debug, Clone)]
pub enum CryptoError {
    /// This error occurs when a unit vector ZKP failed to verify.
    #[error("Incorrect unit vector proof.")]
    UnitVectorZkpError,

    /// This error occurs when a commitment opening fails
    #[error("Commitment verification error.")]
    CommitmentVerificationError,

    /// This error occurs when a ZKP of correct decryption fails
    #[error("Incorrect decryption verification")]
    DecryptionZkpError,

    /// This error occurs when max log is reached when solving the discrete logarithm
    #[error("Max log reached. Could not compute discrete log")]
    MaxLogExceeded,

    /// This error occurs when the proof material is not consistent with the verified
    /// ciphertexts
    #[error("Invalid ciphertext size. Expected {0} and got {1}")]
    InvalidCiphertextSize(usize, usize),

    /// This error occurs when the parts used to form a unit vector proof do not have
    /// the same size
    #[error("Size of IBAs, Ds, and ZWVs must be the same")]
    InvalidPartsSizeUnitVectorZkp,

    /// This error occurs when we try to build a structure from a byte array with unexpected
    /// structure
    #[error("Invalid byte structure: {0}")]
    InvalidByteStructure(String),

    /// This error occurs when we try to build with an invalid buffer
    #[error("Invalid buffer: {0}")]
    InvalidBuffer(ReadError),
}

impl From<ReadError> for CryptoError {
    fn from(e: ReadError) -> CryptoError {
        match e {
            ReadError::StructureInvalid(string) => CryptoError::InvalidByteStructure(string),
            _ => InvalidBuffer(e),
        }
    }
}

impl From<CryptoError> for TallyError {
    fn from(e: CryptoError) -> TallyError {
        match e {
            CryptoError::MaxLogExceeded => TallyError::MaxLogExceeded,
            CryptoError::DecryptionZkpError => TallyError::DecryptionError,
            CryptoError::UnitVectorZkpError => TallyError::InvalidVoteProof,
            CryptoError::InvalidCiphertextSize(_, _) => TallyError::InvalidVoteProof,
            _ => TallyError::InvalidData(e),
        }
    }
}

#[derive(Debug, Error)]
pub enum TallyError {
    /// This error occurs when
    #[error("Invalid data which raised internal error: {0}")]
    InvalidData(CryptoError),

    /// This error occurs when max log is reached when decoding the decrypted tally
    #[error("Max log reached. Could not decode the decrypted tally")]
    MaxLogExceeded,

    /// This error occurs when a committee member submits an invalid decryption share
    #[error("Invalid decryption share submitted by committee member")]
    DecryptionError,

    /// This error occurs when the verification that the published results correspondong
    /// to the decrypted tally fails
    #[error("Decoded votes do not correspond to the decrypted tally")]
    ComparisonError,

    /// This error occurs when a new vote with invalid size (not the same as the Encrypted
    /// Tally) is inserted.
    #[error("Added vote has incorrect size")]
    InvalidNewVoteSize,

    /// This error occurs when a new vote is submitted with an invalid proof
    #[error("Invalid vote proof")]
    InvalidVoteProof,
}
