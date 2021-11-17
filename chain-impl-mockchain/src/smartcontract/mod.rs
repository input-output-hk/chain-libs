//! EVM Smart Contract transactions

use chain_core::mempack::{ReadError, Readable};
#[cfg(feature = "evm")]
use chain_evm::{
    machine::Value,
    primitive_types,
    state::{ByteCode, Key},
    Address,
};
#[cfg(feature = "evm")]
use std::convert::TryInto;
use typed_bytes::ByteBuilder;

use crate::{
    certificate::CertificateSlice,
    transaction::{Payload, PayloadAuthData, PayloadData},
};

/// Variants of Smart Contract deployment
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Contract {
    #[cfg(feature = "evm")]
    Create {
        caller: Address,
        value: Value,
        init_code: ByteCode,
        gas_limit: u64,
        access_list: Vec<(Address, Vec<Key>)>,
    },
    #[cfg(feature = "evm")]
    Create2 {
        caller: Address,
        value: Value,
        init_code: ByteCode,
        salt: primitive_types::H256,
        gas_limit: u64,
        access_list: Vec<(Address, Vec<Key>)>,
    },
    #[cfg(feature = "evm")]
    Call {
        caller: Address,
        address: Address,
        value: Value,
        data: ByteCode,
        gas_limit: u64,
        access_list: Vec<(Address, Vec<Key>)>,
    },
}

impl Contract {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, _bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        match self {
            #[cfg(feature = "evm")]
            Contract::Create {
                caller,
                value,
                init_code,
                gas_limit,
                access_list,
            } => {
                // Set Contract type
                let bb = _bb.u8(0);
                let bb = serialize_address(bb, caller);
                let bb = serialize_u256(bb, value);
                let bb = serialize_bytecode(bb, init_code);
                let bb = serialize_gas_limit(bb, gas_limit);
                serialize_access_list(bb, access_list)
            }
            #[cfg(feature = "evm")]
            Contract::Create2 { .. } => todo!(),
            #[cfg(feature = "evm")]
            Contract::Call { .. } => todo!(),
            #[cfg(not(feature = "evm"))]
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "evm")]
fn serialize_address(bb: ByteBuilder<Contract>, caller: &Address) -> ByteBuilder<Contract> {
    bb.u8(0).bytes(caller.as_fixed_bytes())
}

#[cfg(feature = "evm")]
fn serialize_u256(
    bb: ByteBuilder<Contract>,
    value: &primitive_types::U256,
) -> ByteBuilder<Contract> {
    let mut value_bytes = [0u8; 32];
    value.to_big_endian(&mut value_bytes);
    bb.bytes(&value_bytes)
}

#[cfg(feature = "evm")]
fn serialize_h256(
    bb: ByteBuilder<Contract>,
    value: &primitive_types::H256,
) -> ByteBuilder<Contract> {
    bb.bytes(value.as_fixed_bytes())
}

#[cfg(feature = "evm")]
fn serialize_bytecode(bb: ByteBuilder<Contract>, code: &ByteCode) -> ByteBuilder<Contract> {
    bb.u64(code.len().try_into().unwrap()).bytes(code.as_ref())
}

#[cfg(feature = "evm")]
fn serialize_gas_limit(bb: ByteBuilder<Contract>, gas_limit: &u64) -> ByteBuilder<Contract> {
    bb.u64(*gas_limit)
}

#[cfg(feature = "evm")]
fn serialize_access_list(
    bb: ByteBuilder<Contract>,
    access_list: &[(Address, Vec<Key>)],
) -> ByteBuilder<Contract> {
    bb.u64(access_list.len().try_into().unwrap())
        .fold(access_list.iter(), |bb, (address, keys)| {
            serialize_address(bb, address)
                .u64(keys.len().try_into().unwrap())
                .fold(keys.iter(), |bb, key| serialize_h256(bb, key))
        })
}

#[cfg(feature = "evm")]
fn read_address(
    buf: &mut chain_core::mempack::ReadBuf,
) -> Result<Address, chain_core::mempack::ReadError> {
    Ok(Address::from_slice(buf.get_slice(20)?))
}

#[cfg(feature = "evm")]
fn read_h256(
    buf: &mut chain_core::mempack::ReadBuf,
) -> Result<primitive_types::H256, chain_core::mempack::ReadError> {
    Ok(primitive_types::H256::from_slice(buf.get_slice(32)?))
}

#[cfg(feature = "evm")]
fn read_u256(
    buf: &mut chain_core::mempack::ReadBuf,
) -> Result<primitive_types::U256, chain_core::mempack::ReadError> {
    Ok(primitive_types::U256::from(buf.get_slice(32)?))
}

#[cfg(feature = "evm")]
fn read_bytecode(
    buf: &mut chain_core::mempack::ReadBuf,
) -> Result<ByteCode, chain_core::mempack::ReadError> {
    match buf.get_u64()? {
        n if n > 0 => Ok(ByteCode::from(buf.get_slice(n.try_into().unwrap())?)),
        _ => Ok(ByteCode::default()),
    }
}

#[cfg(feature = "evm")]
fn read_gas_limit(
    buf: &mut chain_core::mempack::ReadBuf,
) -> Result<u64, chain_core::mempack::ReadError> {
    buf.get_u64()
}

#[cfg(feature = "evm")]
fn read_access_list(
    buf: &mut chain_core::mempack::ReadBuf,
) -> Result<Vec<(Address, Vec<Key>)>, chain_core::mempack::ReadError> {
    let count = buf.get_u64()?;
    let access_list = (0..count)
        .into_iter()
        .fold(Vec::new(), |mut access_list, _| {
            let address = read_address(buf).unwrap_or_default();
            let keys_count = buf.get_u64().unwrap_or_default();
            let keys = (0..keys_count).into_iter().fold(Vec::new(), |mut keys, _| {
                let key = read_h256(buf).unwrap_or_default();
                if !key.is_zero() {
                    keys.push(key);
                }
                keys
            });
            access_list.push((address, keys));
            access_list
        });
    Ok(access_list)
}

impl Readable for Contract {
    fn read(
        buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        let contract_type = buf.get_u8()?;
        match contract_type {
            #[cfg(feature = "evm")]
            0 => {
                // CREATE Contract
                let caller = read_address(buf)?;
                let value = read_u256(buf)?;
                let init_code = read_bytecode(buf)?;
                let gas_limit = read_gas_limit(buf)?;
                let access_list = read_access_list(buf)?;

                buf.expect_end()?;

                Ok(Contract::Create {
                    caller,
                    value,
                    init_code,
                    gas_limit,
                    access_list,
                })
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
        let sender = Address::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = None;

        let contract_type = 0; // Contract::Create = 0
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

        let expected = Contract::Create {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(contract, expected);

        // Example with contract that has data
        let sender = Address::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = vec![0, 1, 2, 3];

        let contract_type = 0; // Contract::Create = 0
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

        let expected = Contract::Create {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data: Some(data.into_boxed_slice()),
        };

        assert_eq!(contract, expected);

        // Example with contract that says it has data, but has no data
        let sender = Address::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = None;

        let contract_type = 0; // Contract::Create = 0
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

        let expected = Contract::Create {
            sender,
            address,
            gas: Some(gas),
            gas_price: Some(gas_price),
            value,
            data,
        };

        assert_eq!(contract, expected);

        // Example with contract with truncated byte-array
        let contract_type = 0; // Contract::Create = 0

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
        let sender = Address::random();
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

        let contract = Contract::Create {
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
        let sender = Address::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = vec![0, 1, 2, 3];

        let contract_type = 0; // Contract::Create = 0
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

        let contract = Contract::Create {
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
        let sender = Address::random();
        let address = None;
        let gas: Gas = 10000.into();
        let gas_price: GasPrice = 2000.into();
        let value = None;
        let data = Vec::new().into_boxed_slice();

        let contract_type = 0; // Contract::Create = 0
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

        let contract = Contract::Create {
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
