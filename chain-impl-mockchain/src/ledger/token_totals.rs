use crate::{ledger::Error, tokens::identifier::TokenIdentifier, value::Value};
use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;

#[derive(Clone, PartialEq, Eq, Default)]
pub struct TokenTotals {
    data: Hamt<DefaultHasher, TokenIdentifier, Value>,
}

impl TokenTotals {
    #[must_use = "Does not modify the internal state"]
    pub fn add(&self, token: TokenIdentifier, value: Value) -> Result<TokenTotals, Error> {
        self.data
            .insert_or_update(token, value, |v| v.checked_add(value).map(Some))
            .map(|data| TokenTotals { data })
            .map_err(Into::into)
    }

    pub fn get_total(&self, token: &TokenIdentifier) -> Option<Value> {
        self.data.lookup(token).copied()
    }
}
