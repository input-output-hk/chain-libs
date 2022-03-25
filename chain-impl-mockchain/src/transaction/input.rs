use super::utxo::UtxoPointer;
use crate::account::Identifier;
use crate::fragment::FragmentId;
use crate::key::SpendingPublicKey;
use crate::utxo::Entry;
use crate::value::Value;
use crate::{account, multisig};
use chain_addr::Address;
use chain_core::{
    packer::Codec,
    property::{Deserialize, ReadError, Serialize, SerializedSize, WriteError},
};
use chain_crypto::PublicKey;

pub const INPUT_SIZE: usize = 41;

pub const INPUT_PTR_SIZE: usize = 32;

/// This is either an single account or a multisig account depending on the witness type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnspecifiedAccountIdentifier([u8; INPUT_PTR_SIZE]);

impl UnspecifiedAccountIdentifier {
    pub fn to_single_account(&self) -> Option<account::Identifier> {
        PublicKey::from_binary(&self.0).map(|x| x.into()).ok()
    }
    pub fn to_multi_account(&self) -> multisig::Identifier {
        multisig::Identifier::from(self.0)
    }

    pub fn from_single_account(identifier: account::Identifier) -> Self {
        let mut buf = [0u8; INPUT_PTR_SIZE];
        let pk: PublicKey<account::AccountAlg> = identifier.into();
        buf.copy_from_slice(pk.as_ref());
        UnspecifiedAccountIdentifier(buf)
    }

    pub fn from_multi_account(identifier: multisig::Identifier) -> Self {
        let mut buf = [0u8; INPUT_PTR_SIZE];
        buf.copy_from_slice(identifier.as_ref());
        UnspecifiedAccountIdentifier(buf)
    }
}

impl AsRef<[u8]> for UnspecifiedAccountIdentifier {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; INPUT_PTR_SIZE]> for UnspecifiedAccountIdentifier {
    fn from(v: [u8; INPUT_PTR_SIZE]) -> Self {
        UnspecifiedAccountIdentifier(v)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccountIdentifier {
    Single(account::Identifier),
    Multi(multisig::Identifier),
}

/// Generalized input which have a specific input value, and
/// either contains an account reference or a TransactionSignDataHash+index
///
/// This uniquely refer to a specific source of value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Input {
    index_or_account: u8,
    value: Value,
    input_ptr: [u8; INPUT_PTR_SIZE],
}

pub enum InputType {
    Utxo,
    Account,
}

impl From<UnspecifiedAccountIdentifier> for [u8; INPUT_PTR_SIZE] {
    fn from(v: UnspecifiedAccountIdentifier) -> Self {
        v.0
    }
}

pub enum InputEnum {
    AccountInput(UnspecifiedAccountIdentifier, Value),
    UtxoInput(UtxoPointer),
}

impl From<[u8; INPUT_SIZE]> for Input {
    fn from(data: [u8; INPUT_SIZE]) -> Input {
        let index_or_account = data[0];
        let value = Value::try_from(&data[1..9]).unwrap();
        let mut input_ptr = [0u8; INPUT_PTR_SIZE];
        input_ptr.copy_from_slice(&data[9..]);
        Input::new(index_or_account, value, input_ptr)
    }
}

impl Input {
    pub fn bytes(&self) -> [u8; INPUT_SIZE] {
        let mut out = [0u8; INPUT_SIZE];
        out[0] = self.index_or_account;
        out[1..9].copy_from_slice(&self.value.0.to_be_bytes());
        out[9..].copy_from_slice(&self.input_ptr);
        out
    }

    pub fn new(index_or_account: u8, value: Value, input_ptr: [u8; INPUT_PTR_SIZE]) -> Self {
        Input {
            index_or_account,
            value,
            input_ptr,
        }
    }

    pub fn value(&self) -> Value {
        self.value
    }

    pub fn get_type(&self) -> InputType {
        if self.index_or_account == 0xff {
            InputType::Account
        } else {
            InputType::Utxo
        }
    }

    pub fn from_utxo(utxo_pointer: UtxoPointer) -> Self {
        let mut input_ptr = [0u8; INPUT_PTR_SIZE];
        input_ptr.clone_from_slice(utxo_pointer.transaction_id.as_ref());
        Input {
            index_or_account: utxo_pointer.output_index,
            value: utxo_pointer.value,
            input_ptr,
        }
    }

    pub fn from_utxo_entry(utxo_entry: Entry<Address>) -> Self {
        let mut input_ptr = [0u8; INPUT_PTR_SIZE];
        input_ptr.clone_from_slice(utxo_entry.fragment_id.as_ref());
        Input {
            index_or_account: utxo_entry.output_index,
            value: utxo_entry.output.value,
            input_ptr,
        }
    }

    pub fn from_account_public_key(public_key: SpendingPublicKey, value: Value) -> Self {
        Input::from_account(
            UnspecifiedAccountIdentifier::from_single_account(Identifier::from(public_key)),
            value,
        )
    }

    pub fn from_account(id: UnspecifiedAccountIdentifier, value: Value) -> Self {
        let mut input_ptr = [0u8; INPUT_PTR_SIZE];
        input_ptr.copy_from_slice(&id.0);
        Input {
            index_or_account: 0xff,
            value,
            input_ptr,
        }
    }

    pub fn from_account_single(id: account::Identifier, value: Value) -> Self {
        let id = UnspecifiedAccountIdentifier::from_single_account(id);
        Input::from_account(id, value)
    }

    pub fn from_multisig_account(id: multisig::Identifier, value: Value) -> Self {
        let id = UnspecifiedAccountIdentifier::from_multi_account(id);
        Input::from_account(id, value)
    }

    pub fn to_enum(&self) -> InputEnum {
        match self.get_type() {
            InputType::Account => {
                let account_identifier = self.input_ptr;
                let id = UnspecifiedAccountIdentifier(account_identifier);
                InputEnum::AccountInput(id, self.value)
            }
            InputType::Utxo => InputEnum::UtxoInput(UtxoPointer::new(
                FragmentId::from(self.input_ptr),
                self.index_or_account,
                self.value,
            )),
        }
    }

    pub fn from_enum(ie: InputEnum) -> Input {
        match ie {
            InputEnum::AccountInput(id, value) => Self::from_account(id, value),
            InputEnum::UtxoInput(utxo_pointer) => Self::from_utxo(utxo_pointer),
        }
    }
}

impl SerializedSize for Input {
    fn serialized_size(&self) -> usize {
        self.index_or_account.serialized_size()
            + self.value.serialized_size()
            + self.input_ptr.serialized_size()
    }
}

impl Serialize for Input {
    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), WriteError> {
        codec.put_u8(self.index_or_account)?;
        self.value.serialize(codec)?;
        codec.put_bytes(&self.input_ptr)
    }
}

impl Deserialize for Input {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let index_or_account = codec.get_u8()?;
        let value = Value::deserialize(codec)?;
        let input_ptr = <[u8; INPUT_PTR_SIZE]>::deserialize(codec)?;
        Ok(Input {
            index_or_account,
            value,
            input_ptr,
        })
    }
}
