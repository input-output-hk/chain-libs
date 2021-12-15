//! EVM transactions
use chain_core::{
    packer::Codec,
    property::{DeserializeFromSlice, ReadError},
};
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

/// Variants of supported EVM transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transaction {
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

impl Transaction {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, _bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        match self {
            #[cfg(feature = "evm")]
            Transaction::Create {
                caller,
                value,
                init_code,
                gas_limit,
                access_list,
            } => {
                // Set Transaction type
                let bb = _bb.u8(0);
                let bb = serialize_address(bb, caller);
                let bb = serialize_u256(bb, value);
                let bb = serialize_bytecode(bb, init_code);
                let bb = serialize_gas_limit(bb, gas_limit);
                serialize_access_list(bb, access_list)
            }
            #[cfg(feature = "evm")]
            Transaction::Create2 { .. } => todo!(),
            #[cfg(feature = "evm")]
            Transaction::Call { .. } => todo!(),
            #[cfg(not(feature = "evm"))]
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "evm")]
fn serialize_address(bb: ByteBuilder<Transaction>, caller: &Address) -> ByteBuilder<Transaction> {
    bb.u8(0).bytes(caller.as_fixed_bytes())
}

#[cfg(feature = "evm")]
fn serialize_u256(
    bb: ByteBuilder<Transaction>,
    value: &primitive_types::U256,
) -> ByteBuilder<Transaction> {
    let mut value_bytes = [0u8; 32];
    value.to_big_endian(&mut value_bytes);
    bb.bytes(&value_bytes)
}

#[cfg(feature = "evm")]
fn serialize_h256(
    bb: ByteBuilder<Transaction>,
    value: &primitive_types::H256,
) -> ByteBuilder<Transaction> {
    bb.bytes(value.as_fixed_bytes())
}

#[cfg(feature = "evm")]
fn serialize_bytecode(bb: ByteBuilder<Transaction>, code: &ByteCode) -> ByteBuilder<Transaction> {
    bb.u64(code.len().try_into().unwrap()).bytes(code.as_ref())
}

#[cfg(feature = "evm")]
fn serialize_gas_limit(bb: ByteBuilder<Transaction>, gas_limit: &u64) -> ByteBuilder<Transaction> {
    bb.u64(*gas_limit)
}

#[cfg(feature = "evm")]
fn serialize_access_list(
    bb: ByteBuilder<Transaction>,
    access_list: &[(Address, Vec<Key>)],
) -> ByteBuilder<Transaction> {
    bb.u64(access_list.len().try_into().unwrap())
        .fold(access_list.iter(), |bb, (address, keys)| {
            serialize_address(bb, address)
                .u64(keys.len().try_into().unwrap())
                .fold(keys.iter(), serialize_h256)
        })
}

#[cfg(feature = "evm")]
fn read_address(codec: &mut Codec<&[u8]>) -> Result<Address, ReadError> {
    Ok(Address::from_slice(codec.get_slice(20)?))
}

#[cfg(feature = "evm")]
fn read_h256(codec: &mut Codec<&[u8]>) -> Result<primitive_types::H256, ReadError> {
    Ok(primitive_types::H256::from_slice(codec.get_slice(32)?))
}

#[cfg(feature = "evm")]
fn read_u256(codec: &mut Codec<&[u8]>) -> Result<primitive_types::U256, ReadError> {
    Ok(primitive_types::U256::from(codec.get_slice(32)?))
}

#[cfg(feature = "evm")]
fn read_bytecode(codec: &mut Codec<&[u8]>) -> Result<ByteCode, ReadError> {
    match codec.get_u64()? {
        n if n > 0 => Ok(ByteCode::from(codec.get_slice(n.try_into().unwrap())?)),
        _ => Ok(ByteCode::default()),
    }
}

#[cfg(feature = "evm")]
fn read_gas_limit(codec: &mut Codec<&[u8]>) -> Result<u64, ReadError> {
    codec.get_u64()
}

#[cfg(feature = "evm")]
fn read_access_list(codec: &mut Codec<&[u8]>) -> Result<Vec<(Address, Vec<Key>)>, ReadError> {
    let count = codec.get_u64()?;
    let access_list = (0..count)
        .into_iter()
        .fold(Vec::new(), |mut access_list, _| {
            let address = read_address(codec).unwrap_or_default();
            let keys_count = codec.get_u64().unwrap_or_default();
            let keys = (0..keys_count).into_iter().fold(Vec::new(), |mut keys, _| {
                let key = read_h256(codec).unwrap_or_default();
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

impl DeserializeFromSlice for Transaction {
    fn deserialize_from_slice(codec: &mut Codec<&[u8]>) -> Result<Self, ReadError> {
        let contract_type = codec.get_u8()?;
        match contract_type {
            #[cfg(feature = "evm")]
            0 => {
                // CREATE Transaction
                let caller = read_address(codec)?;
                let value = read_u256(codec)?;
                let init_code = read_bytecode(codec)?;
                let gas_limit = read_gas_limit(codec)?;
                let access_list = read_access_list(codec)?;

                Ok(Transaction::Create {
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

impl Payload for Transaction {
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