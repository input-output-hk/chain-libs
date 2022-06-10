#[cfg(feature = "evm")]
use crate::account::Identifier;
use crate::transaction::{
    Payload, PayloadAuthData, PayloadData, PayloadSlice, SingleAccountBindingSignature,
};
use chain_core::{
    packer::Codec,
    property::{DeserializeFromSlice, ReadError, Serialize, WriteError},
};
#[cfg(feature = "evm")]
use chain_evm::{
    crypto::{
        secp256k1::{Message, RecoverableSignature, RecoveryId},
        sha3::{Digest, Keccak256},
    },
    ethereum::TransactionSignature,
    ethereum_types::{H256, U256},
    rlp::{self, decode, Decodable, DecoderError, Encodable, Rlp, RlpStream},
    transaction::SIGNATURE_BYTES,
    util::{decode_h256_from_u256, sign_data_hash, Secret},
    Address, Error,
};
use typed_bytes::{ByteArray, ByteBuilder};

use super::CertificateSlice;

/// Represents a mapping between a Jormungandr account and an EVM account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvmMapping {
    #[cfg(feature = "evm")]
    pub account_id: Identifier,
    #[cfg(feature = "evm")]
    pub evm_address: Address,
}

impl EvmMapping {
    #[cfg(feature = "evm")]
    pub fn new(evm_address: Address, account_id: Identifier) -> Self {
        Self {
            account_id,
            evm_address,
        }
    }

    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        #[cfg(feature = "evm")]
        {
            bb.bytes(self.account_id.as_ref().as_ref())
                .bytes(self.evm_address.as_bytes())
        }
        #[cfg(not(feature = "evm"))]
        bb
    }

    pub fn serialize(&self) -> ByteArray<Self> {
        self.serialize_in(ByteBuilder::new()).finalize()
    }

    #[cfg(feature = "evm")]
    /// Returns the hash used for signing.
    pub fn signing_hash(&self) -> H256 {
        H256::from_slice(Keccak256::digest(&rlp::encode(self)).as_slice())
    }

    #[cfg(feature = "evm")]
    /// Returns the hash used for signing.
    pub fn sign(&self, secret: &H256) -> Result<SignedEvmMapping, Error> {
        let secret = Secret::from_hash(secret)?;
        let sig = sign_data_hash(&self.signing_hash(), &secret)?;
        let (recovery_id, sig_bytes) = sig.serialize_compact();
        let (r, s) = sig_bytes.split_at(SIGNATURE_BYTES);
        let signature = TransactionSignature::new(
            recovery_id.to_i32() as u64,
            H256::from_slice(r),
            H256::from_slice(s),
        )
        .ok_or(Error::InvalidSignature)?;
        Ok(SignedEvmMapping {
            evm_mapping: EvmMapping {
                account_id: self.account_id.clone(),
                evm_address: self.evm_address,
            },
            odd_y_parity: recovery_id.to_i32() != 0,
            r: *signature.r(),
            s: *signature.s(),
        })
    }
}

/* Auth/Payload ************************************************************* */


/* Ser/De ******************************************************************* */

impl Serialize for EvmMapping {
    fn serialized_size(&self) -> usize {
        #[allow(unused_mut)]
        let mut res = 0;
        #[cfg(feature = "evm")]
        {
            res += self.account_id.serialized_size() + self.evm_address.0.serialized_size();
        }
        res
    }

    fn serialize<W: std::io::Write>(&self, _codec: &mut Codec<W>) -> Result<(), WriteError> {
        #[cfg(feature = "evm")]
        {
            self.account_id.serialize(_codec)?;
            _codec.put_bytes(self.evm_address.as_bytes())?;
        }
        Ok(())
    }
}

impl DeserializeFromSlice for EvmMapping {
    fn deserialize_from_slice(_codec: &mut Codec<&[u8]>) -> Result<Self, ReadError> {
        #[cfg(feature = "evm")]
        {
            let account_id = Identifier::deserialize_from_slice(_codec)?;
            let evm_address = _codec.get_bytes(Address::len_bytes())?;

            Ok(Self {
                account_id,
                evm_address: Address::from_slice(evm_address.as_slice()),
            })
        }
        #[cfg(not(feature = "evm"))]
        Err(ReadError::IoError(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "evm transactions are not supported in this build",
        )))
    }
}

/* RLP en/de ******************************************************************* */

#[cfg(feature = "evm")]
impl Decodable for EvmMapping {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        if rlp.item_count()? == 2 {
            let account_id_h256 = decode_h256_from_u256(rlp.val_at::<U256>(0)?)?;
            let account_key = chain_crypto::PublicKey::<crate::account::AccountAlg>::from_binary(
                account_id_h256.as_fixed_bytes(),
            )
            .map_err(|_| DecoderError::Custom("invalid account identifier"))?;
            return Ok(Self {
                account_id: account_key.into(),
                evm_address: rlp.val_at(1)?,
            });
        }
        Err(DecoderError::Custom("invalid evm mapping type"))
    }
}

#[cfg(feature = "evm")]
impl Encodable for EvmMapping {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(2);
        s.append(&U256::from_big_endian(self.account_id.as_ref().as_ref()));
        s.append(&self.evm_address);
    }
}

/// Represents a signed mapping between a Jormungandr account and an EVM account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedEvmMapping {
    #[cfg(feature = "evm")]
    pub evm_mapping: EvmMapping,
    #[cfg(feature = "evm")]
    pub odd_y_parity: bool,
    #[cfg(feature = "evm")]
    pub r: H256,
    #[cfg(feature = "evm")]
    pub s: H256,
}

#[cfg(feature = "evm")]
impl From<&SignedEvmMapping> for EvmMapping {
    fn from(other: &SignedEvmMapping) -> Self {
        Self {
            account_id: other.evm_mapping.account_id.clone(),
            evm_address: other.evm_mapping.evm_address,
        }
    }
}

impl SignedEvmMapping {
    #[cfg(feature = "evm")]
    pub fn new(evm_address: Address, account_id: Identifier, secret: &H256) -> Result<Self, Error> {
        EvmMapping::new(evm_address, account_id).sign(secret)
    }
    #[cfg(feature = "evm")]
    /// Verifies that the signing key corresponds to the `evm_address`.
    pub fn verify(&self) -> Result<(), Error> {
        if self.recover()? == self.evm_mapping.evm_address {
            Ok(())
        } else {
            Err(Error::InvalidSignature)
        }
    }
    #[cfg(feature = "evm")]
    /// Returns the address used to sign this EVM mapping
    fn recover(&self) -> Result<Address, Error> {
        let recid = RecoveryId::from_i32(self.odd_y_parity as i32)?;
        let data = {
            let r = self.r.as_fixed_bytes();
            let s = self.s.as_fixed_bytes();
            let mut data = [0u8; 64];
            data[..SIGNATURE_BYTES].copy_from_slice(&r[..]);
            data[SIGNATURE_BYTES..].copy_from_slice(&s[..]);
            data
        };
        let signature = RecoverableSignature::from_compact(&data, recid)?;
        let tx_hash = self.evm_mapping.signing_hash();
        let msg = Message::from_slice(tx_hash.as_fixed_bytes())?;
        let pubkey = signature.recover(&msg)?;
        let pubkey_bytes = pubkey.serialize_uncompressed();
        Ok(Address::from_slice(
            &Keccak256::digest(&pubkey_bytes[1..]).as_slice()[12..],
        ))
    }

    #[cfg(feature = "evm")]
    pub fn evm_address(&self) -> &Address {
        &self.evm_mapping.evm_address
    }

    #[cfg(feature = "evm")]
    pub fn account_id(&self) -> &Identifier {
        &self.evm_mapping.account_id
    }

    /// RLP-Encoded SignedEvmMapping bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        #[cfg(feature = "evm")]
        {
            self.rlp_bytes().freeze().to_vec()
        }
        #[cfg(not(feature = "evm"))]
        {
            Vec::new()
        }
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, DecoderError> {
        #[cfg(feature = "evm")]
        {
            let rlp = Rlp::new(data);
            Self::decode(&rlp)
        }
        #[cfg(not(feature = "evm"))]
        {
            Err(DecoderError::Custom(
                "evm transactions are not supported in this build",
            ))
        }
    }
}

/* Auth/Payload ************************************************************* */

impl Payload for SignedEvmMapping {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true;
    type Auth = SingleAccountBindingSignature;

    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(self.to_bytes().into(), std::marker::PhantomData)
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

impl Serialize for SignedEvmMapping {
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

impl DeserializeFromSlice for SignedEvmMapping {
    fn deserialize_from_slice(_codec: &mut Codec<&[u8]>) -> Result<Self, ReadError> {
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

/* RLP en/de ******************************************************************* */

#[cfg(feature = "evm")]
impl Decodable for SignedEvmMapping {
    fn decode(rlp: &Rlp<'_>) -> Result<Self, DecoderError> {
        if rlp.item_count()? == 5 {
            let account_id_h256 = decode_h256_from_u256(rlp.val_at::<U256>(0)?)?;
            let account_key = chain_crypto::PublicKey::<crate::account::AccountAlg>::from_binary(
                account_id_h256.as_fixed_bytes(),
            )
            .map_err(|_| DecoderError::Custom("invalid account identifier"))?;
            return Ok(Self {
                evm_mapping: EvmMapping {
                    account_id: account_key.into(),
                    evm_address: rlp.val_at(1)?,
                },
                odd_y_parity: rlp.val_at(2)?,
                r: decode_h256_from_u256(rlp.val_at::<U256>(3)?)?,
                s: decode_h256_from_u256(rlp.val_at::<U256>(4)?)?,
            });
        }
        Err(DecoderError::Custom("invalid signed evm mapping type"))
    }
}

#[cfg(feature = "evm")]
impl Encodable for SignedEvmMapping {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(5);
        s.append(&U256::from_big_endian(
            self.evm_mapping.account_id.as_ref().as_ref(),
        ));
        s.append(&self.evm_mapping.evm_address);
        s.append(&self.odd_y_parity);
        s.append(&U256::from_big_endian(&self.r[..]));
        s.append(&U256::from_big_endian(&self.s[..]));
    }
}

#[cfg(all(any(test, feature = "property-test-api"), feature = "evm"))]
mod test {
    use super::*;
    use quickcheck::Arbitrary;

    impl Arbitrary for EvmMapping {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self {
                account_id: Arbitrary::arbitrary(g),
                evm_address: [u8::arbitrary(g); Address::len_bytes()].into(),
            }
        }
    }

    impl Arbitrary for SignedEvmMapping {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Self {
                evm_mapping: Arbitrary::arbitrary(g),
                odd_y_parity: Arbitrary::arbitrary(g),
                r: [u8::arbitrary(g); 32].into(),
                s: [u8::arbitrary(g); 32].into(),
            }
        }
    }

    quickcheck! {
        fn evm_mapping_serialization_bijection(b: EvmMapping) -> bool {
            let bytes = b.serialize_in(ByteBuilder::new()).finalize_as_vec();
            let decoded = EvmMapping::deserialize_from_slice(&mut Codec::new(bytes.as_slice())).unwrap();
            decoded == b
        }
    }

    quickcheck! {
        fn evm_mapping_rlp_bijection(b: EvmMapping) -> bool {
            let encoded = b.rlp_bytes();
            let decoded = EvmMapping::decode(&Rlp::new(&encoded[..])).unwrap();
            decoded == b
        }
    }

    quickcheck! {
        fn signed_evm_mapping_rlp_bijection(b: SignedEvmMapping) -> bool {
            let encoded = b.rlp_bytes();
            let decoded = SignedEvmMapping::decode(&Rlp::new(&encoded[..])).unwrap();
            decoded == b
        }
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod prop_impl {
    use proptest::prelude::*;

    #[cfg(feature = "evm")]
    use crate::account::Identifier;
    use crate::certificate::evm_mapping::SignedEvmMapping;
    use crate::certificate::EvmMapping;
    #[cfg(feature = "evm")]
    use chain_evm::{ethereum_types::H256, Address};
    #[cfg(feature = "evm")]
    use proptest::{arbitrary::StrategyFor, strategy::Map};

    impl Arbitrary for EvmMapping {
        type Parameters = ();

        #[cfg(not(feature = "evm"))]
        type Strategy = Just<Self>;
        #[cfg(not(feature = "evm"))]
        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            Just(Self {})
        }

        #[cfg(feature = "evm")]
        type Strategy =
            Map<StrategyFor<(Identifier, [u8; 20])>, fn((Identifier, [u8; 20])) -> Self>;

        #[cfg(feature = "evm")]
        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<(Identifier, [u8; 20])>().prop_map(|(account_id, evm_address)| Self {
                account_id,
                evm_address: Address::from_slice(&evm_address),
            })
        }
    }

    impl Arbitrary for SignedEvmMapping {
        type Parameters = ();

        #[cfg(not(feature = "evm"))]
        type Strategy = Just<Self>;
        #[cfg(not(feature = "evm"))]
        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            Just(Self {})
        }

        #[cfg(feature = "evm")]
        type Strategy = Map<
            StrategyFor<(EvmMapping, bool, [u8; 32], [u8; 32])>,
            fn((EvmMapping, bool, [u8; 32], [u8; 32])) -> Self,
        >;

        #[cfg(feature = "evm")]
        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<(EvmMapping, bool, [u8; 32], [u8; 32])>().prop_map(
                |(evm_mapping, odd_y_parity, r, s)| Self {
                    evm_mapping,
                    odd_y_parity,
                    r: H256::from_slice(&r),
                    s: H256::from_slice(&s),
                },
            )
        }
    }
}
