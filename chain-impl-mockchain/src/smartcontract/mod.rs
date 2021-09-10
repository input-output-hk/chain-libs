//! EVM Smart Contract transactions

use chain_core::mempack::Readable;
#[cfg(feature = "evm")]
use chain_evm::{
    machine::{Gas, GasPrice, Value},
    state::{AccountAddress, ByteCode},
};

use crate::transaction::Payload;

/// Variants of Smart Contract deployment
pub enum Contract {
    #[cfg(feature = "evm")]
    /// Deploys a smart contract from a given `AccountAddress`, as
    /// perfomed by the `eth_sendTransaction` JSON-RPC method.
    EVM {
        /// The address the transaction is send from.
        from: AccountAddress,
        /// (optional when creating new contract) The address the transaction is directed to.
        to: Option<AccountAddress>,
        /// (optional, default: To-Be-Determined) Integer of the gas provided for the transaction execution.
        gas: Option<Gas>,
        /// (optional, default: To-Be-Determined) Integer of the gasPrice used for each payed gas.
        gas_price: Option<GasPrice>,
        /// (optional) Integer of the value send with this transaction.
        value: Option<Value>,
        /// (optional) The compiled code of a contract.
        data: Option<ByteCode>,
    },
}

impl Readable for Contract {
    fn read(
        _buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        todo!();
    }
}

impl Payload for Contract {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = false;
    type Auth = ();

    fn payload_data(&self) -> crate::transaction::PayloadData<Self> {
        todo!();
    }
    fn payload_auth_data(_auth: &Self::Auth) -> crate::transaction::PayloadAuthData<Self> {
        todo!();
    }
    fn payload_to_certificate_slice(
        _p: crate::transaction::PayloadSlice<'_, Self>,
    ) -> Option<crate::certificate::CertificateSlice<'_>> {
        todo!();
    }
}
