//! EVM transactions
use crate::{
    certificate::CertificateSlice,
    transaction::{Payload, PayloadAuthData, PayloadData},
};
use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, WriteError},
};
#[cfg(feature = "evm")]
use chain_evm::{
    ethereum_types::H256,
    machine::{AccessList, Address},
    rlp::{decode, Decodable, DecoderError, Encodable, Rlp, RlpStream},
    state::ByteCode,
};
use typed_bytes::ByteBuilder;

/// Variants of supported EVM action types
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvmActionType {
    #[cfg(feature = "evm")]
    Create { init_code: ByteCode },
    #[cfg(feature = "evm")]
    Create2 { init_code: ByteCode, salt: H256 },
    #[cfg(feature = "evm")]
    Call { address: Address, data: ByteCode },
}

/// Variants of supported EVM transactions
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvmTransaction {
    #[cfg(feature = "evm")]
    pub caller: Address,
    #[cfg(feature = "evm")]
    pub value: u64,
    #[cfg(feature = "evm")]
    pub gas_limit: u64,
    #[cfg(feature = "evm")]
    pub access_list: AccessList,
    pub action_type: EvmActionType,
}

#[cfg(feature = "evm")]
impl From<&EvmActionType> for u8 {
    fn from(other: &EvmActionType) -> Self {
        use EvmActionType::*;
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
        let caller = rlp.val_at(1)?;
        let value = rlp.val_at(2)?;
        let gas_limit = rlp.val_at(3)?;
        let access_list = rlp.list_at(4)?;
        match rlp.val_at(0)? {
            0u8 => Ok(EvmTransaction {
                caller,
                value,
                gas_limit,
                access_list,
                action_type: EvmActionType::Create {
                    init_code: rlp.list_at(5)?.into_boxed_slice(),
                },
            }),
            1u8 => Ok(EvmTransaction {
                caller,
                value,
                gas_limit,
                access_list,
                action_type: EvmActionType::Create2 {
                    init_code: rlp.list_at(5)?.into_boxed_slice(),
                    salt: rlp.val_at(6)?,
                },
            }),
            2u8 => Ok(EvmTransaction {
                caller,
                value,
                gas_limit,
                access_list,
                action_type: EvmActionType::Call {
                    address: rlp.val_at(5)?,
                    data: rlp.list_at(6)?.into_boxed_slice(),
                },
            }),
            _ => Err(DecoderError::Custom("invalid evm transaction")),
        }
    }
}

#[cfg(feature = "evm")]
impl Encodable for EvmTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        match &self.action_type {
            EvmActionType::Create { init_code } => {
                s.begin_list(6);
                s.append(&u8::from(&self.action_type));
                s.append(&self.caller);
                s.append(&self.value);
                s.append(&self.gas_limit);
                s.append_list(&self.access_list);
                s.append_list(init_code);
            }
            EvmActionType::Create2 { init_code, salt } => {
                s.begin_list(7);
                s.append(&u8::from(&self.action_type));
                s.append(&self.caller);
                s.append(&self.value);
                s.append(&self.gas_limit);
                s.append_list(&self.access_list);
                s.append_list(init_code);
                s.append(salt);
            }
            EvmActionType::Call { address, data } => {
                s.begin_list(7);
                s.append(&u8::from(&self.action_type));
                s.append(&self.caller);
                s.append(&self.value);
                s.append(&self.gas_limit);
                s.append_list(&self.access_list);
                s.append(address);
                s.append_list(data);
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

    impl Arbitrary for EvmActionType {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            match u8::arbitrary(g) % 3 {
                0 => Self::Create {
                    init_code: Box::new([Arbitrary::arbitrary(g); 32]),
                },
                1 => Self::Create2 {
                    init_code: Box::new([Arbitrary::arbitrary(g); 32]),
                    salt: [u8::arbitrary(g); H256::len_bytes()].into(),
                },
                2 => Self::Call {
                    address: [u8::arbitrary(g); H160::len_bytes()].into(),
                    data: Box::new([Arbitrary::arbitrary(g); 32]),
                },
                _ => unreachable!(),
            }
        }
    }

    impl Arbitrary for EvmTransaction {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let caller = [u8::arbitrary(g); H160::len_bytes()].into();
            let value = u64::arbitrary(g);
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
            Self {
                caller,
                value,
                gas_limit,
                access_list,
                action_type: Arbitrary::arbitrary(g),
            }
        }
    }

    quickcheck! {
        // this tests encoding/decoding using the Serialize/Deserialize traits
        // with RLP encoding under the hood
        fn evm_transaction_serialization_bijection(b: EvmTransaction) -> bool {
            let encoded = b.serialize_as_vec().unwrap();
            let decoded = EvmTransaction::deserialize(&mut Codec::new(encoded.as_slice())).unwrap();
            decoded == b
        }
    }
}
