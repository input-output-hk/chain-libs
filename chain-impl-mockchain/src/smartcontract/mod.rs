//! EVM Smart Contract transactions

use chain_core::mempack::Readable;

use crate::transaction::Payload;

pub struct Deployment {
    _config: (),
    _input: (),
    _data: (),
    _bytecode: (),
}

impl Readable for Deployment {
    fn read(
        _buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        todo!();
    }
}

impl Payload for Deployment {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = false;
    type Auth = ();

    fn payload_data(&self) -> crate::transaction::PayloadData<Self> {
        todo!();
    }
    fn payload_auth_data(_auth: &Self::Auth) -> crate::transaction::PayloadAuthData<Self> {
        todo!();
    }
    fn payload_to_certificate_slice(
        _p: crate::transaction::PayloadSlice<'_, Self>,
    ) -> Option<crate::certificate::CertificateSlice<'_>> {
        todo!();
    }
}
