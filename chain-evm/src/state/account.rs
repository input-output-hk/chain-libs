use crate::state::{storage::Storage, trie::Trie};
use crate::Address;

use primitive_types::U256;

pub type Nonce = U256;
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd)]
pub struct Balance(U256);

impl std::fmt::Display for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<U256> for Balance {
    type Error = &'static str;
    fn try_from(value: U256) -> Result<Self, Self::Error> {
        if value > U256::from(u64::max_value()) {
            Err("Balance values cannot exceed 64 significant bits")
        } else {
            Ok(Balance(value))
        }
    }
}

impl TryFrom<Balance> for u64 {
    type Error = &'static str;
    fn try_from(value: Balance) -> Result<Self, Self::Error> {
        if value > Balance(U256::from(u64::max_value())) {
            Err("Balance values cannot exceed 64 significant bits")
        } else {
            Ok(value.0.as_u64())
        }
    }
}

impl From<Balance> for U256 {
    fn from(other: Balance) -> U256 {
        other.0
    }
}

impl Balance {
    pub fn zero() -> Self {
        Balance(U256::zero())
    }
    pub fn checked_sub(self, other: U256) -> Option<Balance> {
        self.0.checked_sub(other).map(Balance)
    }
}

/// Smart-contract bytecode, such as the one compiled from Solidity code, for example.
pub type ByteCode = Vec<u8>;

/// A represantation of an EVM account.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Account {
    /// Account nonce. A number of value transfers from this account.
    pub nonce: Nonce,
    /// Account balance.
    pub balance: Balance,
    /// Account data storage.
    pub storage: Storage,
    /// EVM bytecode of this account.
    pub code: ByteCode,
}

impl Account {
    pub fn is_empty(&self) -> bool {
        self.nonce == Nonce::zero() && self.balance == Balance::zero() && self.storage.is_empty()
    }
}

/// In-memory representation of all accounts.
pub type AccountTrie = Trie<Address, Account>;

impl AccountTrie {
    /// Modify account
    ///
    /// If the element is not present, the closure F is apllied to the Default::default() value,
    /// otherwise the closure F is applied to the found element.
    /// If the closure returns None, then the key is deleted
    pub fn modify_account<F>(self, address: Address, f: F) -> Self
    where
        F: FnOnce(Account) -> Option<Account>,
    {
        let account = match self.get(&address) {
            Some(account) => account.clone(),
            None => Default::default(),
        };

        match f(account) {
            Some(account) => self.put(address, account),
            None => self.remove(&address),
        }
    }
}
