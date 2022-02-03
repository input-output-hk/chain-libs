use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::mem::size_of;
use std::str::FromStr;

use chain_evm::primitive_types::{H160, H256, U256};
use chain_evm::state::{Account, Trie};
use chain_evm::Address;

use crate::config::{EvmConfig, EvmConfigParams};
use crate::evm::EvmTransaction;
use crate::ledger::evm::Ledger;

struct TestEvmState {
    ledger: Ledger,
    config: EvmConfigParams,
}

impl TestEvmState {
    fn new() -> Self {
        Self {
            ledger: Default::default(),
            config: Default::default(),
        }
    }
}

impl TestEvmState {
    fn set_evm_config(mut self, config: EvmConfig) -> Self {
        self.config.config = config;
        self
    }

    fn set_account(mut self, address: Address, account: Account) -> Self {
        self.ledger.accounts = self.ledger.accounts.put(address, account);
        self
    }

    fn set_chain_id(mut self, chain_id: U256) -> Self {
        self.config.environment.chain_id = chain_id;
        self
    }
}

impl TestEvmState {
    fn try_apply_network(self, network: String) -> Result<Self, String> {
        println!("Network type: {}", network);
        match network.as_str() {
            "Berlin" => Ok(self.set_evm_config(EvmConfig::Berlin)),
            "Istanbul" => Ok(self.set_evm_config(EvmConfig::Istanbul)),
            "London" => unimplemented!(),
            network => Err(format!("Not known network type, {}", network)),
        }
    }

    fn try_apply_account(self, address: String, account: TestAccountState) -> Result<Self, String> {
        Ok(self.set_account(
            H160::from_str(&address).map_err(|_| "Can not parse address")?,
            account.try_into()?,
        ))
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

    fn try_apply_transaction(mut self, tx: TestEvmTransaction) -> Result<Self, String> {
        let nonce = U256::from_str(&tx.nonce).map_err(|_| "Can not parse transaction nonce")?;
        let gas_price = U256::from_str(&tx.gas_price)
            .map_err(|_| "Can not parse transaction gas limit")?
            .as_u64();
        let address = H160::from_str(&tx.to).map_err(|_| "Can not parse transaction to")?;

        // update account nonce
        self.ledger.accounts = self
            .ledger
            .accounts
            .modify_account(address, |account| Some(Account { nonce, ..account }));

        Ok(self)
    }

    fn try_apply_block(mut self, block: TestBlock) -> Result<Self, String> {
        self = self.try_apply_block_header(block.block_header)?;
        for transaction in block.transactions {
            self = self.try_apply_transaction(transaction)?;
        }

        Ok(self)
    }

    fn try_apply_blocks<I>(mut self, iter: I) -> Result<Self, String>
    where
        I: Iterator<Item = TestBlock>,
    {
        for block in iter {
            self = self.try_apply_block(block)?;
        }
        Ok(self)
    }
}

impl TestEvmState {
    fn validate_accounts<I>(&self, iter: I) -> Result<(), String>
    where
        I: Iterator<Item = (String, TestAccountState)>,
    {
        for (address, account) in iter {
            self.validate_account(address, account)?;
        }
        Ok(())
    }

    fn validate_account(
        &self,
        address: String,
        expected_state: TestAccountState,
    ) -> Result<(), String> {
        self.ledger
            .accounts
            .get(&H160::from_str(&address).map_err(|_| "Can not parse address")?)
            .ok_or("Can not find account")?
            .eq(&expected_state.try_into()?)
            .then(|| ())
            .ok_or("Account mismatch".to_string())
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
    type Error = String;
    fn try_from(account: TestAccountState) -> Result<Self, Self::Error> {
        let mut storage = Trie::default();
        for (k, v) in account.storage {
            let feel_zeros = |mut val: String| -> Result<String, String> {
                val = val[0..2]
                    .eq("0x")
                    .then(|| val[2..].to_string())
                    .ok_or("Missing '0x' prefix for hex data")?;

                while val.len() < size_of::<H256>() * 2 {
                    val = "00".to_string() + &val;
                }
                val = "0x".to_string() + &val;
                Ok(val)
            };
            storage = storage.put(
                H256::from_str(&feel_zeros(k)?).map_err(|_| "Can not parse account storage key")?,
                H256::from_str(&feel_zeros(v)?).map_err(|_| "Can not parse account storage key")?,
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
    data: String,
    gas_limit: String,
    gas_price: String,
    nonce: String,
    r: String,
    s: String,
    v: String,
    sender: String,
    to: String,
    value: String,
}

impl TryFrom<TestEvmTransaction> for EvmTransaction {
    type Error = String;
    fn try_from(tx: TestEvmTransaction) -> Result<Self, Self::Error> {
        let gas_limit = U256::from_str(&tx.gas_limit)
            .map_err(|_| "Can not parse transaction gas limit")?
            .as_u64();
        let value = U256::from_str(&tx.value).map_err(|_| "Can not parse transaction value")?;
        let data = hex::decode(tx.data).map_err(|_| "Can not parse transaction data")?;
        let caller = H160::from_str(&tx.sender).map_err(|_| "Can not parse transaction sender")?;
        let address = H160::from_str(&tx.to).map_err(|_| "Can not parse transaction to")?;

        Ok(Self::Call {
            address,
            gas_limit,
            value,
            data,
            caller,
            access_list: Vec::new(),
        })
    }
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
    genesis_block_header: TestBlockHeader,
    blocks: Vec<TestBlock>,
    post_state: BTreeMap<String, TestAccountState>,
}

fn run_test(path: &str) {
    let file = File::open(path).expect("Open file failed");
    let reader = BufReader::new(file);

    let test: BTreeMap<String, TestCase> =
        serde_json::from_reader(reader).expect("Parse test cases failed");

    for (test_name, test_case) in test {
        println!("\nRunning test: {} ...", test_name);
        let evm_state_builder = TestEvmState::new()
            .set_chain_id(U256::from_str("0xff").unwrap())
            .try_apply_network(test_case.network)
            .unwrap()
            .try_apply_accounts(test_case.pre.into_iter())
            .unwrap()
            .try_apply_block_header(test_case.genesis_block_header)
            .unwrap()
            .try_apply_blocks(test_case.blocks.into_iter())
            .unwrap();

        evm_state_builder
            .validate_accounts(test_case.post_state.into_iter())
            .unwrap();
    }
}

// TOOD: need fix this test
#[test]
fn vm_add_test() {
    run_test("../evm-tests/BlockchainTests/GeneralStateTests/VMTests/vmArithmeticTest/add.json");
}
