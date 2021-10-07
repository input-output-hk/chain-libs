use chain_core::mempack::Readable;

use crate::{transaction::{Payload, SingleAccountBindingSignature, UnspecifiedAccountIdentifier}, vote::Weight};

#[derive(Debug, Clone)]
pub struct SetVotingPower {
    account_id: UnspecifiedAccountIdentifier,
    voting_power: Weight,
}

#[derive(Debug, Clone)]
pub struct SetVotingPowerProof(SingleAccountBindingSignature);

impl Payload for SetVotingPower {
    const HAS_AUTH: bool = true;
    const HAS_DATA: bool = true;
    type Auth = SetVotingPowerProof;

    fn payload_data(&self) -> crate::transaction::PayloadData<Self> {
        todo!()
    }

    fn payload_auth_data(auth: &Self::Auth) -> crate::transaction::PayloadAuthData<Self> {
        todo!()
    }

    fn payload_to_certificate_slice(p: crate::transaction::PayloadSlice<'_, Self>) -> Option<super::CertificateSlice<'_>> {
        todo!()
    }
}

impl Readable for SetVotingPower {
    fn read(buf: &mut chain_core::mempack::ReadBuf) -> Result<Self, chain_core::mempack::ReadError> {
        todo!()
    }
}

impl Readable for SetVotingPowerProof {
    fn read(buf: &mut chain_core::mempack::ReadBuf) -> Result<Self, chain_core::mempack::ReadError> {
        todo!()
    }
}
