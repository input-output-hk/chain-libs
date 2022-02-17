#[macro_use]
mod macros;
pub mod apply_block_tests;
pub mod certificate_tests;
pub mod discrimination_tests;
#[cfg(feature = "evm")]
pub mod evm_test_suite;
#[cfg(feature = "evm")]
mod evm_tests;
pub mod initial_funds_tests;
pub mod ledger_tests;
pub mod transaction_tests;
