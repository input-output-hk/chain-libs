use crate::{accounting::account::AccountState, value::Value};

impl AccountState<()> {
    pub fn should_use_full_withdrawal(&self) -> bool {
        self.get_counter() == std::u32::MAX
    }

    /// Sub is correct if value to subs is smaller than account balance,
    /// If counter == Max then only full withwdrawal is allowed
    pub fn validate_sub(&self, value: Value) -> bool {
        if self.should_use_full_withdrawal() {
            self.value() == value
        } else {
            self.value() >= value
        }
    }
}
