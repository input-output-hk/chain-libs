use crate::transaction::{SingleAccountBindingSignature, TransactionBindingAuthData};
use crate::vote::CommitteeId;
use crate::{
    certificate::{CertificateSlice, VotePlanId},
    transaction::{Payload, PayloadAuthData, PayloadData, PayloadSlice},
};
use chain_core::{
    packer::Codec,
    property::{Deserialize, DeserializeFromSlice, ReadError, Serialize},
};
use chain_crypto::Verification;
use typed_bytes::{ByteArray, ByteBuilder};

#[derive(Debug, Clone)]
pub struct EncryptedVoteTallyProof {
    pub id: CommitteeId,
    pub signature: SingleAccountBindingSignature,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct EncryptedVoteTally {
    id: VotePlanId,
}

impl EncryptedVoteTallyProof {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.id.as_ref()).bytes(self.signature.as_ref())
    }

    pub fn verify<'a>(&self, verify_data: &TransactionBindingAuthData<'a>) -> Verification {
        let pk = self.id.public_key();
        self.signature.verify_slice(&pk, verify_data)
    }
}

impl EncryptedVoteTally {
    pub fn new(id: VotePlanId) -> Self {
        Self { id }
    }

    pub fn id(&self) -> &VotePlanId {
        &self.id
    }

    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.id().as_ref())
    }

    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }
}

/* Auth/Payload ************************************************************* */

impl Payload for EncryptedVoteTally {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true; // TODO: true it is the Committee signatures
    type Auth = EncryptedVoteTallyProof;

    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            std::marker::PhantomData,
        )
    }

    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(
            auth.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            std::marker::PhantomData,
        )
    }

    fn payload_to_certificate_slice(p: PayloadSlice<'_, Self>) -> Option<CertificateSlice<'_>> {
        Some(CertificateSlice::from(p))
    }
}

/* Ser/De ******************************************************************* */

impl Serialize for EncryptedVoteTally {
    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), std::io::Error> {
        codec.put_bytes(self.serialize().as_slice())
    }
}

impl DeserializeFromSlice for EncryptedVoteTallyProof {
    fn deserialize_from_slice(codec: &mut Codec<&[u8]>) -> Result<Self, ReadError> {
        let id = CommitteeId::deserialize_from_slice(codec)?;
        let signature = SingleAccountBindingSignature::deserialize_from_slice(codec)?;
        Ok(Self { id, signature })
    }
}

impl Deserialize for EncryptedVoteTally {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let id = <[u8; 32]>::deserialize(codec)?.into();
        Ok(Self { id })
    }
}
