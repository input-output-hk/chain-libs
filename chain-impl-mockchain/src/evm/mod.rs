//! EVM transactions
use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, WriteError},
};
#[cfg(feature = "evm")]
use chain_evm::{
    ethereum_types::{H256, U256},
    machine::{AccessList, Address},
    rlp::{decode, Decodable, DecoderError, Encodable, Rlp, RlpStream},
    state::ByteCode,
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
            let bytes = self.rlp_bytes();
            _bb.u64(bytes.len() as u64).bytes(&bytes)
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
pub fn read_u256(codec: &mut Codec<&[u8]>) -> Result<U256, ReadError> {
    Ok(U256::from(codec.get_slice(32)?))
}

impl Serialize for EvmTransaction {
    fn serialize<W: std::io::Write>(&self, _codec: &mut Codec<W>) -> Result<(), WriteError> {
        #[cfg(feature = "evm")]
        {
            let bytes = self.rlp_bytes();
            _codec.put_be_u64(bytes.len() as u64)?;
            _codec.put_bytes(&bytes)?;
            Ok(())
        }
        #[cfg(not(feature = "evm"))]
        Err(WriteError::IoError(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "evm transactions are not supported in this build",
        )))
    }
}

impl Deserialize for EvmTransaction {
    fn deserialize<R: std::io::Read>(_codec: &mut Codec<R>) -> Result<Self, ReadError> {
        #[cfg(feature = "evm")]
        {
            let len = _codec.get_be_u64()?;
            let rlp_bytes = _codec.get_bytes(len as usize)?;
            decode(rlp_bytes.as_slice()).map_err(|e| ReadError::InvalidData(format!("{:?}", e)))
        }
        #[cfg(not(feature = "evm"))]
        Err(ReadError::IoError(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "evm transactions are not supported in this build",
        )))
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
    use chain_evm::{
        ethereum_types::{H160, H256},
        machine::AccessListItem,
    };
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmTransaction {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let caller = [u8::arbitrary(g); H160::len_bytes()].into();
            let value = u128::arbitrary(g).into();
            let gas_limit = Arbitrary::arbitrary(g);
            let access_list: AccessList = match u8::arbitrary(g) % 2 {
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
                    init_code: Box::new([Arbitrary::arbitrary(g); 32]),
                    gas_limit,
                    access_list,
                },
                1 => Self::Create2 {
                    caller,
                    value,
                    init_code: Box::new([Arbitrary::arbitrary(g); 32]),
                    salt: [u8::arbitrary(g); H256::len_bytes()].into(),
                    gas_limit,
                    access_list,
                },
                2 => Self::Call {
                    caller,
                    address: [u8::arbitrary(g); H160::len_bytes()].into(),
                    value,
                    data: Box::new([Arbitrary::arbitrary(g); 32]),
                    gas_limit,
                    access_list,
                },
                _ => unreachable!(),
            }
        }
    }

    quickcheck! {
        // this tests RLP encoding/decoding using the Serialize/Deserialize traits
        fn evm_transaction_serialization_bijection(b: EvmTransaction) -> bool {
            let encoded = b.serialize_as_vec().unwrap();
            let decoded = EvmTransaction::deserialize(&mut Codec::new(encoded.as_slice())).unwrap();
            decoded == b
        }
    }

    quickcheck! {
        // this tests RLP encoding/decoding using the rlp::Encodable/rlp::Decodable traits
        fn evm_transaction_serialization_bijection_rlp(b: EvmTransaction) -> bool {
            let rlp_encoded = b.rlp_bytes();
            let rlp_decoded: EvmTransaction = decode(&rlp_encoded).unwrap();
            rlp_decoded == b
        }
    }
}
