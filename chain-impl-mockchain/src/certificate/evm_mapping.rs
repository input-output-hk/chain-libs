use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};
#[cfg(feature = "evm")]
use chain_evm::Address;
use typed_bytes::{ByteArray, ByteBuilder};

use crate::transaction::{
    Payload, PayloadAuthData, PayloadData, PayloadSlice, SingleAccountBindingSignature,
};

use super::CertificateSlice;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmMapping {
    #[cfg(feature = "evm")]
    evm_address: Address,
}

impl EvmMapping {
    #[cfg(feature = "evm")]
    pub fn new(evm_address: Address) -> Self {
        Self { evm_address }
    }

    #[cfg(feature = "evm")]
    pub fn evm_address(&self) -> &Address {
        &self.evm_address
    }

    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        #[cfg(feature = "evm")]
        {
            bb.bytes(self.evm_address.as_bytes())
        }
        #[cfg(not(feature = "evm"))]
        bb
    }

    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }
}

/* Auth/Payload ************************************************************* */

impl Payload for EvmMapping {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true;
    type Auth = SingleAccountBindingSignature;

    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            std::marker::PhantomData,
        )
    }

    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self> {
        let bb = ByteBuilder::<Self>::new()
            .bytes(auth.as_ref())
            .finalize_as_vec();
        PayloadAuthData(bb.into(), std::marker::PhantomData)
    }

    fn payload_to_certificate_slice(p: PayloadSlice<'_, Self>) -> Option<CertificateSlice<'_>> {
        Some(CertificateSlice::from(p))
    }
}

/* Ser/De ******************************************************************* */

impl property::Serialize for EvmMapping {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, _writer: W) -> Result<(), Self::Error> {
        #[cfg(feature = "evm")]
        {
            let mut codec = chain_core::packer::Codec::new(_writer);
            codec.put_bytes(self.evm_address.as_bytes())?;
        }
        Ok(())
    }
}

impl property::Deserialize for EvmMapping {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(_reader: R) -> Result<Self, Self::Error> {
        #[cfg(feature = "evm")]
        {
            let mut codec = chain_core::packer::Codec::new(_reader);
            let bytes = codec.get_bytes(Address::len_bytes())?;
            Ok(Self {
                evm_address: Address::from_slice(bytes.as_slice()),
            })
        }
        #[cfg(not(feature = "evm"))]
        unimplemented!()
    }
}

impl Readable for EvmMapping {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        #[cfg(feature = "evm")]
        {
            Ok(Self {
                evm_address: Address::from_slice(buf.get_slice(Address::len_bytes())?),
            })
        }
        #[cfg(not(feature = "evm"))]
        unimplemented!()
    }
}
