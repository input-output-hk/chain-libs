use crate::accounting::account::{DelegationRatio, DelegationType, DELEGATION_RATIO_MAX_DECLS};
use crate::certificate::CertificateSlice;
use crate::transaction::{
    AccountBindingSignature, Payload, PayloadAuthData, PayloadData, PayloadSlice,
    UnspecifiedAccountIdentifier,
};

use chain_core::property::WriteError;
use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize},
};
use std::marker::PhantomData;
use typed_bytes::{ByteArray, ByteBuilder};

/// A self delegation to a specific StakePoolId.
///
/// This structure is not sufficient to identify the owner, and instead we rely on a special
/// authenticated transaction, which has 1 input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStakeDelegation {
    pub delegation: DelegationType,
}

impl OwnerStakeDelegation {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.sub(|sb| serialize_delegation_type(&self.delegation, sb))
    }
    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }

    pub fn get_delegation_type(&self) -> &DelegationType {
        &self.delegation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    any(test, feature = "property-test-api"),
    derive(test_strategy::Arbitrary)
)]
pub struct StakeDelegation {
    pub account_id: UnspecifiedAccountIdentifier,
    pub delegation: DelegationType,
}

impl StakeDelegation {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.account_id.as_ref())
            .sub(|sb| serialize_delegation_type(&self.delegation, sb))
    }
    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }

    pub fn get_delegation_type(&self) -> &DelegationType {
        &self.delegation
    }
}

impl Serialize for OwnerStakeDelegation {
    fn serialized_size(&self) -> usize {
        let delegation_buf =
            serialize_delegation_type(&self.delegation, ByteBuilder::new()).finalize_as_vec();
        delegation_buf.len()
    }

    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), WriteError> {
        let delegation_buf =
            serialize_delegation_type(&self.delegation, ByteBuilder::new()).finalize_as_vec();
        codec.put_bytes(delegation_buf.as_slice())
    }
}

impl Deserialize for OwnerStakeDelegation {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let delegation = deserialize_delegation_type(codec)?;
        Ok(Self { delegation })
    }
}

impl Payload for OwnerStakeDelegation {
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
        PayloadAuthData(Vec::with_capacity(0).into(), PhantomData)
    }
    fn payload_to_certificate_slice(p: PayloadSlice<'_, Self>) -> Option<CertificateSlice<'_>> {
        Some(CertificateSlice::from(p))
    }
}

impl Serialize for StakeDelegation {
    fn serialized_size(&self) -> usize {
        let delegation_buf =
            serialize_delegation_type(&self.delegation, ByteBuilder::new()).finalize_as_vec();
        self.account_id.as_ref().len() + delegation_buf.len()
    }

    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), WriteError> {
        let delegation_buf =
            serialize_delegation_type(&self.delegation, ByteBuilder::new()).finalize_as_vec();
        codec.put_bytes(self.account_id.as_ref())?;
        codec.put_bytes(delegation_buf.as_slice())
    }
}

impl Deserialize for StakeDelegation {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let account_identifier = <[u8; 32]>::deserialize(codec)?;
        let delegation = deserialize_delegation_type(codec)?;
        Ok(StakeDelegation {
            account_id: account_identifier.into(),
            delegation,
        })
    }
}

impl Payload for StakeDelegation {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true;
    type Auth = AccountBindingSignature;
    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }

    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self> {
        let bb = auth.serialize_in(ByteBuilder::new()).finalize_as_vec();
        PayloadAuthData(bb.into(), PhantomData)
    }
    fn payload_to_certificate_slice(p: PayloadSlice<'_, Self>) -> Option<CertificateSlice<'_>> {
        Some(CertificateSlice::from(p))
    }
}

// Format is either:
// 0 (byte)
// 1 (byte)     POOL_ID (32 bytes)
// PARTS (byte) #POOLS (bytes) [ POOL_PART (1 byte) POOL_ID (32 bytes)] (repeated #POOLS time)
fn serialize_delegation_type(
    d: &DelegationType,
    bb: ByteBuilder<DelegationType>,
) -> ByteBuilder<DelegationType> {
    match d {
        DelegationType::NonDelegated => bb.u8(0),
        DelegationType::Full(pool_id) => bb.u8(1).bytes(pool_id.as_ref()),
        DelegationType::Ratio(ratio) => {
            let parts = ratio.parts();
            assert!(parts >= 2);
            bb.u8(parts)
                .iter8(ratio.pools().iter(), |b, (pool_id, pool_part)| {
                    b.u8(*pool_part).bytes(pool_id.as_ref())
                })
        }
    }
}

fn deserialize_delegation_type<R: std::io::Read>(
    codec: &mut Codec<R>,
) -> Result<DelegationType, ReadError> {
    let parts = codec.get_u8()?;
    match parts {
        0 => Ok(DelegationType::NonDelegated),
        1 => {
            let pool_id = <[u8; 32]>::deserialize(codec)?.into();
            Ok(DelegationType::Full(pool_id))
        }
        _ => {
            let sz = codec.get_u8()?;
            if sz as usize > DELEGATION_RATIO_MAX_DECLS {
                return Err(ReadError::SizeTooBig(
                    sz as usize,
                    DELEGATION_RATIO_MAX_DECLS,
                ));
            }
            let mut pools = Vec::with_capacity(sz as usize);
            for _ in 0..sz {
                let pool_parts = codec.get_u8()?;
                let pool_id = <[u8; 32]>::deserialize(codec)?.into();
                pools.push((pool_id, pool_parts))
            }
            match DelegationRatio::new(parts, pools) {
                None => Err(ReadError::StructureInvalid(
                    "invalid delegation ratio".to_string(),
                )),
                Some(dr) => Ok(DelegationType::Ratio(dr)),
            }
        }
    }
}
