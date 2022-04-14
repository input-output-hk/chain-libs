use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, WriteError},
};
use ethereum::TransactionV2;
use rlp::{decode, Decodable, DecoderError, Encodable, Rlp, RlpStream};
use typed_bytes::ByteBuilder;

/// Wrapper type for `ethereum::TransactionV2`, which includes the `EIP1559Transaction`, `EIP2930Transaction`, `LegacyTransaction` variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EthereumTransaction(TransactionV2);

impl EthereumTransaction {
    /// Serialize the contract into a `ByteBuilder`.
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        let bytes = self.rlp_bytes();
        bb.u64(bytes.len() as u64).bytes(&bytes)
    }
}

impl Decodable for EthereumTransaction {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        let tx = TransactionV2::decode(rlp)?;
        Ok(EthereumTransaction(tx))
    }
}

impl Encodable for EthereumTransaction {
    fn rlp_append(&self, s: &mut RlpStream) {
        self.0.rlp_append(s);
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
