use crate::account::{self, LedgerError};
use crate::certificate::EvmMapping;
use crate::chaineval::HeaderContentEvalContext;
use crate::evm::EvmTransaction;
use crate::header::BlockDate;
use crate::key::Hash;
use crate::value::Value;
use crate::{account::Identifier as JorAddress, accounting::account::AccountState as JorAccount};
use chain_core::packer::Codec;
use chain_core::property::DeserializeFromSlice;
use chain_evm::ExitError;
use chain_evm::{
    machine::{
        transact_call, transact_create, transact_create2, BlockHash, BlockNumber, BlockTimestamp,
        Environment, EvmState, Log, VirtualMachine,
    },
    state::{Account as EvmAccount, LogsState},
    Address as EvmAddress,
};
use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error(
        "For the provided jormungandr account: {} or evm account: {} mapping is already exist", .0.to_string(), .1.to_string()
    )]
    ExistingMapping(JorAddress, EvmAddress),
    #[error("Canot map address: {0}")]
    CannotMap(#[from] LedgerError),
    #[error("EVM transaction error: {0}")]
    EvmTransaction(#[from] chain_evm::machine::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressMapping {
    evm_to_jor: Hamt<DefaultHasher, EvmAddress, JorAddress>,
    jor_to_evm: Hamt<DefaultHasher, JorAddress, EvmAddress>,
}

fn transfrom_evm_to_jor(evm_id: &EvmAddress) -> JorAddress {
    let mut data = [0u8; EvmAddress::len_bytes() + 3];
    data[0..3].copy_from_slice(b"evm");
    data[3..].copy_from_slice(evm_id.as_bytes());

    let hash = Hash::hash_bytes(&data);

    JorAddress::deserialize_from_slice(&mut Codec::new(hash.as_bytes())).unwrap()
}

impl AddressMapping {
    fn new() -> Self {
        Self {
            evm_to_jor: Hamt::new(),
            jor_to_evm: Hamt::new(),
        }
    }

    fn jor_address(&self, evm_id: &EvmAddress) -> JorAddress {
        match self.evm_to_jor.lookup(evm_id).cloned() {
            Some(jor_address) => jor_address,
            None => transfrom_evm_to_jor(evm_id),
        }
    }

    fn del_accounts(&mut self, jor_id: &JorAddress) {
        if let Some(evm_id) = self.jor_to_evm.lookup(jor_id) {
            self.evm_to_jor = self.evm_to_jor.remove(evm_id).unwrap();
            self.jor_to_evm = self.jor_to_evm.remove(jor_id).unwrap();
        }
    }

    fn map_accounts(
        mut self,
        jor_id: JorAddress,
        evm_id: EvmAddress,
        mut accounts: account::Ledger,
    ) -> Result<(account::Ledger, Self), Error> {
        let evm_to_jor = self
            .evm_to_jor
            .insert(evm_id, jor_id.clone())
            .map_err(|_| Error::ExistingMapping(jor_id.clone(), evm_id))?;
        let jor_to_evm = self
            .jor_to_evm
            .insert(jor_id.clone(), evm_id)
            .map_err(|_| Error::ExistingMapping(jor_id.clone(), evm_id))?;

        // should update and move account evm account state
        let old_jor_id = transfrom_evm_to_jor(&evm_id);
        accounts = accounts.evm_move_state(jor_id, &old_jor_id)?;

        self.evm_to_jor = evm_to_jor;
        self.jor_to_evm = jor_to_evm;
        Ok((accounts, self))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ledger {
    pub(crate) logs: LogsState,
    pub(crate) environment: Environment,
    pub(crate) current_epoch: BlockEpoch,
    pub(crate) address_mapping: AddressMapping,
}

impl Default for Ledger {
    fn default() -> Self {
        Ledger::new()
    }
}

struct EvmStateImpl<'a> {
    accounts: account::Ledger,
    evm: &'a mut Ledger,
}

impl<'a> EvmState for EvmStateImpl<'a> {
    fn environment(&self) -> &Environment {
        &self.evm.environment
    }

    fn account(&self, evm_address: &EvmAddress) -> Option<EvmAccount> {
        let jor_address = self.evm.address_mapping.jor_address(evm_address);
        match self.accounts.get_state(&jor_address) {
            Ok(account) => Some(EvmAccount {
                balance: account.value.0.into(),
                state: account.evm_state.clone(),
            }),
            Err(_) => None,
        }
    }

    fn contains(&self, evm_address: &EvmAddress) -> bool {
        let jor_address = self.evm.address_mapping.jor_address(evm_address);
        self.accounts.exists(&jor_address)
    }

    fn modify_account<F>(&mut self, address: EvmAddress, f: F) -> Result<(), ExitError>
    where
        F: FnOnce(EvmAccount) -> Option<EvmAccount>,
    {
        let address = self.evm.address_mapping.jor_address(&address);
        let account = self
            .accounts
            .get_state(&address)
            .cloned()
            .unwrap_or_else(|_| JorAccount::new(Value::zero(), ()));

        match f(EvmAccount {
            balance: account.value.0.into(),
            state: account.evm_state,
        }) {
            Some(account) => {
                self.accounts = self
                    .accounts
                    .evm_insert_or_update(
                        address,
                        Value(u64::from(account.balance)),
                        account.state,
                        (),
                    )
                    .map_err(|e| ExitError::Other(e.to_string().into()))?;
            }
            // remove account
            None => {
                self.evm.address_mapping.del_accounts(&address);
            }
        }
        Ok(())
    }

    fn update_logs(&mut self, block_hash: BlockHash, logs: Vec<Log>) {
        self.evm.logs.put(block_hash, logs);
    }
}

impl Ledger {
    pub fn apply_map_accounts(
        mut evm: Ledger,
        mut accounts: account::Ledger,
        mapping: &EvmMapping,
    ) -> Result<(account::Ledger, Ledger), Error> {
        (accounts, evm.address_mapping) = evm.address_mapping.map_accounts(
            mapping.account_id().clone(),
            *mapping.evm_address(),
            accounts,
        )?;

        Ok((accounts, evm))
    }

    pub fn run_transaction(
        mut evm: Ledger,
        accounts: account::Ledger,
        contract: EvmTransaction,
        config: chain_evm::Config,
    ) -> Result<(account::Ledger, Ledger), Error> {
        let config = config.into();
        let mut vm_state = EvmStateImpl {
            accounts,
            evm: &mut evm,
        };
        match contract {
            EvmTransaction::Create {
                caller,
                value,
                init_code,
                gas_limit,
                access_list,
            } => {
                let vm = VirtualMachine::new(&mut vm_state, &config, caller, gas_limit, true);
                transact_create(vm, value, init_code, access_list)?;
            }
            EvmTransaction::Create2 {
                caller,
                value,
                init_code,
                salt,
                gas_limit,
                access_list,
            } => {
                let vm = VirtualMachine::new(&mut vm_state, &config, caller, gas_limit, true);
                transact_create2(vm, value, init_code, salt, access_list)?;
            }
            EvmTransaction::Call {
                caller,
                address,
                value,
                data,
                gas_limit,
                access_list,
            } => {
                let vm = VirtualMachine::new(&mut vm_state, &config, caller, gas_limit, true);
                let _byte_code_msg = transact_call(vm, address, value, data, access_list)?;
            }
        }
        Ok((vm_state.accounts, evm))
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
        let next_hash: BlockHash = <[u8; 32]>::from(metadata.block_id).into();
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
            address_mapping: AddressMapping::new(),
        }
    }
}

impl Ledger {
    pub(crate) fn stats(&self) -> String {
        let Ledger {
            address_mapping, ..
        } = self;
        let mut res = "EVM accounts mapping".to_string();
        for (jor_id, evm_id) in address_mapping.jor_to_evm.iter() {
            res = format!(
                "{}\n jormungandr account: {}, evm account: {}",
                res, jor_id, evm_id
            );
        }
        res
    }

    pub(crate) fn info_eq(&self, other: &Self) -> String {
        format!(
            "evm: {}",
            (self.address_mapping == other.address_mapping
                && self.environment == other.environment
                && self.logs == other.logs)
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chain_crypto::{Ed25519, PublicKey};
    use chain_evm::state::{AccountState, Nonce};

    quickcheck! {
        fn address_transformation_test(evm_rand_seed: u64) -> bool {
            let evm_id = EvmAddress::from_low_u64_be(evm_rand_seed);

            transfrom_evm_to_jor(&evm_id);
            true
        }
    }

    #[test]
    fn address_mapping_test() {
        let mut address_mapping = AddressMapping::new();
        let mut accounts = account::Ledger::new();

        let evm_id1 = EvmAddress::from_low_u64_be(0);
        let jor_id1 = JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap());
        let evm_id2 = EvmAddress::from_low_u64_be(1);
        let jor_id2 = JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[1; 32]).unwrap());

        assert_ne!(address_mapping.jor_address(&evm_id1), jor_id1);
        assert_ne!(address_mapping.jor_address(&evm_id2), jor_id2);

        (accounts, address_mapping) = address_mapping
            .map_accounts(jor_id1.clone(), evm_id1, accounts)
            .unwrap();

        assert_eq!(address_mapping.jor_address(&evm_id1), jor_id1);
        assert_ne!(address_mapping.jor_address(&evm_id2), jor_id2);

        assert_eq!(
            address_mapping
                .clone()
                .map_accounts(jor_id1.clone(), evm_id1, accounts.clone()),
            Err(Error::ExistingMapping(jor_id1.clone(), evm_id1))
        );

        assert_eq!(
            address_mapping
                .clone()
                .map_accounts(jor_id2.clone(), evm_id1, accounts.clone()),
            Err(Error::ExistingMapping(jor_id2.clone(), evm_id1))
        );

        assert_eq!(
            address_mapping
                .clone()
                .map_accounts(jor_id1.clone(), evm_id2, accounts.clone()),
            Err(Error::ExistingMapping(jor_id1.clone(), evm_id2))
        );

        (accounts, address_mapping) = address_mapping
            .map_accounts(jor_id2.clone(), evm_id2, accounts)
            .unwrap();

        assert_eq!(address_mapping.jor_address(&evm_id1), jor_id1);
        assert_eq!(address_mapping.jor_address(&evm_id2), jor_id2);

        address_mapping.del_accounts(&jor_id1);

        assert_ne!(address_mapping.jor_address(&evm_id1), jor_id1);
        assert_eq!(address_mapping.jor_address(&evm_id2), jor_id2);

        (_, address_mapping) = address_mapping
            .map_accounts(jor_id1.clone(), evm_id1, accounts)
            .unwrap();

        assert_eq!(address_mapping.jor_address(&evm_id1), jor_id1);
        assert_eq!(address_mapping.jor_address(&evm_id2), jor_id2);

        address_mapping.del_accounts(&jor_id1);
        address_mapping.del_accounts(&jor_id1);
        address_mapping.del_accounts(&jor_id2);
        address_mapping.del_accounts(&jor_id2);

        assert_ne!(address_mapping.jor_address(&evm_id1), jor_id1);
        assert_ne!(address_mapping.jor_address(&evm_id2), jor_id2);
    }

    #[test]
    fn apply_map_accounts_test_1() {
        // Prev state:
        // evm_mapping: [] (empty)
        // accounts: [] (empty)
        //
        // Applly 'mapping' ('account_id', 'evm_address')
        //
        // Post state;
        // evm_mapping: [ 'accountd_id' <-> 'evm_address' ]
        // accounts: [] (empty)

        let mapping = EvmMapping {
            account_id: JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap()),
            evm_address: EvmAddress::from_low_u64_be(0),
        };

        let mut evm = Ledger::new();
        let mut accounts = account::Ledger::new();

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Err(account::LedgerError::NonExistent)
        );

        assert_ne!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            mapping.account_id.clone()
        );

        (accounts, evm) = Ledger::apply_map_accounts(evm, accounts, &mapping).unwrap();

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Err(account::LedgerError::NonExistent)
        );

        assert_eq!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            mapping.account_id.clone()
        );
    }

    #[test]
    fn apply_map_accounts_test_2() {
        // Prev state:
        // evm_mapping: [] (empty)
        // accounts: [ transfrom_evm_to_jor('evm_address') <-> 'state` (state.evm_state != empty) ]
        //
        // Applly 'mapping' ('account_id', 'evm_address')
        //
        // Post state;
        // evm_mapping: [ 'accountd_id' <-> 'evm_address' ]
        // accounts: [ 'accountd_id' <-> 'state' ]

        let mapping = EvmMapping {
            account_id: JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap()),
            evm_address: EvmAddress::from_low_u64_be(0),
        };

        let value = Value(100);
        let evm_state = AccountState {
            storage: Default::default(),
            code: vec![0, 1, 2],
            nonce: Nonce::one(),
        };

        let mut evm = Ledger::new();
        let mut accounts = account::Ledger::new()
            .evm_insert_or_update(
                transfrom_evm_to_jor(&mapping.evm_address),
                value,
                evm_state.clone(),
                (),
            )
            .unwrap();

        assert_eq!(
            accounts.get_state(&transfrom_evm_to_jor(&mapping.evm_address)),
            Ok(&JorAccount::new_evm(evm_state.clone(), value, ()))
        );

        assert_ne!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            mapping.account_id.clone()
        );

        (accounts, evm) = Ledger::apply_map_accounts(evm, accounts, &mapping).unwrap();

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Ok(&JorAccount::new_evm(evm_state, value, ()))
        );

        assert_eq!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            mapping.account_id.clone()
        );
    }

    #[test]
    fn apply_map_accounts_test_3() {
        // Prev state:
        // evm_mapping: [] (empty)
        // accounts: [ transfrom_evm_to_jor('evm_address') <-> 'state1` (state1.evm_state != empty, state1.value = value1),
        //             'accountd_id' <-> 'state2' (state2.evm_state == empty, state2.value = value2) ]
        //
        // Applly 'mapping' ('accountd_id', 'evm_address')
        //
        // Post state;
        // evm_mapping: [ 'accountd_id' <-> 'evm_address' ]
        // accounts: ['accountd_id' <-> 'state3' (state3.value == state2.value + state1.value) ]

        let mapping = EvmMapping {
            account_id: JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap()),
            evm_address: EvmAddress::from_low_u64_be(0),
        };

        let value1 = Value(100);
        let value2 = Value(150);
        let evm_state = AccountState {
            storage: Default::default(),
            code: vec![0, 1, 2],
            nonce: Nonce::one(),
        };

        let mut evm = Ledger::new();
        let mut accounts = account::Ledger::new()
            .evm_insert_or_update(
                transfrom_evm_to_jor(&mapping.evm_address),
                value1,
                evm_state.clone(),
                (),
            )
            .unwrap()
            .add_account(mapping.account_id.clone(), value2, ())
            .unwrap();

        assert_eq!(
            accounts.get_state(&transfrom_evm_to_jor(&mapping.evm_address)),
            Ok(&JorAccount::new_evm(evm_state.clone(), value1, ()))
        );

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Ok(&JorAccount::new(value2, ()))
        );

        assert_ne!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            mapping.account_id.clone()
        );

        (accounts, evm) = Ledger::apply_map_accounts(evm, accounts, &mapping).unwrap();

        assert_eq!(
            accounts.get_state(&transfrom_evm_to_jor(&mapping.evm_address)),
            Err(account::LedgerError::NonExistent)
        );

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Ok(&JorAccount::new_evm(
                evm_state,
                value1.saturating_add(value2),
                ()
            ))
        );

        assert_eq!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            mapping.account_id.clone()
        );
    }

    #[test]
    fn apply_map_accounts_error_test_1() {
        // Prev state:
        // evm_mapping: [] (empty)
        // accounts: [ transfrom_evm_to_jor('evm_address') <-> 'state1` (state1.evm_state != empty),
        //             'accountd_id' <-> 'state2` (state2.evm_state != empty' ]
        //
        // Applly 'mapping' ('accountd_id', 'evm_address')
        //
        // Post state;
        // Error Error::CannotMap(account::LedgerError::AlreadyExists)

        let mapping = EvmMapping {
            account_id: JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap()),
            evm_address: EvmAddress::from_low_u64_be(0),
        };

        let value1 = Value(100);
        let value2 = Value(150);
        let evm_state1 = AccountState {
            storage: Default::default(),
            code: vec![0, 1, 2],
            nonce: Nonce::one(),
        };
        let evm_state2 = AccountState {
            storage: Default::default(),
            code: vec![3, 4, 5],
            nonce: Nonce::one(),
        };

        let evm = Ledger::new();
        let accounts = account::Ledger::new()
            .evm_insert_or_update(
                transfrom_evm_to_jor(&mapping.evm_address),
                value1,
                evm_state1.clone(),
                (),
            )
            .unwrap()
            .evm_insert_or_update(mapping.account_id.clone(), value2, evm_state2.clone(), ())
            .unwrap();

        assert_eq!(
            accounts.get_state(&transfrom_evm_to_jor(&mapping.evm_address)),
            Ok(&JorAccount::new_evm(evm_state1, value1, ()))
        );

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Ok(&JorAccount::new_evm(evm_state2, value2, ()))
        );

        assert_ne!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            mapping.account_id.clone()
        );

        assert_eq!(
            Ledger::apply_map_accounts(evm, accounts, &mapping),
            Err(Error::CannotMap(account::LedgerError::AlreadyExists))
        );
    }

    #[test]
    fn apply_map_accounts_error_test_2() {
        // Prev state:
        // evm_mapping: [ 'accountd_id' <-> 'evm_address1' ]
        // accounts: [] (empty)
        //
        // Applly 'mapping' ('accountd_id', 'evm_address2')
        //
        // Post state;
        // Error Error::ExistingMapping( 'accountd_id' , 'evm_address2' )

        let evm_address1 = EvmAddress::from_low_u64_be(0);
        let evm_address2 = EvmAddress::from_low_u64_be(1);
        let mapping = EvmMapping {
            account_id: JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap()),
            evm_address: evm_address2,
        };

        let mut evm = Ledger::new();
        let mut accounts = account::Ledger::new();
        (accounts, evm.address_mapping) = evm
            .address_mapping
            .map_accounts(mapping.account_id.clone(), evm_address2, accounts)
            .unwrap();

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Err(account::LedgerError::NonExistent)
        );

        assert_ne!(
            evm.address_mapping.jor_address(&evm_address1),
            mapping.account_id
        );

        assert_eq!(
            evm.address_mapping.jor_address(&evm_address2),
            mapping.account_id
        );

        assert_eq!(
            Ledger::apply_map_accounts(evm, accounts, &mapping),
            Err(Error::ExistingMapping(mapping.account_id, evm_address2))
        );
    }

    #[test]
    fn apply_map_accounts_error_test_3() {
        // Prev state:
        // evm_mapping: [ 'accountd_id1' <-> 'evm_address' ]
        // accounts: [] (empty)
        //
        // Applly 'mapping' ('accountd_id2', 'evm_address')
        //
        // Post state;
        // Error Error::ExistingMapping( 'accountd_id2' , 'evm_address' )

        let account_id1 = JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[0; 32]).unwrap());
        let account_id2 = JorAddress::from(<PublicKey<Ed25519>>::from_binary(&[1; 32]).unwrap());
        let mapping = EvmMapping {
            account_id: account_id2.clone(),
            evm_address: EvmAddress::from_low_u64_be(0),
        };

        let mut evm = Ledger::new();
        let mut accounts = account::Ledger::new();
        (accounts, evm.address_mapping) = evm
            .address_mapping
            .map_accounts(account_id1.clone(), mapping.evm_address, accounts)
            .unwrap();

        assert_eq!(
            accounts.get_state(&mapping.account_id),
            Err(account::LedgerError::NonExistent)
        );

        assert_eq!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            account_id1
        );

        assert_ne!(
            evm.address_mapping.jor_address(&mapping.evm_address),
            account_id2
        );

        assert_eq!(
            Ledger::apply_map_accounts(evm, accounts, &mapping),
            Err(Error::ExistingMapping(account_id2, mapping.evm_address))
        );
    }
}
