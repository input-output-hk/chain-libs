#[cfg(feature = "evm")]
use super::Error;
#[cfg(feature = "evm")]
use crate::smartcontract::Contract;
#[cfg(feature = "evm")]
use chain_evm::{
    machine::{Config, Environment, ExitReason, Log, VirtualMachine},
    state::{AccountTrie, Balance},
};

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Ledger {
    #[cfg(feature = "evm")]
    pub(crate) accounts: AccountTrie,
    #[cfg(feature = "evm")]
    pub(crate) logs: Box<[Log]>,
}

impl Ledger {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "evm")]
            accounts: Default::default(),
            #[cfg(feature = "evm")]
            logs: Default::default(),
        }
    }
    #[cfg(feature = "evm")]
    pub fn deploy_contract<'runtime>(
        &mut self,
        contract: Contract,
        config: &'runtime Config,
        environment: &'runtime Environment,
    ) -> Result<ExitReason, Error> {
        let mut vm = self.virtual_machine(config, environment);
        match contract {
            Contract::Create {
                caller,
                value,
                init_code,
                gas_limit,
                access_list,
            } => {
                //
                let exit_reason =
                    vm.transact_create(caller, value, init_code, gas_limit, access_list, true);
                Ok(exit_reason)
            }
            Contract::Create2 {
                caller,
                value,
                init_code,
                salt,
                gas_limit,
                access_list,
            } => {
                let exit_reason = vm.transact_create2(
                    caller,
                    value,
                    init_code,
                    salt,
                    gas_limit,
                    access_list,
                    true,
                );

                Ok(exit_reason)
            }
            Contract::Call {
                caller,
                address,
                value,
                data,
                gas_limit,
                access_list,
            } => {
                let (exit_reason, _byte_code_msg) =
                    vm.transact_call(caller, address, value, data, gas_limit, access_list, true);
                Ok(exit_reason)
            }
        }
    }

    #[cfg(feature = "evm")]
    pub(crate) fn virtual_machine<'runtime>(
        &self,
        config: &'runtime Config,
        environment: &'runtime Environment,
    ) -> VirtualMachine<'runtime> {
        VirtualMachine::new_with_state(config, environment, self.accounts.clone())
    }
}

#[cfg(not(feature = "evm"))]
impl Ledger {
    pub(crate) fn stats(&self) -> Option<String> {
        None
    }

    pub(crate) fn info_eq(&self, _other: &Self) -> Option<String> {
        None
    }
}

#[cfg(feature = "evm")]
impl Ledger {
    pub(crate) fn stats(&self) -> Option<String> {
        let Ledger { accounts, .. } = self;
        let mut count = 0;
        let mut total = Balance::zero();
        for (_, account) in accounts {
            count += 1;
            total += account.balance;
        }
        Some(format!("EVM accounts: #{} Total={:?}", count, total))
    }

    pub(crate) fn info_eq(&self, other: &Self) -> Option<String> {
        Some(format!("evm: {}", self.accounts == other.accounts))
    }
}
