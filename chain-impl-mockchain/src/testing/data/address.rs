use crate::{
    account::{Identifier, SpendingCounter},
    chaintypes::HeaderId,
    key::EitherEd25519SecretKey,
    testing::builders::make_witness,
    transaction::{Input, Output, TransactionAuthData, Witness},
    utxo::Entry,
    value::Value,
};
use chain_addr::{Address, AddressReadable, Discrimination, Kind, KindType};
use chain_crypto::{
    testing::TestCryptoGen, AsymmetricKey, Ed25519, Ed25519Extended, KeyPair, PublicKey,
};
use rand_core::RngCore;

use std::fmt::{self, Debug};

///
/// Struct is responsible for adding some code which makes converting into transaction input/output easily.
/// Also it held all needed information (private key, public key) which can construct witness for transaction.
///
#[derive(Clone)]
pub struct AddressData {
    pub private_key: EitherEd25519SecretKey,
    pub spending_counter: Option<SpendingCounter>,
    pub address: Address,
}

impl Debug for AddressData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AddressData")
            .field("public_key", &self.public_key())
            .field("spending_counter", &self.spending_counter)
            .field("address", &self.address)
            .finish()
    }
}

impl PartialEq for AddressData {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address
    }
}

impl AddressData {
    pub fn new(
        private_key: EitherEd25519SecretKey,
        spending_counter: Option<SpendingCounter>,
        address: Address,
    ) -> Self {
        AddressData {
            private_key,
            address,
            spending_counter,
        }
    }

    pub fn from_discrimination_and_kind_type(
        discrimination: Discrimination,
        kind: KindType,
    ) -> Self {
        match kind {
            KindType::Account => AddressData::account(discrimination),
            KindType::Single => AddressData::utxo(discrimination),
            KindType::Group => AddressData::delegation(discrimination),
            _ => panic!("not implemented yet"),
        }
    }

    pub fn utxo(discrimination: Discrimination) -> Self {
        let (sk, pk) = AddressData::generate_key_pair::<Ed25519Extended>().into_keys();
        let sk = EitherEd25519SecretKey::Extended(sk);
        let user_address = Address(discrimination, Kind::Single(pk));
        AddressData::new(sk, None, user_address)
    }

    pub fn account(discrimination: Discrimination) -> Self {
        AddressData::account_with_spending_counter(discrimination, 0u32)
    }

    pub fn account_with_spending_counter(
        discrimination: Discrimination,
        spending_counter: u32,
    ) -> Self {
        let (sk, pk) = AddressData::generate_key_pair::<Ed25519Extended>().into_keys();
        let sk = EitherEd25519SecretKey::Extended(sk);
        let user_address = Address(discrimination, Kind::Account(pk));
        AddressData::new(sk, Some(spending_counter.into()), user_address)
    }

    pub fn delegation(discrimination: Discrimination) -> Self {
        let (single_sk, single_pk) =
            AddressData::generate_key_pair::<Ed25519Extended>().into_keys();
        let (_delegation_sk, delegation_pk) =
            AddressData::generate_key_pair::<Ed25519Extended>().into_keys();

        let user_address = Address(discrimination, Kind::Group(single_pk, delegation_pk));
        let single_sk = EitherEd25519SecretKey::Extended(single_sk);
        AddressData::new(single_sk, None, user_address)
    }

    pub fn delegation_from(
        primary_address: &AddressData,
        delegation_address: &AddressData,
    ) -> Self {
        let single_sk = primary_address.private_key.clone();
        let single_pk = primary_address.public_key();
        let user_address = Address(
            primary_address.discrimination(),
            Kind::Group(single_pk, delegation_address.public_key()),
        );
        AddressData::new(single_sk, None, user_address)
    }

    pub fn delegation_for(address: &AddressData) -> Self {
        AddressData::delegation_from(&AddressData::delegation(address.discrimination()), address)
    }

    pub fn make_input(&self, value: Value, utxo: Option<Entry<Address>>) -> Input {
        match self.address.kind() {
            Kind::Account { .. } => Input::from_account_public_key(self.public_key(), value),
            Kind::Single { .. } | Kind::Group { .. } => {
                Input::from_utxo_entry(utxo.unwrap_or_else(|| {
                    panic!(
                        "invalid state, utxo should be Some if Kind not Account {:?}",
                        &self.address
                    )
                }))
            }
            Kind::Multisig { .. } => unimplemented!(),
        }
    }

    pub fn delegation_id(&self) -> Identifier {
        Identifier::from(self.delegation_key())
    }

    pub fn to_id(&self) -> Identifier {
        Identifier::from(self.public_key())
    }

    pub fn make_output(&self, value: Value) -> Output<Address> {
        Output::from_address(self.address.clone(), value)
    }

    pub fn public_key(&self) -> PublicKey<Ed25519> {
        match self.kind() {
            Kind::Account(key) => key,
            Kind::Group(key, _) => key,
            Kind::Single(key) => key,
            Kind::Multisig(_) => panic!("not yet implemented"),
        }
    }

    pub fn delegation_key(&self) -> PublicKey<Ed25519> {
        match self.kind() {
            Kind::Group(_, delegation_key) => delegation_key,
            Kind::Account(public_key) => public_key,
            _ => panic!("wrong kind of address to to get delegation key"),
        }
    }

    pub fn confirm_transaction(&self) {
        self.spending_counter.map(|sp| sp.increment().unwrap());
    }

    pub fn spending_counter(&self) -> Option<SpendingCounter> {
        self.spending_counter
    }

    pub fn private_key(&self) -> EitherEd25519SecretKey {
        self.private_key.clone()
    }

    pub fn kind(&self) -> Kind {
        self.address.kind().clone()
    }

    pub fn discrimination(&self) -> Discrimination {
        self.address.discrimination()
    }

    pub fn to_bech32_str(&self) -> String {
        let prefix = match self.discrimination() {
            Discrimination::Production => "ta",
            Discrimination::Test => "ca",
        };
        AddressReadable::from_address(prefix, &self.address).to_string()
    }

    pub fn generate_key_pair<A: AsymmetricKey>() -> KeyPair<A> {
        TestCryptoGen(0).keypair::<A>(rand_core::OsRng.next_u32())
    }

    pub fn delegation_for_account(
        other: AddressData,
        delegation_public_key: PublicKey<Ed25519>,
    ) -> Self {
        let user_address = Address(
            other.address.discrimination(),
            Kind::Group(other.public_key(), delegation_public_key),
        );
        AddressData::new(other.private_key, other.spending_counter, user_address)
    }

    pub fn make_witness<'a>(
        &mut self,
        block0_hash: &HeaderId,
        tad: TransactionAuthData<'a>,
    ) -> Witness {
        let witness = make_witness(block0_hash, &self, &tad.hash());
        self.confirm_transaction();
        witness
    }

    pub fn address(&self) -> Address {
        self.address.clone()
    }
}

impl Into<Address> for AddressData {
    fn into(self) -> Address {
        self.address
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AddressDataValue {
    pub address_data: AddressData,
    pub value: Value,
}

impl AddressDataValue {
    pub fn new(address_data: AddressData, value: Value) -> Self {
        AddressDataValue {
            address_data: address_data,
            value: value,
        }
    }

    pub fn utxo(discrimination: Discrimination, value: Value) -> Self {
        AddressDataValue::new(AddressData::utxo(discrimination), value)
    }

    pub fn account(discrimination: Discrimination, value: Value) -> Self {
        AddressDataValue::new(AddressData::account(discrimination), value)
    }

    pub fn account_with_spending_counter(
        discrimination: Discrimination,
        spending_counter: u32,
        value: Value,
    ) -> Self {
        let address_data =
            AddressData::account_with_spending_counter(discrimination, spending_counter);
        Self::new(address_data, value)
    }

    pub fn delegation(discrimination: Discrimination, value: Value) -> Self {
        AddressDataValue::new(AddressData::delegation(discrimination), value)
    }

    pub fn from_discrimination_and_kind_type(
        discrimination: Discrimination,
        kind: KindType,
        value: Value,
    ) -> Self {
        AddressDataValue::new(
            AddressData::from_discrimination_and_kind_type(discrimination, kind),
            value,
        )
    }

    pub fn to_id(&self) -> Identifier {
        self.address_data.to_id()
    }

    pub fn public_key(&self) -> PublicKey<Ed25519> {
        self.address_data.public_key()
    }

    pub fn private_key(&self) -> EitherEd25519SecretKey {
        self.address_data.private_key.clone()
    }

    pub fn make_input(&self, utxo: Option<Entry<Address>>) -> Input {
        self.make_input_with_value(utxo, self.value)
    }

    pub fn make_input_with_value(&self, utxo: Option<Entry<Address>>, value: Value) -> Input {
        self.address_data.make_input(value, utxo)
    }

    pub fn make_output(&self) -> Output<Address> {
        self.make_output_with_value(self.value)
    }

    pub fn make_output_with_value(&self, value: Value) -> Output<Address> {
        self.address_data.make_output(value)
    }

    pub fn increment_spending_counter(&mut self) {
        let counter: u32 = self.address_data.spending_counter.unwrap().into();
        self.address_data.spending_counter = Some((counter + 1u32).into());
    }
    pub fn make_witness<'a>(
        &mut self,
        block0_hash: &HeaderId,
        tad: TransactionAuthData<'a>,
    ) -> Witness {
        self.address_data.make_witness(block0_hash, tad)
    }

    pub fn confirm_transaction(&mut self) {
        self.address_data.confirm_transaction();
    }

    pub fn kind(&self) -> Kind {
        self.address_data.kind()
    }

    pub fn is_utxo(&self) -> bool {
        match self.kind() {
            Kind::Single { .. } | Kind::Group { .. } => true,
            _ => false,
        }
    }

    pub fn address(&self) -> Address {
        self.address_data.address.clone()
    }

    pub fn address_data(&self) -> AddressData {
        self.address_data.clone()
    }
}

impl Into<AddressData> for AddressDataValue {
    fn into(self) -> AddressData {
        self.address_data
    }
}
