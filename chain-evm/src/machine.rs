//! # The Virtual Machine
//!
//! Abstractions for the EVM runtime.
//!
//! This module contains types that abstract away implementation details from the `evm` crate,
//! specially those involving node and stack configurations, and runtime execution.
//!
//! ## Handler <- EVM Context Handler
//! ## StackState<'config>
//!

use std::collections::BTreeMap;
use std::rc::Rc;

use evm::{
    backend::{Apply, ApplyBackend, Backend, Basic, Log},
    executor::{MemoryStackState, PrecompileFn, StackExecutor, StackSubstateMetadata},
    Context, Runtime,
};
use primitive_types::{H160, H256, U256};

use crate::state::AccountTrie;

/// Export EVM types
pub use evm::{backend::MemoryVicinity as Environment, Config};

/// A block's chain ID.
pub type ChainId = U256;

/// A block hash.
pub type BlockHash = H256;

/// A block hash.
pub type BlockHashes = Vec<BlockHash>;

/// A block's number.
pub type BlockNumber = U256;

/// A block's timestamp.
pub type BlockTimestamp = U256;

/// A block's difficulty.
pub type BlockDifficulty = U256;

/// A block's gas limit.
pub type BlockGasLimit = U256;

/// A block's origin
pub type Origin = H160;

/// A block's coinbase
pub type BlockCoinBase = H160;

/// Gas quantity integer for EVM operations.
pub type Gas = U256;

/// Gas price integer for EVM operations.
pub type GasPrice = U256;

/// Gas limit for EVM operations.
pub type GasLimit = U256;

/// Integer of the value sent with an EVM transaction.
pub type Value = U256;

/// The context of the EVM runtime
pub type RuntimeContext = Context;

type Precompiles = BTreeMap<H160, PrecompileFn>;

/// Top-level abstraction for the EVM with the
/// necessary types used to get the runtime going.
pub struct VirtualMachine<'runtime> {
    /// EVM Block Configuration.
    config: &'runtime Config,
    environment: &'runtime Environment,
    precompiles: Precompiles,
    state: AccountTrie,
    logs: Vec<Log>,
}

fn precompiles() -> Precompiles {
    // TODO: provide adequate precompiles
    Default::default()
}

impl<'runtime> VirtualMachine<'runtime> {
    /// Creates a new `VirtualMachine` given configuration parameters.
    pub fn new(config: &'runtime Config, environment: &'runtime Environment) -> Self {
        Self::new_with_state(config, environment, Default::default())
    }

    /// Creates a new `VirtualMachine` given configuration params and a given account storage.
    pub fn new_with_state(
        config: &'runtime Config,
        environment: &'runtime Environment,
        state: AccountTrie,
    ) -> Self {
        Self {
            config,
            environment,
            precompiles: precompiles(),
            state,
            logs: Default::default(),
        }
    }

    /// Returns an initialized instance of `evm::executor::StackExecutor`.
    pub fn executor(
        &self,
        gas_limit: u64,
    ) -> StackExecutor<'_, '_, MemoryStackState<'_, '_, VirtualMachine>, BTreeMap<H160, PrecompileFn>>
    {
        let metadata = StackSubstateMetadata::new(gas_limit, self.config);
        let memory_stack_state = MemoryStackState::new(metadata, self);
        StackExecutor::new_with_precompiles(memory_stack_state, self.config, &self.precompiles)
    }

    /// Returns an initialized instance of `evm::Runtime`.
    pub fn runtime(
        &self,
        code: Rc<Vec<u8>>,
        data: Rc<Vec<u8>>,
        context: RuntimeContext,
    ) -> Runtime<'_> {
        Runtime::new(code, data, context, self.config)
    }
}

impl<'runtime> Backend for VirtualMachine<'runtime> {
    fn gas_price(&self) -> U256 {
        self.environment.gas_price
    }
    fn origin(&self) -> H160 {
        self.environment.origin
    }
    fn block_hash(&self, number: U256) -> H256 {
        if number >= self.environment.block_number
            || self.environment.block_number - number - U256::one()
                >= U256::from(self.environment.block_hashes.len())
        {
            H256::default()
        } else {
            let index = (self.environment.block_number - number - U256::one()).as_usize();
            self.environment.block_hashes[index]
        }
    }
    fn block_number(&self) -> U256 {
        self.environment.block_number
    }
    fn block_coinbase(&self) -> H160 {
        self.environment.block_coinbase
    }
    fn block_timestamp(&self) -> U256 {
        self.environment.block_timestamp
    }
    fn block_difficulty(&self) -> U256 {
        self.environment.block_difficulty
    }
    fn block_gas_limit(&self) -> U256 {
        self.environment.block_gas_limit
    }
    fn block_base_fee_per_gas(&self) -> U256 {
        self.environment.block_base_fee_per_gas
    }
    fn chain_id(&self) -> U256 {
        self.environment.chain_id
    }
    fn exists(&self, address: H160) -> bool {
        self.state.contains(&address)
    }
    fn basic(&self, address: H160) -> Basic {
        self.state
            .get(&address)
            .map(|a| Basic {
                balance: a.balance,
                nonce: a.nonce,
            })
            .unwrap_or_default()
    }
    fn code(&self, address: H160) -> Vec<u8> {
        self.state
            .get(&address)
            .map(|val| val.code.to_vec())
            .unwrap_or_default()
    }
    fn storage(&self, address: H160, index: H256) -> H256 {
        self.state
            .get(&address)
            .map(|val| val.storage.get(&index).cloned().unwrap_or_default())
            .unwrap_or_default()
    }
    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        Some(self.storage(address, index))
    }
}

impl<'runtime> ApplyBackend for VirtualMachine<'runtime> {
    fn apply<A, I, L>(&mut self, values: A, logs: L, delete_empty: bool)
    where
        A: IntoIterator<Item = Apply<I>>,
        I: IntoIterator<Item = (H256, H256)>,
        L: IntoIterator<Item = Log>,
    {
        for apply in values {
            match apply {
                Apply::Modify {
                    address,
                    basic: Basic { balance, nonce },
                    code,
                    storage: apply_storage,
                    reset_storage,
                } => {
                    // get the account if stored, else use Default::default().
                    // Then, modify the account balance, nonce, and code.
                    // If reset_storage is set, the account's balance is
                    // set to be Default::default().
                    let mut account =
                        self.state
                            .modify_account(&address, balance, nonce, code, reset_storage);

                    // iterate over the apply_storage keys and values
                    // and put them into the account.
                    for (index, value) in apply_storage {
                        account.storage = if value == crate::state::Value::default() {
                            // value is full of zeroes, remove it
                            account.storage.clone().remove(&index)
                        } else {
                            account.storage.clone().put(index, value)
                        }
                    }

                    self.state = if delete_empty && account.is_empty() {
                        self.state.clone().remove(&address)
                    } else {
                        self.state.clone().put(address, account)
                    }
                }
                Apply::Delete { address } => {
                    self.state = self.state.clone().remove(&address);
                }
            }
        }

        // save the logs
        for log in logs {
            self.logs.push(log);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use evm::{Capture, ExitReason, ExitSucceed};

    use super::*;

    #[test]
    fn code_to_execute_evm_runtime_with_defaults_and_no_code_no_data() {
        let config = Config::istanbul();
        let environment = Environment {
            gas_price: Default::default(),
            origin: Default::default(),
            chain_id: Default::default(),
            block_hashes: Default::default(),
            block_number: Default::default(),
            block_coinbase: Default::default(),
            block_timestamp: Default::default(),
            block_difficulty: Default::default(),
            block_gas_limit: Default::default(),
            block_base_fee_per_gas: Default::default(),
        };

        let vm = VirtualMachine::new(&config, &environment);

        let gas_limit = u64::max_value();
        let mut executor = vm.executor(gas_limit);

        // Byte-encoded smart contract code
        let code = Rc::new(Vec::new());
        // Byte-encoded input data used for smart contract code
        let data = Rc::new(Vec::new());
        // EVM Context
        let context = RuntimeContext {
            address: Default::default(),
            caller: Default::default(),
            apparent_value: Default::default(),
        };
        // Handle for the EVM runtime
        let mut runtime = vm.runtime(code, data, context);

        let exit_reason = runtime.run(&mut executor);

        if let Capture::Exit(ExitReason::Succeed(ExitSucceed::Stopped)) = exit_reason {
            // We consume the executor to extract the stack state after executing
            // the code.
            let state = executor.into_state();
            // Next, we consume the stack state and extract the values and logs
            // used to modify the accounts trie in the VirtualMachine.
            let (values, logs) = state.deconstruct();

            // We assert that there are no values or logs from the code execution.
            assert_eq!(0, values.into_iter().count());
            assert_eq!(0, logs.into_iter().count());
            // // Here is where we would apply the changes in the backend
            // vm.apply(values, logs, true);
        } else {
            panic!("unexpected evm result");
        }
    }
}
