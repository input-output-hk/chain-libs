use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::str::FromStr;

use chain_evm::primitive_types::{H160, H256, U256};
use chain_evm::state::{Account, Trie};
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
    fn get_ledger(&self) -> Ledger {
        self.ledger.clone()
    }

    fn get_config(&self) -> EvmConfigParams {
        self.config.clone()
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
    fn try_apply_network(self, network: String) -> Result<Self, String> {
        println!("Network type: {}", network);
        match network.as_str() {
            "Berlin" => Ok(self.set_evm_config(EvmConfig::Berlin)),
            "Istanbul" => Ok(self.set_evm_config(EvmConfig::Istanbul)),
            "London" => unimplemented!(),
            network => Err(format!("Not known network type, {}", network)),
        }
    }

    fn try_apply_accounts<I>(mut self, iter: I) -> Result<Self, String>
    where
        I: Iterator<Item = (String, TestAccountState)>,
    {
        for (address, account) in iter {
            self = self.try_apply_account(address, account)?;
        }
        Ok(self)
    }

    fn try_apply_account(self, address: String, account: TestAccountState) -> Result<Self, String> {
        Ok(self.set_account(
            H160::from_str(&address).map_err(|_| "Can not parse address")?,
            account.try_into()?,
        ))
    }

    fn try_apply_block_header(mut self, block_header: TestBlockHeader) -> Result<Self, String> {
        self.config.environment.block_gas_limit =
            U256::from_str(&block_header.gas_limit).map_err(|_| "Can not parse gas limit")?;
        self.config.environment.block_number =
            U256::from_str(&block_header.number).map_err(|_| "Can not parse number")?;
        self.config.environment.block_timestamp =
            U256::from_str(&block_header.timestamp).map_err(|_| "Can not parse timestamp")?;
        self.config.environment.block_difficulty =
            U256::from_str(&block_header.difficulty).map_err(|_| "Can not parse difficulty")?;

        self.config
            .environment
            .block_hashes
            .push(H256::from_str(&block_header.hash).expect("Can not parse hash"));

        Ok(self)
    }

    fn try_apply_transaction(mut self, transaction: TestEvmTransaction) -> Result<Self, String> {
        Ok(self)
    }

    fn try_apply_block(mut self, block: TestBlock) -> Result<Self, String> {
        self = self.try_apply_block_header(block.block_header)?;
        Ok(self)
    }
}

#[derive(Deserialize)]
struct TestAccountState {
    balance: String,
    code: String,
    nonce: String,
    storage: BTreeMap<String, String>,
}

impl TryFrom<TestAccountState> for Account {
    type Error = &'static str;
    fn try_from(account: TestAccountState) -> Result<Self, Self::Error> {
        let mut storage = Trie::default();
        for (k, v) in account.storage {
            storage = storage.put(
                H256::from_str(&k).map_err(|_| "Can not parse account storage key")?,
                H256::from_str(&v).map_err(|_| "Can not parse account storage key")?,
            );
        }
        Ok(Self {
            nonce: U256::from_str(&account.nonce).map_err(|_| "Can not parse nonce")?,
            balance: U256::from_str(&account.balance).map_err(|_| "Can not parse balance")?,
            storage,
            code: hex::decode(
                account.code[0..2]
                    .eq("0x")
                    .then(|| account.code[2..].to_string())
                    .expect("Missing '0x' prefix for hex data"),
            )
            .map_err(|_| "Can not parse code")?,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestEvmTransaction {
    data: Vec<String>,
    gas_limit: Vec<String>,
    gas_price: String,
    nonce: String,
    r: String,
    s: String,
    v: String,
    sender: String,
    to: String,
    value: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestBlockHeader {
    bloom: String,
    coinbase: String,
    difficulty: String,
    extra_data: String,
    gas_limit: String,
    gas_used: String,
    hash: String,
    mix_hash: String,
    nonce: String,
    number: String,
    parent_hash: String,
    receipt_trie: String,
    state_root: String,
    timestamp: String,
    transactions_trie: String,
    uncle_hash: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestBlock {
    block_header: TestBlockHeader,
    transactions: Vec<TestEvmTransaction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestCase {
    pre: BTreeMap<String, TestAccountState>,
    network: String,
    post_state: BTreeMap<String, TestAccountState>,
}

fn run_test(path: &str) {
    let file = File::open(path).expect("Open file failed");
    let reader = BufReader::new(file);

    let test: BTreeMap<String, TestCase> =
        serde_json::from_reader(reader).expect("Parse test cases failed");

    for (test_name, test_case) in test {
        println!("\nRunning test: {} ...", test_name);

        println!("Setup initial test state");

        let mut evm_state_builder = EvmStateBuilder::new()
            .set_chain_id(U256::from_str("0xff").unwrap())
            .try_apply_network(test_case.network)
            .unwrap()
            .try_apply_accounts(test_case.pre.into_iter())
            .unwrap();

        println!("Applying state ...");

        // let ledger = evm_state_builder.get_ledger();
        // let config = evm_state_builder.get_config();

        println!("Check evm state ...");
    }
}

#[test]
fn vm_add_test() {
    run_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/add.json");
}
