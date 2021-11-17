#[cfg(feature = "evm")]
use super::Error;
#[cfg(feature = "evm")]
use crate::smartcontract::Contract;
#[cfg(feature = "evm")]
use chain_evm::{
    machine::{Config, Environment, VirtualMachine},
    state::{AccountTrie, Balance},
};

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Ledger {
    #[cfg(feature = "evm")]
    pub(crate) accounts: AccountTrie,
}

impl Ledger {
    pub fn new() -> Self {
        Default::default()
    }
    #[cfg(feature = "evm")]
    pub fn deploy_contract<'runtime>(
        &mut self,
        contract: Contract,
        config: &'runtime Config,
        environment: &'runtime Environment,
    ) -> Result<(), Error> {
        match contract {
            Contract::Create {
                caller: _,
                value: _,
                init_code: _,
                gas_limit: _,
                access_list: _,
            } => {
                //
                let _vm = self.virtual_machine(config, environment);

                todo!("execute the contract and update ledger.evm.accounts");
            }
        }
    }

    #[cfg(feature = "evm")]
    pub fn virtual_machine<'runtime>(
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
        let Ledger { accounts } = self;
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
