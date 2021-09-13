//! EVM Smart Contract transactions

use chain_core::mempack::{ReadError, Readable};
#[cfg(feature = "evm")]
use chain_evm::{
    machine::{Gas, GasPrice, Value},
    state::{AccountAddress, ByteCode},
};

use crate::transaction::Payload;

/// Variants of Smart Contract deployment
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl Contract {}

impl Readable for Contract {
    fn read(
        buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        let contract_type = buf.get_u8()?;
        match contract_type {
            0 => {
                #[cfg(not(feature = "evm"))]
                {
                    Err(ReadError::UnknownTag(0))
                }
                #[cfg(feature = "evm")]
                {
                    // EVM Contract
                    let from = AccountAddress::from_slice(buf.get_slice(20)?);
                    let to = match buf.get_u8()? {
                        0 => None,
                        1 => Some(AccountAddress::from_slice(buf.get_slice(20)?)),
                        _ => {
                            return Err(ReadError::StructureInvalid("Invalid byte sequence".into()))
                        }
                    };
                    let gas = match buf.get_u8()? {
                        0 => None,
                        1 => Some(Gas::from(buf.get_slice(32)?)),
                        _ => {
                            return Err(ReadError::StructureInvalid("Invalid byte sequence".into()))
                        }
                    };
                    let gas_price = match buf.get_u8()? {
                        0 => None,
                        1 => Some(GasPrice::from(buf.get_slice(32)?)),
                        _ => {
                            return Err(ReadError::StructureInvalid("Invalid byte sequence".into()))
                        }
                    };
                    let value = match buf.get_u8()? {
                        0 => None,
                        1 => Some(GasPrice::from(buf.get_slice(32)?)),
                        _ => {
                            return Err(ReadError::StructureInvalid("Invalid byte sequence".into()))
                        }
                    };
                    let data = match buf.get_u8()? {
                        0 => None,
                        1 => Some(ByteCode::from(buf.get_slice_end())),
                        _ => {
                            return Err(ReadError::StructureInvalid("Invalid byte sequence".into()))
                        }
                    };
                    let contract = Contract::EVM {
                        from,
                        to,
                        gas,
                        gas_price,
                        value,
                        data,
                    };
                    Ok(contract)
                }
            }
            n => {
                //
                Err(ReadError::UnknownTag(n as u32))
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use typed_bytes::ByteBuilder;

    #[cfg(feature = "evm")]
    #[test]
    fn test_readable_evm_contract() {
        use chain_core::mempack::ReadBuf;
        use typed_bytes::ByteArray;

        let from = AccountAddress::random();
        let to = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = None;

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 0;

        let bb: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(from.as_fixed_bytes())
            .u8(has_to)
            .u8(has_gas)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(has_gas_price)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(has_value)
            .u8(has_data)
            .finalize();

        let mut readbuf = ReadBuf::from(bb.as_slice());
        let contract = Contract::read(&mut readbuf).unwrap();

        let expected = Contract::EVM {
            from,
            to,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(contract, expected);
    }
}
