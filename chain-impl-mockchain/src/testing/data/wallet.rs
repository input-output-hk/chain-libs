use crate::{
    chaintypes::HeaderId,
    key::EitherEd25519SecretKey,
    testing::data::{AddressData, AddressDataValue},
    transaction::{Input, Output, TransactionAuthData, Witness},
    value::Value,
};
use chain_addr::{Address, Discrimination};
use chain_crypto::{Ed25519, PublicKey};

#[derive(Clone, Debug)]
pub struct Wallet {
    alias: String,
    account: AddressDataValue,
    related_utxos: Option<Vec<AddressDataValue>>,
}

impl Wallet {
    pub fn from_address_data_value(account: AddressDataValue) -> Self {
        Wallet {
            alias: "".to_owned(),
            account: account,
            related_utxos: None,
        }
    }

    pub fn from_value(initial_value: Value) -> Self {
        Wallet::new("", initial_value)
    }

    pub fn new(alias: &str, initial_value: Value) -> Self {
        Wallet {
            alias: alias.to_owned(),
            account: AddressDataValue::account(Discrimination::Test, initial_value),
            related_utxos: None,
        }
    }

    pub fn alias(&self) -> String {
        self.alias.clone()
    }

    pub fn value(&self) -> Value {
        self.account.value
    }

    pub fn public_key(&self) -> PublicKey<Ed25519> {
        self.account.public_key()
    }

    pub fn private_key(&self) -> EitherEd25519SecretKey {
        self.account.private_key()
    }

    pub fn make_output(&self) -> Output<Address> {
        self.account.make_output()
    }

    pub fn make_output_with_value(&self, value: &Value) -> Output<Address> {
        self.account.make_output_with_value(&value)
    }

    pub fn make_input_with_value(&self, value: &Value) -> Input {
        self.account.make_input_with_value(None, value)
    }

    pub fn as_account(&self) -> AddressDataValue {
        self.account.clone()
    }

    pub fn as_account_data(&self) -> AddressData {
        self.as_account().into()
    }

    pub fn as_address(&self) -> Address {
        self.as_account_data().address.clone()
    }

    pub fn confirm_transaction(&mut self) {
        self.account.increment_spending_counter();
    }

    pub fn make_witness<'a>(
        &mut self,
        block0_hash: &HeaderId,
        tad: TransactionAuthData<'a>,
    ) -> Witness {
        self.as_account().make_witness(block0_hash, tad)
    }
}
