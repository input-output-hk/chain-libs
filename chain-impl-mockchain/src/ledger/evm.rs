use crate::account::Identifier as JorAddress;
use crate::chaineval::HeaderContentEvalContext;
use crate::evm::EvmTransaction;
use crate::header::BlockDate;
use chain_evm::{
    machine::{BlockHash, BlockNumber, BlockTimestamp, Environment, EvmState, Log, VirtualMachine},
    primitive_types::{H256, U256},
    state::{Account as EvmAccount, AccountTrie, LogsState},
    Address as EvmAddress,
};
use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error(
        "for the provided jormungandr account: {} or evm account: {} mapping is already exist", .0.to_string(), .1.to_string()
    )]
    ExistedMapping(JorAddress, EvmAddress),
    #[error("evm transaction error: {0}")]
    EvmTransactionError(#[from] chain_evm::machine::Error),
}

#[derive(Clone, PartialEq, Eq)]
pub struct AddressMapping {
    evm_to_jor: Hamt<DefaultHasher, EvmAddress, JorAddress>,
    jor_to_evm: Hamt<DefaultHasher, JorAddress, EvmAddress>,
}

impl AddressMapping {
    fn new() -> Self {
        Self {
            evm_to_jor: Hamt::new(),
            jor_to_evm: Hamt::new(),
        }
    }

    #[allow(dead_code)]
    fn evm_address(&self, jor_id: JorAddress) -> Option<EvmAddress> {
        self.jor_to_evm.lookup(&jor_id).cloned()
    }

    fn jor_address(&self, evm_id: EvmAddress) -> Option<JorAddress> {
        self.evm_to_jor.lookup(&evm_id).cloned()
    }

    fn map_accounts(&mut self, jor_id: JorAddress, evm_id: EvmAddress) -> Result<(), Error> {
        (!self.evm_to_jor.contains_key(&evm_id) && !self.jor_to_evm.contains_key(&jor_id))
            .then(|| ())
            .ok_or_else(|| Error::ExistedMapping(jor_id.clone(), evm_id))?;

        self.evm_to_jor = self.evm_to_jor.insert(evm_id, jor_id.clone()).unwrap();
        self.jor_to_evm = self.jor_to_evm.insert(jor_id, evm_id).unwrap();
        Ok(())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Ledger {
    pub(crate) accounts: AccountTrie,
    pub(crate) logs: LogsState,
    pub(crate) environment: Environment,
    pub(crate) current_epoch: BlockEpoch,
    pub(crate) adress_mapping: AddressMapping,
}

impl Default for Ledger {
    fn default() -> Self {
        Ledger::new()
    }
}

impl EvmState for super::Ledger {
    fn environment(&self) -> &Environment {
        &self.evm.environment
    }

    fn account(&self, address: EvmAddress) -> Option<EvmAccount> {
        self.evm.accounts.get(&address).cloned()
    }

    fn contains(&self, address: EvmAddress) -> bool {
        self.evm.adress_mapping.jor_address(address).is_some()
    }

    fn modify_account<F>(&mut self, evm_address: EvmAddress, f: F)
    where
        F: FnOnce(EvmAccount) -> Option<EvmAccount>,
    {
        self.evm.accounts = self.evm.accounts.clone().modify_account(evm_address, f);
    }

    fn update_logs(&mut self, block_hash: H256, logs: Vec<Log>) {
        self.evm.logs.put(block_hash, logs);
    }
}

impl super::Ledger {
    pub fn map_accounts(mut self, jor: JorAddress, evm: EvmAddress) -> Result<Self, Error> {
        self.evm.adress_mapping.map_accounts(jor, evm)?;
        Ok(self)
    }

    pub fn run_transaction(mut self, contract: EvmTransaction) -> Result<Self, Error> {
        let evm_config = self.settings.evm_config;
        let mut vm = VirtualMachine::new(&mut self);
        match contract {
            EvmTransaction::Create {
                caller,
                value,
                init_code,
                gas_limit,
                access_list,
            } => {
                vm.transact_create(
                    evm_config,
                    caller,
                    value,
                    init_code,
                    gas_limit,
                    access_list,
                    true,
                )?;
            }
            EvmTransaction::Create2 {
                caller,
                value,
                init_code,
                salt,
                gas_limit,
                access_list,
            } => {
                vm.transact_create2(
                    evm_config,
                    caller,
                    value,
                    init_code,
                    salt,
                    gas_limit,
                    access_list,
                    true,
                )?;
            }
            EvmTransaction::Call {
                caller,
                address,
                value,
                data,
                gas_limit,
                access_list,
            } => {
                let _byte_code_msg = vm.transact_call(
                    evm_config,
                    caller,
                    address,
                    value,
                    data,
                    gas_limit,
                    access_list,
                    true,
                )?;
            }
        }
        Ok(self)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockEpoch {
    epoch: u32,
    epoch_start: BlockTimestamp,
    slots_per_epoch: u32,
    slot_duration: u8,
}

impl Ledger {
    /// Updates block values for EVM environment
    pub fn update_block_environment(
        &mut self,
        metadata: &HeaderContentEvalContext,
        slots_per_epoch: u32,
        slot_duration: u8,
    ) {
        // use content hash from the apply block as the EVM block hash
        let next_hash: BlockHash = <[u8; 32]>::from(metadata.content_hash).into();
        self.environment.block_hashes.insert(0, next_hash);
        self.environment.block_number = BlockNumber::from(self.environment.block_hashes.len());
        self.update_block_timestamp(metadata.block_date, slots_per_epoch, slot_duration);
    }
    /// Updates the block timestamp for EVM environment
    fn update_block_timestamp(
        &mut self,
        block_date: BlockDate,
        slots_per_epoch: u32,
        slot_duration: u8,
    ) {
        let BlockDate {
            epoch: this_epoch,
            slot_id,
        } = block_date;

        // update block epoch if needed
        if this_epoch > self.current_epoch.epoch {
            let BlockEpoch {
                epoch: _,
                epoch_start,
                slots_per_epoch: spe,
                slot_duration: sdur,
            } = self.current_epoch;
            let new_epoch = this_epoch;
            let new_epoch_start = epoch_start + spe as u64 * sdur as u64;
            self.current_epoch = BlockEpoch {
                epoch: new_epoch,
                epoch_start: new_epoch_start,
                slots_per_epoch,
                slot_duration,
            };
        }

        // calculate the U256 value from epoch and slot parameters
        let block_timestamp = self.current_epoch.epoch_start
            + slot_id as u64 * self.current_epoch.slot_duration as u64;
        // update EVM enviroment
        self.environment.block_timestamp = block_timestamp;
    }
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            accounts: Default::default(),
            logs: Default::default(),
            environment: Environment {
                gas_price: Default::default(),
                chain_id: Default::default(),
                block_hashes: Default::default(),
                block_number: Default::default(),
                block_coinbase: Default::default(),
                block_timestamp: Default::default(),
                block_difficulty: Default::default(),
                block_gas_limit: Default::default(),
                block_base_fee_per_gas: Default::default(),
            },
            current_epoch: BlockEpoch {
                epoch: 0,
                epoch_start: BlockTimestamp::default(),
                slots_per_epoch: 1,
                slot_duration: 10,
            },
            adress_mapping: AddressMapping::new(),
        }
    }
}

impl Ledger {
    pub(crate) fn stats(&self) -> String {
        let Ledger { accounts, .. } = self;
        let mut count = 0;
        let mut total = U256::zero();
        for (_, account) in accounts {
            count += 1;
            total += account.balance.into();
        }
        format!("EVM accounts: #{} Total={:?}", count, total)
    }

    pub(crate) fn info_eq(&self, other: &Self) -> String {
        format!("evm: {}", self.accounts == other.accounts)
    }
}

#[cfg(test)]
mod test {
    use chain_crypto::{Ed25519, PublicKey};

    use super::*;

    #[test]
    fn address_mapping_test() {
        let mut address_mapping = AddressMapping::new();

        let evm_id1 = EvmAddress::from_low_u64_be(0);
        let jor_id1 = JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap());
        let evm_id2 = EvmAddress::from_low_u64_be(1);
        let jor_id2 = JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[1; 32]).unwrap());

        assert_eq!(address_mapping.evm_address(jor_id1.clone()), None);
        assert_eq!(address_mapping.jor_address(evm_id1), None);
        assert_eq!(address_mapping.evm_address(jor_id2.clone()), None);
        assert_eq!(address_mapping.jor_address(evm_id2), None);

        assert_eq!(
            address_mapping.map_accounts(jor_id1.clone(), evm_id1),
            Ok(())
        );

        assert_eq!(address_mapping.evm_address(jor_id1.clone()), Some(evm_id1));
        assert_eq!(address_mapping.jor_address(evm_id1), Some(jor_id1.clone()));
        assert_eq!(address_mapping.evm_address(jor_id2.clone()), None);
        assert_eq!(address_mapping.jor_address(evm_id2), None);

        assert_eq!(
            address_mapping.map_accounts(jor_id1.clone(), evm_id1),
            Err(Error::ExistedMapping(jor_id1.clone(), evm_id1))
        );
        assert_eq!(
            address_mapping.map_accounts(jor_id2.clone(), evm_id1),
            Err(Error::ExistedMapping(jor_id2.clone(), evm_id1))
        );
        assert_eq!(
            address_mapping.map_accounts(jor_id1.clone(), evm_id2),
            Err(Error::ExistedMapping(jor_id1.clone(), evm_id2))
        );
        assert_eq!(
            address_mapping.map_accounts(jor_id2.clone(), evm_id2),
            Ok(())
        );

        assert_eq!(address_mapping.evm_address(jor_id1.clone()), Some(evm_id1));
        assert_eq!(address_mapping.jor_address(evm_id1), Some(jor_id1));
        assert_eq!(address_mapping.evm_address(jor_id2.clone()), Some(evm_id2));
        assert_eq!(address_mapping.jor_address(evm_id2), Some(jor_id2));
    }
}
