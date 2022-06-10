pub use ethereum;
pub use ethereum_types;
pub use rlp;
pub use sha3;

pub mod crypto;
pub mod machine;
mod precompiles;
pub mod state;
pub mod transaction;
pub mod util;

#[cfg(test)]
mod tests;

pub use machine::{AccessList, Address, BlockGasLimit, Config, Environment, GasLimit, GasPrice};
use thiserror::Error;

/// EVM-related error variants.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error(transparent)]
    VirtualMachine(crate::machine::Error),
    #[error("signature error")]
    Signature(#[from] ::secp256k1::Error),
    #[error("invalid signature")]
    InvalidSignature,
    #[error("rlp decoding error: {0}")]
    RlpDecoding(#[from] rlp::DecoderError),
}
