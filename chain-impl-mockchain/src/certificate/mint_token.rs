use crate::{
    account::Identifier,
    certificate::CertificateSlice,
    tokens::{MintingPolicy, TokenIdentifier},
    transaction::{Payload, PayloadAuthData, PayloadData, PayloadSlice},
    value::Value,
};

use chain_core::mempack::{ReadBuf, ReadError, Readable};
use typed_bytes::ByteBuilder;

use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct MintToken {
    pub token: TokenIdentifier,
    pub policy: MintingPolicy,
    pub to: Identifier,
    pub value: Value,
}

impl MintToken {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(&self.token.bytes())
            .bytes(&self.policy.bytes())
            .bytes(self.to.as_ref().as_ref())
            .bytes(&self.value.bytes())
    }
}

impl Payload for MintToken {
    const HAS_DATA: bool = true;

    const HAS_AUTH: bool = false;

    type Auth = ();

    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }

    fn payload_auth_data(_: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(Box::new([]), PhantomData)
    }

    fn payload_to_certificate_slice(p: PayloadSlice<'_, Self>) -> Option<CertificateSlice<'_>> {
        Some(CertificateSlice::from(p))
    }
}

impl Readable for MintToken {
    fn read(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let token = TokenIdentifier::read(buf)?;
        let policy = MintingPolicy::read(buf)?;
        let to = Identifier::read(buf)?;
        let value = Value::read(buf)?;

        if policy.hash() != token.policy_hash {
            return Err(ReadError::InvalidData(
                "policy hash does not match".to_string(),
            ));
        }

        Ok(Self {
            token,
            policy,
            to,
            value,
        })
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for MintToken {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let token_name = Arbitrary::arbitrary(g);
            let policy = MintingPolicy::arbitrary(g);
            let token = TokenIdentifier {
                policy_hash: policy.hash(),
                token_name,
            };
            let to = Arbitrary::arbitrary(g);
            let value = Arbitrary::arbitrary(g);
            Self {
                token,
                policy,
                to,
                value,
            }
        }
    }
}
