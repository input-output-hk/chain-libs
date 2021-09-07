//! EVM Smart Contract transactions

use chain_core::mempack::Readable;
#[cfg(feature = "evm")]
use chain_evm::{Config, Environment, GasLimit};

use crate::transaction::Payload;

pub enum Contract {
    #[cfg(feature = "evm")]
    EVM {
        _config: Config,
        _environment: Environment,
        _gas_limit: GasLimit,
        _input: Box<u8>,
        _data: Box<u8>,
        _bytecode: Box<u8>,
    },
}

impl Readable for Contract {
    fn read(
        _buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        todo!();
    }
}

impl Payload for Contract {
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
