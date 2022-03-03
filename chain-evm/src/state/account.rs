use crate::state::{storage::Storage, trie::Trie, Error as StateError};
use crate::Address;

use primitive_types::U256;

pub type Nonce = U256;

/// Ethereum account balance which uses the least 64 significant bits of the `U256` type.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd)]
pub struct Balance(U256);

impl std::fmt::Display for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<U256> for Balance {
    type Error = StateError;
    fn try_from(other: U256) -> Result<Self, Self::Error> {
        match other {
            U256([_, 0, 0, 0]) => Ok(Balance(other)),
            _ => Err(StateError::BalanceOverflow),
        }
    }
}

impl TryFrom<Balance> for u64 {
    type Error = StateError;
    fn try_from(other: Balance) -> Result<Self, Self::Error> {
        match other {
            Balance(U256([value, 0, 0, 0])) => Ok(value),
            _ => Err(StateError::BalanceOverflow),
        }
    }
}

impl From<u64> for Balance {
    fn from(other: u64) -> Self {
        Balance(U256([other, 0, 0, 0]))
    }
}

impl From<Balance> for U256 {
    fn from(other: Balance) -> U256 {
        other.0
    }
}

impl Balance {
    /// Zero (additive identity) of this type.
    pub fn zero() -> Self {
        Balance(U256::zero())
    }
    /// Checked substraction of `U256` types. Returns `Some(balance)` or `None` if overflow
    /// occurred.
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn account_balance_zero() {
        assert_eq!(Balance::zero(), Balance(U256([0, 0, 0, 0])));
    }

    #[test]
    fn account_balance_checked_sub() {
        assert_eq!(
            Balance::from(100u64).checked_sub(U256::from(0u64)),
            Some(Balance(U256([100, 0, 0, 0])))
        );
        assert_eq!(Balance::from(0u64).checked_sub(U256::from(1u64)), None);
    }

    #[test]
    fn account_balance_can_never_use_more_than_64_bits() {
        // convert from u64
        assert_eq!(Balance::from(u64::MAX), Balance(U256([u64::MAX, 0, 0, 0])));
        // try to convert from U256
        assert!(Balance::try_from(U256::from(u64::MAX)).is_ok());
        assert!(Balance::try_from(U256::from(u64::MAX) + U256::from(1_u64)).is_err());

        // Anything larger than the least significant 64 bits
        // returns error
        assert!(Balance::try_from(U256([0, 1, 0, 0])).is_err());
        assert!(Balance::try_from(U256([0, 0, 1, 0])).is_err());
        assert!(Balance::try_from(U256([0, 0, 0, 1])).is_err());
    }
}
