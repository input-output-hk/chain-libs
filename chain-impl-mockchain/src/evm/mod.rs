//! EVM transactions

use chain_core::mempack::{ReadError, Readable};
#[cfg(feature = "evm")]
use chain_evm::{
    machine::Value,
    primitive_types,
    state::{ByteCode, Key},
    Address,
};
use typed_bytes::ByteBuilder;

use crate::{
    certificate::CertificateSlice,
    transaction::{Payload, PayloadAuthData, PayloadData},
};

/// Variants of supported EVM transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvmTransaction {
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

impl EvmTransaction {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, _bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        match self {
            #[cfg(feature = "evm")]
            EvmTransaction::Create {
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
            EvmTransaction::Create2 {
                caller,
                value,
                init_code,
                salt,
                gas_limit,
                access_list,
            } => {
                let bb = _bb.u8(1);
                let bb = serialize_address(bb, caller);
                let bb = serialize_u256(bb, value);
                let bb = serialize_bytecode(bb, init_code);
                let bb = serialize_h256(bb, salt);
                let bb = serialize_gas_limit(bb, gas_limit);
                serialize_access_list(bb, access_list)
            }
            #[cfg(feature = "evm")]
            EvmTransaction::Call {
                caller,
                address,
                value,
                data,
                gas_limit,
                access_list,
            } => {
                let bb = _bb.u8(2);
                let bb = serialize_address(bb, caller);
                let bb = serialize_address(bb, address);
                let bb = serialize_u256(bb, value);
                let bb = serialize_bytecode(bb, data);
                let bb = serialize_gas_limit(bb, gas_limit);
                serialize_access_list(bb, access_list)
            }
            #[cfg(not(feature = "evm"))]
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "evm")]
fn serialize_address(
    bb: ByteBuilder<EvmTransaction>,
    caller: &Address,
) -> ByteBuilder<EvmTransaction> {
    bb.bytes(caller.as_fixed_bytes())
}

#[cfg(feature = "evm")]
fn serialize_u256(
    bb: ByteBuilder<EvmTransaction>,
    value: &primitive_types::U256,
) -> ByteBuilder<EvmTransaction> {
    let mut value_bytes = [0u8; 32];
    value.to_big_endian(&mut value_bytes);
    bb.bytes(&value_bytes)
}

#[cfg(feature = "evm")]
fn serialize_h256(
    bb: ByteBuilder<EvmTransaction>,
    value: &primitive_types::H256,
) -> ByteBuilder<EvmTransaction> {
    bb.bytes(value.as_fixed_bytes())
}

#[cfg(feature = "evm")]
fn serialize_bytecode(bb: ByteBuilder<EvmTransaction>, code: &[u8]) -> ByteBuilder<EvmTransaction> {
    bb.u64(code.len() as u64).bytes(code)
}

#[cfg(feature = "evm")]
fn serialize_gas_limit(
    bb: ByteBuilder<EvmTransaction>,
    gas_limit: &u64,
) -> ByteBuilder<EvmTransaction> {
    bb.u64(*gas_limit)
}

#[cfg(feature = "evm")]
fn serialize_access_list(
    bb: ByteBuilder<EvmTransaction>,
    access_list: &[(Address, Vec<Key>)],
) -> ByteBuilder<EvmTransaction> {
    bb.u64(access_list.len() as u64)
        .fold(access_list.iter(), |bb, (address, keys)| {
            serialize_address(bb, address)
                .u64(keys.len() as u64)
                .fold(keys.iter(), serialize_h256)
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
        n if n > 0 => {
            dbg!(n);
            Ok(ByteCode::from(buf.get_slice(n as usize)?))
        }
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

impl Readable for EvmTransaction {
    fn read(
        buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        let contract_type = buf.get_u8()?;
        match contract_type {
            #[cfg(feature = "evm")]
            0 => {
                // CREATE Transactionё
                let caller = read_address(buf)?;
                let value = read_u256(buf)?;
                let init_code = read_bytecode(buf)?;
                let gas_limit = read_gas_limit(buf)?;
                let access_list = read_access_list(buf)?;

                buf.expect_end()?;

                Ok(EvmTransaction::Create {
                    caller,
                    value,
                    init_code,
                    gas_limit,
                    access_list,
                })
            }
            #[cfg(feature = "evm")]
            1 => {
                // CREATE2 Transaction
                let caller = read_address(buf)?;
                let value = read_u256(buf)?;
                let init_code = read_bytecode(buf)?;
                let salt = read_h256(buf)?;
                let gas_limit = read_gas_limit(buf)?;
                let access_list = read_access_list(buf)?;

                buf.expect_end()?;

                Ok(EvmTransaction::Create2 {
                    caller,
                    value,
                    init_code,
                    salt,
                    gas_limit,
                    access_list,
                })
            }
            #[cfg(feature = "evm")]
            2 => {
                // CALL Transaction
                let caller = read_address(buf)?;
                let address = read_address(buf)?;
                let value = read_u256(buf)?;
                let data = read_bytecode(buf)?;
                let gas_limit = read_gas_limit(buf)?;
                let access_list = read_access_list(buf)?;

                buf.expect_end()?;

                Ok(EvmTransaction::Call {
                    caller,
                    address,
                    value,
                    data,
                    gas_limit,
                    access_list,
                })
            }
            n => Err(ReadError::UnknownTag(n as u32)),
        }
    }
}

impl Payload for EvmTransaction {
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

#[cfg(all(any(test, feature = "property-test-api"), feature = "evm"))]
mod test {
    use super::*;
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmTransaction {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let caller = [u8::arbitrary(g); 20].into();
            let value = u128::arbitrary(g).into();
            let gas_limit = Arbitrary::arbitrary(g);
            let access_list = Vec::new();
            match u8::arbitrary(g) % 3 {
                0 => Self::Create {
                    caller,
                    value,
                    init_code: Arbitrary::arbitrary(g),
                    gas_limit,
                    access_list,
                },
                1 => Self::Create2 {
                    caller,
                    value,
                    init_code: Arbitrary::arbitrary(g),
                    salt: [u8::arbitrary(g); 32].into(),
                    gas_limit,
                    access_list,
                },
                2 => Self::Call {
                    caller,
                    address: [u8::arbitrary(g); 20].into(),
                    value,
                    data: Arbitrary::arbitrary(g),
                    gas_limit,
                    access_list,
                },
                _ => unreachable!(),
            }
        }
    }

    quickcheck! {
        fn evm_transaction_serialization_bijection(b: EvmTransaction) -> bool {
            let bytes = b.serialize_in(ByteBuilder::new()).finalize_as_vec();
            let decoded = EvmTransaction::read(&mut chain_core::mempack::ReadBuf::from(&bytes)).unwrap();
            decoded == b
        }
    }
}
