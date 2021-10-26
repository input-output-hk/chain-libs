//! EVM Smart Contract transactions

use chain_core::mempack::{ReadError, Readable};
#[cfg(feature = "evm")]
use chain_evm::{
    machine::{Gas, GasPrice, Value},
    state::{AccountAddress, ByteCode},
};
use typed_bytes::ByteBuilder;

use crate::{
    certificate::CertificateSlice,
    transaction::{Payload, PayloadAuthData, PayloadData},
};

/// Variants of Smart Contract deployment
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Contract {
    #[cfg(feature = "evm")]
    /// Deploys a smart contract from a given `AccountAddress`, as
    /// perfomed by the `eth_sendTransaction` JSON-RPC method.
    EVM {
        /// The address from which the transaction is sent. Also referred to as `caller`.
        sender: AccountAddress,
        /// (optional when creating new contract) The address the transaction is directed to.
        address: Option<AccountAddress>,
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

impl Contract {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, _bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        match self {
            #[cfg(feature = "evm")]
            Contract::EVM {
                sender,
                address,
                gas,
                gas_price,
                value,
                data,
            } => {
                //
                let bb = _bb.u8(0).bytes(sender.as_fixed_bytes());
                let bb = if let Some(to_addr) = address {
                    bb.u8(1).bytes(to_addr.as_fixed_bytes())
                } else {
                    bb.u8(0)
                };
                let bb = if let Some(gas) = gas {
                    let mut gas_bytes = [0u8; 32];
                    gas.to_big_endian(&mut gas_bytes);
                    bb.u8(1).bytes(&gas_bytes)
                } else {
                    bb.u8(0)
                };
                let bb = if let Some(gas_price) = gas_price {
                    let mut gas_price_bytes = [0u8; 32];
                    gas_price.to_big_endian(&mut gas_price_bytes);
                    bb.u8(1).bytes(&gas_price_bytes)
                } else {
                    bb.u8(0)
                };
                let bb = if let Some(value) = value {
                    let mut value_bytes = [0u8; 32];
                    value.to_big_endian(&mut value_bytes);
                    bb.u8(1).bytes(&value_bytes)
                } else {
                    bb.u8(0)
                };
                let bb = if let Some(data) = data {
                    if !data.as_ref().is_empty() {
                        bb.u8(1).bytes(data.as_ref())
                    } else {
                        bb.u8(0)
                    }
                } else {
                    bb.u8(0)
                };
                bb
            }
            #[cfg(not(feature = "evm"))]
            _ => unreachable!(),
        }
    }
}

impl Readable for Contract {
    fn read(
        buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        let contract_type = buf.get_u8()?;
        match contract_type {
            #[cfg(feature = "evm")]
            0 => {
                // EVM Contract
                let sender = AccountAddress::from_slice(buf.get_slice(20)?);
                let address = match buf.get_u8()? {
                    0 => None,
                    1 => {
                        let a = AccountAddress::from_slice(buf.get_slice(20)?);
                        if a.is_zero() {
                            None
                        } else {
                            Some(a)
                        }
                    }
                    _ => return Err(ReadError::StructureInvalid("Invalid byte sequence".into())),
                };
                let gas = match buf.get_u8()? {
                    0 => None,
                    1 => {
                        let g = Gas::from(buf.get_slice(32)?);
                        if g.is_zero() {
                            None
                        } else {
                            Some(g)
                        }
                    }
                    _ => return Err(ReadError::StructureInvalid("Invalid byte sequence".into())),
                };
                let gas_price = match buf.get_u8()? {
                    0 => None,
                    1 => {
                        let gp = GasPrice::from(buf.get_slice(32)?);
                        if gp.is_zero() {
                            None
                        } else {
                            Some(gp)
                        }
                    }
                    _ => return Err(ReadError::StructureInvalid("Invalid byte sequence".into())),
                };
                let value = match buf.get_u8()? {
                    0 => None,
                    1 => {
                        let val = Value::from(buf.get_slice(32)?);
                        if val.is_zero() {
                            None
                        } else {
                            Some(val)
                        }
                    }
                    _ => return Err(ReadError::StructureInvalid("Invalid byte sequence".into())),
                };
                let data = match buf.get_u8()? {
                    0 => None,
                    1 => {
                        if buf.is_end() {
                            None
                        } else {
                            Some(ByteCode::from(buf.get_slice_end()))
                        }
                    }
                    _ => return Err(ReadError::StructureInvalid("Invalid byte sequence".into())),
                };

                if let Err(e) = buf.expect_end() {
                    Err(e)
                } else {
                    Ok(Contract::EVM {
                        sender,
                        address,
                        gas,
                        gas_price,
                        value,
                        data,
                    })
                }
            }
            n => Err(ReadError::UnknownTag(n as u32)),
        }
    }
}

impl Payload for Contract {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = false;
    type Auth = ();

    fn payload_data(&self) -> crate::transaction::PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            std::marker::PhantomData,
        )
    }
    fn payload_auth_data(_auth: &Self::Auth) -> crate::transaction::PayloadAuthData<Self> {
        PayloadAuthData(Vec::new().into(), std::marker::PhantomData)
    }
    fn payload_to_certificate_slice(
        _p: crate::transaction::PayloadSlice<'_, Self>,
    ) -> Option<CertificateSlice<'_>> {
        None
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "evm")]
    use super::*;
    #[cfg(feature = "evm")]
    use chain_core::mempack::{ReadBuf, ReadError};
    #[cfg(feature = "evm")]
    use typed_bytes::{ByteArray, ByteBuilder};

    #[cfg(feature = "evm")]
    #[test]
    fn test_readable_evm_contract() {
        // Example with contract that has no data
        let sender = AccountAddress::random();
        let address = None;
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
            .bytes(sender.as_fixed_bytes())
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
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(contract, expected);

        // Example with contract that has data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = vec![0, 1, 2, 3];

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 1;

        let bb: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
            .u8(has_to)
            .u8(has_gas)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(has_gas_price)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(has_value)
            .u8(has_data)
            .bytes(&data)
            .finalize();

        let mut readbuf = ReadBuf::from(bb.as_slice());
        let contract = Contract::read(&mut readbuf).unwrap();

        let expected = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data: Some(data.into_boxed_slice()),
        };

        assert_eq!(contract, expected);

        // Example with contract that says it has data, but has no data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = None;

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 1;

        let bb: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
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
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(contract, expected);

        // Example with contract with truncated byte-array
        let contract_type = 0; // Contract::EVM = 0

        let bb: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(&[0, 1, 2, 3, 4])
            .finalize();

        let mut readbuf = ReadBuf::from(bb.as_slice());

        assert_eq!(
            Contract::read(&mut readbuf),
            Err(ReadError::NotEnoughBytes(5, 20))
        );
    }
    #[cfg(feature = "evm")]
    #[test]
    fn test_serialize_in_evm_contract() {
        use typed_bytes::ByteArray;

        // Example with contract that has no data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = None;

        let expected: ByteArray<Contract> = ByteBuilder::new()
            .u8(0)
            .bytes(sender.as_fixed_bytes())
            .u8(0)
            .u8(1)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(1)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(0)
            .u8(0)
            .finalize();

        let contract = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(
            contract.serialize_in(ByteBuilder::new()).finalize(),
            expected
        );

        // Example with contract that says it has data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = vec![0, 1, 2, 3];

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 1;

        let expected: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
            .u8(has_to)
            .u8(has_gas)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(has_gas_price)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(has_value)
            .u8(has_data)
            .bytes(&data)
            .finalize();

        let contract = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data: Some(data.into_boxed_slice()),
        };

        assert_eq!(
            contract.serialize_in(ByteBuilder::new()).finalize(),
            expected
        );

        // Example with contract that says it has data, but has no data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = Vec::new().into_boxed_slice();

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 0;

        let expected: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
            .u8(has_to)
            .u8(has_gas)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(has_gas_price)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(has_value)
            .u8(has_data)
            .finalize();

        let contract = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data: Some(data),
        };

        assert_eq!(
            contract.serialize_in(ByteBuilder::new()).finalize(),
            expected
        );
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "evm")]
    use super::*;
    #[cfg(feature = "evm")]
    use chain_core::mempack::{ReadBuf, ReadError};
    #[cfg(feature = "evm")]
    use typed_bytes::{ByteArray, ByteBuilder};

    #[cfg(feature = "evm")]
    #[test]
    fn test_readable_evm_contract() {
        // Example with contract that has no data
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
            .bytes(sender.as_fixed_bytes())
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
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(contract, expected);

        // Example with contract that has data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = vec![0, 1, 2, 3];

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 1;

        let bb: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
            .u8(has_to)
            .u8(has_gas)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(has_gas_price)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(has_value)
            .u8(has_data)
            .bytes(&data)
            .finalize();

        let mut readbuf = ReadBuf::from(bb.as_slice());
        let contract = Contract::read(&mut readbuf).unwrap();

        let expected = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data: Some(data.into_boxed_slice()),
        };

        assert_eq!(contract, expected);

        // Example with contract that says it has data, but has no data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = None;

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 1;

        let bb: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
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
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(contract, expected);

        // Example with contract with truncated byte-array
        let contract_type = 0; // Contract::EVM = 0

        let bb: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(&[0, 1, 2, 3, 4])
            .finalize();

        let mut readbuf = ReadBuf::from(bb.as_slice());

        assert_eq!(
            Contract::read(&mut readbuf),
            Err(ReadError::NotEnoughBytes(5, 20))
        );
    }
    #[cfg(feature = "evm")]
    #[test]
    fn test_serialize_in_evm_contract() {
        use typed_bytes::ByteArray;

        // Example with contract that has no data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = None;

        let expected: ByteArray<Contract> = ByteBuilder::new()
            .u8(0)
            .bytes(sender.as_fixed_bytes())
            .u8(0)
            .u8(1)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(1)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(0)
            .u8(0)
            .finalize();

        let contract = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(
            contract.serialize_in(ByteBuilder::new()).finalize(),
            expected
        );

        // Example with contract that says it has data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = vec![0, 1, 2, 3];

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 1;

        let expected: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
            .u8(has_to)
            .u8(has_gas)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(has_gas_price)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(has_value)
            .u8(has_data)
            .bytes(&data)
            .finalize();

        let contract = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data: Some(data.into_boxed_slice()),
        };

        assert_eq!(
            contract.serialize_in(ByteBuilder::new()).finalize(),
            expected
        );

        // Example with contract that says it has data, but has no data
        let sender = AccountAddress::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = Vec::new().into_boxed_slice();

        let contract_type = 0; // Contract::EVM = 0
        let has_to = 0;
        let has_gas = 1;
        let has_gas_price = 1;
        let has_value = 0;
        let has_data = 0;

        let expected: ByteArray<Contract> = ByteBuilder::new()
            .u8(contract_type)
            .bytes(sender.as_fixed_bytes())
            .u8(has_to)
            .u8(has_gas)
            .bytes(&<[u8; 32]>::from(gas))
            .u8(has_gas_price)
            .bytes(&<[u8; 32]>::from(gas_price))
            .u8(has_value)
            .u8(has_data)
            .finalize();

        let contract = Contract::EVM {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data: Some(data),
        };

        assert_eq!(
            contract.serialize_in(ByteBuilder::new()).finalize(),
            expected
        );
    }
}
