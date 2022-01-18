use chain_evm::primitive_types::{H160, H256, U256};
use chain_evm::state::Account;
use chain_evm::Address;

use crate::config::{EvmConfig, EvmConfigParams};
use crate::evm::EvmTransaction;
use crate::ledger::evm::Ledger;

struct EvmStateBuilder {
    ledger: Ledger,
    config: EvmConfigParams,
}

impl EvmStateBuilder {
    pub fn new() -> Self {
        Self {
            ledger: Default::default(),
            config: Default::default(),
        }
    }
}

impl EvmStateBuilder {
    pub fn set_evm_config(mut self, config: EvmConfig) -> Self {
        self.config.config = config;
        self
    }

    pub fn set_account(mut self, address: Address, account: Account) -> Self {
        self.ledger.accounts = self.ledger.accounts.put(address, account);
        self
    }

    pub fn set_gas_price(mut self, gas_price: U256) -> Self {
        self.config.environment.gas_price = gas_price;
        self
    }

    pub fn set_origin(mut self, origin: H160) -> Self {
        self.config.environment.origin = origin;
        self
    }

    pub fn set_chain_id(mut self, chain_id: U256) -> Self {
        self.config.environment.chain_id = chain_id;
        self
    }

    pub fn set_block_hashes(mut self, block_hashes: Vec<H256>) -> Self {
        self.config.environment.block_hashes = block_hashes;
        self
    }

    pub fn set_block_number(mut self, block_number: U256) -> Self {
        self.config.environment.block_number = block_number;
        self
    }

    pub fn set_block_coinbase(mut self, block_coinbase: H160) -> Self {
        self.config.environment.block_coinbase = block_coinbase;
        self
    }

    pub fn set_block_timestamp(mut self, block_timestamp: U256) -> Self {
        self.config.environment.block_timestamp = block_timestamp;
        self
    }

    pub fn set_block_difficulty(mut self, block_difficulty: U256) -> Self {
        self.config.environment.block_difficulty = block_difficulty;
        self
    }

    pub fn set_block_gas_limit(mut self, block_gas_limit: U256) -> Self {
        self.config.environment.block_gas_limit = block_gas_limit;
        self
    }

    pub fn set_block_base_fee_per_gas(mut self, block_base_fee_per_gas: U256) -> Self {
        self.config.environment.block_base_fee_per_gas = block_base_fee_per_gas;
        self
    }
}
