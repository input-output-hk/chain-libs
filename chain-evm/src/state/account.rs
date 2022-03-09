use crate::{
    machine::CappedU256,
    state::{storage::Storage, trie::Trie},
    Address,
};

use primitive_types::U256;

pub type Nonce = U256;

/// Ethereum account balance which uses the least 64 significant bits of the `U256` type.
pub type Balance = CappedU256;

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

    const MAX_SIZE: u64 = u64::MAX;

    #[test]
    fn capped_u256_zero() {
        assert_eq!(CappedU256::zero(), CappedU256(0u64));
    }

    #[test]
    fn capped_u256_checked_add() {
        let val = 100u64;
        assert_eq!(
            CappedU256::from(val).checked_add(U256::from(0u64)),
            Some(CappedU256(val))
        );
        assert_eq!(
            CappedU256::from(MAX_SIZE).checked_add(U256::from(1u64)),
            None
        );
    }

    #[test]
    fn capped_u256_checked_sub() {
        let val = 100u64;
        assert_eq!(
            CappedU256::from(val).checked_sub(U256::from(0u64)),
            Some(CappedU256(val))
        );
        assert_eq!(CappedU256::from(0u64).checked_sub(U256::from(1u64)), None);
    }

    #[test]
    fn capped_u256_can_never_use_more_than_64_bits() {
        // convert from u64
        assert_eq!(CappedU256::from(MAX_SIZE), CappedU256(MAX_SIZE));
        // try to convert from U256
        assert!(CappedU256::try_from(U256::from(MAX_SIZE)).is_ok());
        assert!(CappedU256::try_from(U256::from(MAX_SIZE) + U256::from(1_u64)).is_err());

        // Anything larger than the least significant 64 bits
        // returns error
        assert!(CappedU256::try_from(U256([0, 1, 0, 0])).is_err());
        assert!(CappedU256::try_from(U256([0, 0, 1, 0])).is_err());
        assert!(CappedU256::try_from(U256([0, 0, 0, 1])).is_err());
    }
}
