//! EVM transactions
use chain_core::{
    packer::Codec,
    property::{DeserializeFromSlice, ReadError},
};
#[cfg(feature = "evm")]
use chain_evm::{
    ethereum_types::{H256, U256},
    machine::{convert_access_list_to_tuples_vec, AccessList, AccessListItem, Address},
    rlp::{Decodable, DecoderError, Encodable, Rlp, RlpStream},
    state::{ByteCode, Key},
};
use typed_bytes::ByteBuilder;

use crate::{
    certificate::CertificateSlice,
    transaction::{Payload, PayloadAuthData, PayloadData},
};

#[cfg(feature = "evm")]
pub use chain_evm::Config;

/// Variants of supported EVM transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvmTransaction {
    #[cfg(feature = "evm")]
    Create {
        caller: Address,
        value: U256,
        init_code: ByteCode,
        gas_limit: u64,
        access_list: AccessList,
    },
    #[cfg(feature = "evm")]
    Create2 {
        caller: Address,
        value: U256,
        init_code: ByteCode,
        salt: H256,
        gas_limit: u64,
        access_list: AccessList,
    },
    #[cfg(feature = "evm")]
    Call {
        caller: Address,
        address: Address,
        value: U256,
        data: ByteCode,
        gas_limit: u64,
        access_list: AccessList,
    },
}

#[cfg(feature = "evm")]
impl From<&EvmTransaction> for u8 {
    fn from(other: &EvmTransaction) -> Self {
        use EvmTransaction::*;
        match other {
            Create { .. } => 0,
            Create2 { .. } => 1,
            Call { .. } => 2,
        }
    }
}

#[cfg(feature = "evm")]
impl Decodable for EvmTransaction {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        fn decode_tx_create(rlp: &Rlp<'_>) -> Result<EvmTransaction, DecoderError> {
            Ok(EvmTransaction::Create {
                caller: rlp.val_at(1)?,
                value: rlp.val_at(2)?,
                init_code: rlp.val_at(3)?,
                gas_limit: rlp.val_at(4)?,
                access_list: rlp.list_at(5)?,
            })
        }
        fn decode_tx_create2(rlp: &Rlp<'_>) -> Result<EvmTransaction, DecoderError> {
            Ok(EvmTransaction::Create2 {
                caller: rlp.val_at(1)?,
                value: rlp.val_at(2)?,
                init_code: rlp.val_at(3)?,
                salt: rlp.val_at(4)?,
                gas_limit: rlp.val_at(5)?,
                access_list: rlp.list_at(6)?,
            })
        }
        fn decode_tx_call(rlp: &Rlp<'_>) -> Result<EvmTransaction, DecoderError> {
            Ok(EvmTransaction::Call {
                caller: rlp.val_at(1)?,
                address: rlp.val_at(2)?,
                value: rlp.val_at(3)?,
                data: rlp.val_at(4)?,
                gas_limit: rlp.val_at(5)?,
                access_list: rlp.list_at(6)?,
            })
        }

        match rlp.val_at(0)? {
            0u8 => decode_tx_create(rlp),
            1u8 => decode_tx_create2(rlp),
            2u8 => decode_tx_call(rlp),
            _ => Err(DecoderError::Custom("invalid evm transaction")),
        }
    }
}

#[cfg(feature = "evm")]
impl Encodable for EvmTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        use EvmTransaction::*;
        match self {
            Create {
                caller,
                value,
                init_code,
                gas_limit,
                access_list,
            } => {
                s.begin_list(6);
                s.append(&u8::from(self));
                s.append(caller);
                s.append(value);
                s.append(init_code);
                s.append(gas_limit);
                s.append_list(access_list);
            }
            Create2 {
                caller,
                value,
                init_code,
                salt,
                gas_limit,
                access_list,
            } => {
                s.begin_list(7);
                s.append(&u8::from(self));
                s.append(caller);
                s.append(value);
                s.append(init_code);
                s.append(salt);
                s.append(gas_limit);
                s.append_list(access_list);
            }
            Call {
                caller,
                address,
                value,
                data,
                gas_limit,
                access_list,
            } => {
                s.begin_list(7);
                s.append(&u8::from(self));
                s.append(caller);
                s.append(address);
                s.append(value);
                s.append(data);
                s.append(gas_limit);
                s.append_list(access_list);
            }
        }
    }
}

impl EvmTransaction {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, _bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        #[cfg(feature = "evm")]
        {
            _bb.bytes(&self.rlp_bytes())
        }
        #[cfg(not(feature = "evm"))]
        {
            _bb
        }
    }
}

#[cfg(feature = "evm")]
/// Serializes H160 types as fixed bytes.
pub fn serialize_address<T>(bb: ByteBuilder<T>, caller: &Address) -> ByteBuilder<T> {
    bb.bytes(caller.as_fixed_bytes())
}

#[cfg(feature = "evm")]
/// Serializes U256 types as fixed bytes.
pub fn serialize_u256<T>(bb: ByteBuilder<T>, value: &U256) -> ByteBuilder<T> {
    let mut value_bytes = [0u8; 32];
    value.to_big_endian(&mut value_bytes);
    bb.bytes(&value_bytes)
}

#[cfg(feature = "evm")]
/// Serializes H256 types as fixed bytes.
pub fn serialize_h256<T>(bb: ByteBuilder<T>, value: &H256) -> ByteBuilder<T> {
    bb.bytes(value.as_fixed_bytes())
}

#[cfg(feature = "evm")]
/// Serializes H256 types as fixed bytes.
pub fn serialize_h256_list<T>(bb: ByteBuilder<T>, value: &[H256]) -> ByteBuilder<T> {
    bb.u64(value.len() as u64)
        .fold(value.iter(), serialize_h256)
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
fn read_address(codec: &mut Codec<&[u8]>) -> Result<Address, ReadError> {
    Ok(Address::from_slice(codec.get_slice(20)?))
}

#[cfg(feature = "evm")]
fn read_h256(codec: &mut Codec<&[u8]>) -> Result<H256, ReadError> {
    Ok(H256::from_slice(codec.get_slice(32)?))
}

#[cfg(feature = "evm")]
pub fn read_u256(codec: &mut Codec<&[u8]>) -> Result<U256, ReadError> {
    Ok(U256::from(codec.get_slice(32)?))
}

#[cfg(feature = "evm")]
fn read_bytecode(codec: &mut Codec<&[u8]>) -> Result<ByteCode, ReadError> {
    match codec.get_be_u64()? {
        n if n > 0 => Ok(ByteCode::from(codec.get_slice(n.try_into().unwrap())?)),
        _ => Ok(ByteCode::default()),
    }
}

#[cfg(feature = "evm")]
fn read_gas_limit(codec: &mut Codec<&[u8]>) -> Result<u64, ReadError> {
    codec.get_be_u64()
}

#[cfg(feature = "evm")]
fn read_access_list(codec: &mut Codec<&[u8]>) -> Result<AccessList, ReadError> {
    let count = codec.get_be_u64()?;
    let access_list = (0..count)
        .into_iter()
        .fold(Vec::new(), |mut access_list, _| {
            let address = read_address(codec).unwrap_or_default();
            let keys_count = codec.get_be_u64().unwrap_or_default();
            let keys = (0..keys_count).into_iter().fold(Vec::new(), |mut keys, _| {
                let key = read_h256(codec).unwrap_or_default();
                if !key.is_zero() {
                    keys.push(key);
                }
                keys
            });
            access_list.push(AccessListItem {
                address,
                storage_keys: keys,
            });
            access_list
        });
    Ok(access_list)
}

impl DeserializeFromSlice for EvmTransaction {
    fn deserialize_from_slice(codec: &mut Codec<&[u8]>) -> Result<Self, ReadError> {
        let mut rlp_bytes = vec![];
        codec.read_to_end(&mut rlp_bytes)?;
        let rlp = Rlp::new(&rlp_bytes);
        EvmTransaction::decode(&rlp).map_err(|e| ReadError::InvalidData(format!("{:?}", e)))
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
    use chain_evm::ethereum_types::{H160, H256};
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmTransaction {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let caller = [u8::arbitrary(g); H160::len_bytes()].into();
            let value = u128::arbitrary(g).into();
            let gas_limit = Arbitrary::arbitrary(g);
            let access_list: AccessList = match u8::arbitrary(g) % 5 {
                0 => vec![],
                _ => vec![
                    AccessListItem {
                        address: H160::from([u8::arbitrary(g); H160::len_bytes()]),
                        storage_keys: vec![
                            H256::from([u8::arbitrary(g); H256::len_bytes()]),
                            H256::from([u8::arbitrary(g); H256::len_bytes()]),
                        ],
                    },
                    AccessListItem {
                        address: H160::from([u8::arbitrary(g); H160::len_bytes()]),
                        storage_keys: vec![
                            H256::from([u8::arbitrary(g); H256::len_bytes()]),
                            H256::from([u8::arbitrary(g); H256::len_bytes()]),
                        ],
                    },
                    AccessListItem {
                        address: H160::from([u8::arbitrary(g); H160::len_bytes()]),
                        storage_keys: vec![
                            H256::from([u8::arbitrary(g); H256::len_bytes()]),
                            H256::from([u8::arbitrary(g); H256::len_bytes()]),
                        ],
                    },
                ],
            };
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
                    salt: [u8::arbitrary(g); H256::len_bytes()].into(),
                    gas_limit,
                    access_list,
                },
                2 => Self::Call {
                    caller,
                    address: [u8::arbitrary(g); H160::len_bytes()].into(),
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
        // this tests RLP encoding/decoding using the Payload/DeserializeFromSlice traits
        fn evm_transaction_serialization_bijection_codec(b: EvmTransaction) -> bool {
            let bytes = b.serialize_in(ByteBuilder::new()).finalize_as_vec();
            let decoded = EvmTransaction::deserialize_from_slice(&mut Codec::new(&bytes)).unwrap();
            decoded == b
        }
    }

    quickcheck! {
        // this tests RLP encoding/decoding
        fn evm_transaction_serialization_bijection_rlp(b: EvmTransaction) -> bool {
            let bytes = b.rlp_bytes();
            let rlp = Rlp::new(bytes.as_ref());
            let decoded = EvmTransaction::decode(&rlp).unwrap();
            decoded == b
        }
    }
}
