use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, WriteError},
};
use ethereum::{
    util::enveloped, EIP1559TransactionMessage, EIP2930TransactionMessage,
    LegacyTransactionMessage, TransactionSignature, TransactionV2,
};
use ethereum_types::{H256, U256};
use rlp::{decode, Decodable, DecoderError, Encodable, Rlp, RlpStream};
use typed_bytes::ByteBuilder;

/// Wrapper type for `ethereum::TransactionV2`, which includes the `EIP1559Transaction`, `EIP2930Transaction`, `LegacyTransaction` variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EthereumTransaction {
    Legacy(LegacyTransactionMessage),
    EIP2930(EIP2930TransactionMessage),
    EIP1559(EIP1559TransactionMessage),
}

impl EthereumTransaction {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        let bytes = self.rlp_bytes();
        bb.u64(bytes.len() as u64).bytes(&bytes)
    }

    pub fn hash(&self) -> H256 {
        match self {
            Self::Legacy(tx) => tx.hash(),
            Self::EIP2930(tx) => tx.hash(),
            Self::EIP1559(tx) => tx.hash(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.rlp_bytes().freeze().to_vec()
    }

    /// Sign the current transaction given an H256-encoded secret key.
    ///
    /// Legacy transaction signature as specified in [EIP-155](https://eips.ethereum.org/EIPS/eip-155).
    pub fn sign(self, secret: &H256) -> EthereumSignedTransaction {
        let secret = {
            let mut sk: [u8; 32] = [0u8; 32];
            sk.copy_from_slice(&secret[0..]);
            secp256k1::SecretKey::from_slice(&sk).unwrap()
        };
        match self {
            Self::Legacy(tx) => {
                let sig = super::util::sign_transaction_hash(&tx.hash(), &secret).unwrap();
                let (recovery_id, sig_bytes) = sig.serialize_compact();
                let chain_id = tx.chain_id.unwrap();
                let signature = TransactionSignature::new(
                    recovery_id.to_i32() as u64 % 2 + chain_id * 2 + 35,
                    H256::from_slice(&sig_bytes[0..32]),
                    H256::from_slice(&sig_bytes[32..64]),
                )
                .unwrap();
                EthereumSignedTransaction(TransactionV2::Legacy(ethereum::LegacyTransaction {
                    nonce: tx.nonce,
                    gas_price: tx.gas_price,
                    gas_limit: tx.gas_limit,
                    action: tx.action,
                    value: tx.value,
                    input: tx.input,
                    signature,
                }))
            }
            Self::EIP2930(tx) => {
                let sig = super::util::sign_transaction_hash(&tx.hash(), &secret).unwrap();
                let (recovery_id, sig_bytes) = sig.serialize_compact();
                let signature = TransactionSignature::new(
                    recovery_id.to_i32() as u64,
                    H256::from_slice(&sig_bytes[0..32]),
                    H256::from_slice(&sig_bytes[32..64]),
                )
                .unwrap();
                EthereumSignedTransaction(TransactionV2::EIP2930(ethereum::EIP2930Transaction {
                    chain_id: tx.chain_id,
                    nonce: tx.nonce,
                    gas_price: tx.gas_price,
                    gas_limit: tx.gas_limit,
                    action: tx.action,
                    value: tx.value,
                    input: tx.input,
                    access_list: tx.access_list,
                    odd_y_parity: recovery_id.to_i32() != 0,
                    r: *signature.r(),
                    s: *signature.s(),
                }))
            }
            Self::EIP1559(tx) => {
                let sig = super::util::sign_transaction_hash(&tx.hash(), &secret).unwrap();
                let (recovery_id, sig_bytes) = sig.serialize_compact();
                let signature = TransactionSignature::new(
                    recovery_id.to_i32() as u64,
                    H256::from_slice(&sig_bytes[0..32]),
                    H256::from_slice(&sig_bytes[32..64]),
                )
                .unwrap();
                EthereumSignedTransaction(TransactionV2::EIP1559(ethereum::EIP1559Transaction {
                    chain_id: tx.chain_id,
                    nonce: tx.nonce,
                    max_priority_fee_per_gas: tx.max_priority_fee_per_gas,
                    max_fee_per_gas: tx.max_fee_per_gas,
                    gas_limit: tx.gas_limit,
                    action: tx.action,
                    value: tx.value,
                    input: tx.input,
                    access_list: tx.access_list,
                    odd_y_parity: recovery_id.to_i32() != 0,
                    r: *signature.r(),
                    s: *signature.s(),
                }))
            }
        }
    }
}

impl Decodable for EthereumTransaction {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        let slice = rlp.data()?;

        let first = *slice.get(0).ok_or(DecoderError::Custom("empty slice"))?;

        let item_count = rlp.item_count()?;

        if item_count == 6 {
            return Ok(Self::Legacy(LegacyTransactionMessage {
                nonce: rlp.val_at(0)?,
                gas_price: rlp.val_at(1)?,
                gas_limit: rlp.val_at(2)?,
                action: rlp.val_at(3)?,
                value: rlp.val_at(4)?,
                input: rlp.val_at(5)?,
                chain_id: None,
            }));
        }
        if item_count == 9 {
            let r = {
                let mut bytes = [0u8; 32];
                rlp.val_at::<U256>(7)?.to_big_endian(&mut bytes);
                H256::from(bytes)
            };
            let s = {
                let mut bytes = [0u8; 32];
                rlp.val_at::<U256>(8)?.to_big_endian(&mut bytes);
                H256::from(bytes)
            };
            if r == H256::zero() && s == H256::zero() {
                return Ok(Self::Legacy(LegacyTransactionMessage {
                    nonce: rlp.val_at(0)?,
                    gas_price: rlp.val_at(1)?,
                    gas_limit: rlp.val_at(2)?,
                    action: rlp.val_at(3)?,
                    value: rlp.val_at(4)?,
                    input: rlp.val_at(5)?,
                    chain_id: rlp.val_at(6)?,
                }));
            }
        }

        let rlp = Rlp::new(slice.get(1..).ok_or(DecoderError::Custom("no tx body"))?);

        if first == 1u8 {
            if rlp.item_count()? != 9 {
                return Err(DecoderError::RlpIncorrectListLen);
            }
            return Ok(Self::EIP2930(EIP2930TransactionMessage {
                chain_id: rlp.val_at(1)?,
                nonce: rlp.val_at(2)?,
                gas_price: rlp.val_at(3)?,
                gas_limit: rlp.val_at(4)?,
                action: rlp.val_at(5)?,
                value: rlp.val_at(6)?,
                input: rlp.val_at(7)?,
                access_list: rlp.list_at(8)?,
            }));
        }

        if first == 2u8 {
            if rlp.item_count()? != 10 {
                return Err(DecoderError::RlpIncorrectListLen);
            }
            return Ok(Self::EIP1559(EIP1559TransactionMessage {
                chain_id: rlp.val_at(0)?,
                nonce: rlp.val_at(1)?,
                max_priority_fee_per_gas: rlp.val_at(2)?,
                max_fee_per_gas: rlp.val_at(3)?,
                gas_limit: rlp.val_at(4)?,
                action: rlp.val_at(5)?,
                value: rlp.val_at(6)?,
                input: rlp.val_at(7)?,
                access_list: rlp.list_at(8)?,
            }));
        }

        Err(DecoderError::Custom("invalid tx type"))
    }
}

impl Encodable for EthereumTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        match self {
            Self::Legacy(tx) => tx.rlp_append(s),
            Self::EIP2930(tx) => enveloped(1, tx, s),
            Self::EIP1559(tx) => enveloped(2, tx, s),
        }
    }
}

impl Serialize for EthereumTransaction {
    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), WriteError> {
        let bytes = self.rlp_bytes();
        codec.put_be_u64(bytes.len() as u64)?;
        codec.put_bytes(&bytes)?;
        Ok(())
    }
}

impl Deserialize for EthereumTransaction {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let len = codec.get_be_u64()?;
        let rlp_bytes = codec.get_bytes(len as usize)?;
        decode(rlp_bytes.as_slice()).map_err(|e| ReadError::InvalidData(format!("{:?}", e)))
    }
}

/// Wrapper type for `ethereum::TransactionV2`, which includes the `EIP1559Transaction`, `EIP2930Transaction`, `LegacyTransaction` variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EthereumSignedTransaction(TransactionV2);

impl EthereumSignedTransaction {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        let bytes = self.rlp_bytes();
        bb.u64(bytes.len() as u64).bytes(&bytes)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.rlp_bytes().freeze().to_vec()
    }
}

impl Decodable for EthereumSignedTransaction {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        let tx = TransactionV2::decode(rlp)?;
        Ok(EthereumSignedTransaction(tx))
    }
}

impl Encodable for EthereumSignedTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        self.0.rlp_append(s);
    }
}

impl Serialize for EthereumSignedTransaction {
    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), WriteError> {
        let bytes = self.rlp_bytes();
        codec.put_be_u64(bytes.len() as u64)?;
        codec.put_bytes(&bytes)?;
        Ok(())
    }
}

impl Deserialize for EthereumSignedTransaction {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let len = codec.get_be_u64()?;
        let rlp_bytes = codec.get_bytes(len as usize)?;
        decode(rlp_bytes.as_slice()).map_err(|e| ReadError::InvalidData(format!("{:?}", e)))
    }
}
