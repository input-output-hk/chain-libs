use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

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
    fn new() -> Self {
        Self {
            ledger: Default::default(),
            config: Default::default(),
        }
    }
}

impl EvmStateBuilder {
    fn set_evm_config(mut self, config: EvmConfig) -> Self {
        self.config.config = config;
        self
    }

    fn set_account(mut self, address: Address, account: Account) -> Self {
        self.ledger.accounts = self.ledger.accounts.put(address, account);
        self
    }

    fn set_gas_price(mut self, gas_price: U256) -> Self {
        self.config.environment.gas_price = gas_price;
        self
    }

    fn set_origin(mut self, origin: H160) -> Self {
        self.config.environment.origin = origin;
        self
    }

    fn set_chain_id(mut self, chain_id: U256) -> Self {
        self.config.environment.chain_id = chain_id;
        self
    }

    fn set_block_hashes(mut self, block_hashes: Vec<H256>) -> Self {
        self.config.environment.block_hashes = block_hashes;
        self
    }

    fn set_block_number(mut self, block_number: U256) -> Self {
        self.config.environment.block_number = block_number;
        self
    }

    fn set_block_coinbase(mut self, block_coinbase: H160) -> Self {
        self.config.environment.block_coinbase = block_coinbase;
        self
    }

    fn set_block_timestamp(mut self, block_timestamp: U256) -> Self {
        self.config.environment.block_timestamp = block_timestamp;
        self
    }

    fn set_block_difficulty(mut self, block_difficulty: U256) -> Self {
        self.config.environment.block_difficulty = block_difficulty;
        self
    }

    fn set_block_gas_limit(mut self, block_gas_limit: U256) -> Self {
        self.config.environment.block_gas_limit = block_gas_limit;
        self
    }

    fn set_block_base_fee_per_gas(mut self, block_base_fee_per_gas: U256) -> Self {
        self.config.environment.block_base_fee_per_gas = block_base_fee_per_gas;
        self
    }
}

impl EvmStateBuilder {
    fn apply_test_evn(mut self, env: TestEnv) -> Self {
        self = self.set_block_base_fee_per_gas(
            U256::from_str(&env.current_base_fee).expect("Can not parse base fee"),
        );
        self = self.set_block_coinbase(
            H160::from_str(&env.current_coinbase).expect("Can not parse coinbase"),
        );
        self = self.set_block_difficulty(
            U256::from_str(&env.current_difficulty).expect("Can not parse difficulty"),
        );
        self = self.set_block_gas_limit(
            U256::from_str(&env.current_gas_limit).expect("Can not parse gas limit"),
        );
        self = self.set_block_number(
            U256::from_str(&env.current_number).expect("Can not parse block number"),
        );
        self = self.set_block_timestamp(
            U256::from_str(&env.current_timestamp).expect("Can not parse timestamp"),
        );
        self = self.set_block_hashes(vec![
            H256::from_str(&env.previous_hash).expect("Can not parse previous hash")
        ]);
        self
    }

    fn apply_test_account(self, address: String, account: TestAccountState) -> Self {
        let account = Account {
            nonce: U256::from_str(&account.nonce).expect("Can not parse nonce"),
            balance: U256::from_str(&account.balance).expect("Can not parse balance"),
            storage: account
                .storage
                .iter()
                .map(|(k, v)| {
                    (
                        H256::from_str(k).expect("Can not parse account storage key"),
                        H256::from_str(v).expect("Can not parse account storage key"),
                    )
                })
                .collect(),
            code: hex::decode(
                account.code[0..2]
                    .eq("0x")
                    .then(|| account.code[2..].to_string())
                    .expect("Missing '0x' prefix for hex data"),
            )
            .expect("Can not parse code"),
        };
        self.set_account(
            H160::from_str(&address).expect("Can not parse address"),
            account,
        )
    }
}

#[derive(Deserialize)]
struct TestAccountState {
    balance: String,
    code: String,
    nonce: String,
    storage: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestEvmTransaction {
    data: Vec<String>,
    gas_limit: Vec<String>,
    gas_price: String,
    nonce: String,
    secret_key: String,
    sender: String,
    to: String,
    value: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestEnv {
    current_base_fee: String,
    current_coinbase: String,
    current_difficulty: String,
    current_gas_limit: String,
    current_number: String,
    current_timestamp: String,
    previous_hash: String,
}

#[derive(Deserialize)]
struct TestIndexes {
    data: u64,
    gas: u64,
    value: u64,
}

#[derive(Deserialize)]
struct TestResult {
    hash: String,
    indexes: TestIndexes,
    logs: String,
    txbytes: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TestExpect {
    berlin: Vec<TestResult>,
    london: Vec<TestResult>,
    istanbul: Vec<TestResult>,
}

#[derive(Deserialize)]
struct TestCase {
    pre: BTreeMap<String, TestAccountState>,
    post: TestExpect,
    env: TestEnv,
    transaction: TestEvmTransaction,
}

fn run_test(path: &str) {
    let file = File::open(path).expect("Open file failed");
    let reader = BufReader::new(file);

    let test: BTreeMap<String, TestCase> =
        serde_json::from_reader(reader).expect("Parse test cases failed");

    for (test_name, test_case) in test {
        println!("Test name: {}", test_name);

        println!("Setup initial test state");

        let mut evm_state_builder = EvmStateBuilder::new()
            .set_chain_id(U256::from_str("0xff").unwrap())
            .apply_test_evn(test_case.env);

        for (address, account) in test_case.pre {
            evm_state_builder = evm_state_builder.apply_test_account(address, account);
        }
    }
}

#[test]
fn vm_add_test() {
    run_test("../evm-tests/GeneralStateTests/VMTests/vmArithmeticTest/add.json");
}
