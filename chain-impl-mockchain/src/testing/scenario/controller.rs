use crate::{
    testing::{
        data::{Wallet,StakePool,AddressData},
        builders::{
            build_stake_pool_registration_cert,
            build_stake_delegation_cert, 
            build_stake_pool_retirement_cert,
            build_stake_owner_delegation_cert,
            TransactionBuilder,
            TransactionCertBuilder,
        }
    },
    certificate::Certificate,
    ledger::{ledger::Ledger,Error as LedgerError},
    value::Value,
    fragment::Fragment,
    key::Hash,
    block::{HeaderContentEvalContext,ChainLength},
    date::BlockDate
};

use super::scenario_builder::{prepare_scenario,wallet};
use chain_addr::Discrimination;

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub ControllerError
        UnknownWallet { alias: String } = "cannot find wallet with alias {alias}",
        UnknownStakePool { alias: String } = "cannot find stake pool with alias {alias}",
}

pub struct Controller {
    pub block0_hash: Hash,
    pub declared_wallets: Vec<Wallet>,
    pub declared_stake_pools: Vec<StakePool>
}

impl Controller {

    pub fn wallet(&self,alias: &str) -> Result<Wallet,ControllerError> {
        self.declared_wallets
            .iter()
            .cloned()
            .find(|x| x.alias() == alias)
            .ok_or(ControllerError::UnknownWallet{
                alias: alias.to_owned()
            })
    }

    fn empty_context() -> HeaderContentEvalContext {
        HeaderContentEvalContext{
            block_date: BlockDate::first(),
            chain_length: ChainLength(0),
            nonce: None,
        }
    }

    pub fn stake_pool(&self,alias: &str) ->  Result<StakePool,ControllerError> {
        self.declared_stake_pools
            .iter()
            .cloned()
            .find(|x| x.alias() == alias)
            .ok_or(ControllerError::UnknownStakePool{
                alias: alias.to_owned()
            })
    }

    pub fn transfer_funds(&self,from: &Wallet, to: &Wallet, ledger: Ledger, funds: u64) -> Result<Ledger,LedgerError>{
        let fees = ledger.get_ledger_parameters().fees;
        let fee_value = Value(fees.constant + fees.coefficient*2);
        let funds_value = Value(funds);
        let input_value = (funds_value + fee_value).unwrap();
        let signed_tx = TransactionBuilder::new()
            .with_input(from.make_input_with_value(input_value))
            .with_output(to.make_output_with_value(funds_value))
            .authenticate()
            .with_witness(&self.block0_hash, &from.as_account_data())
            .seal();
        let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();
        let parameters = ledger.get_ledger_parameters();
        ledger.apply_transaction(&fragment_id, &signed_tx, &parameters).map(|(ledger,_)| ledger)
    }


    pub fn register(&self,funder: &Wallet, stake_pool: &StakePool, ledger: Ledger) -> Result<Ledger,LedgerError> {
        let cert = build_stake_pool_registration_cert(&stake_pool.info());
        self.apply_transaction_with_cert(&funder,cert,ledger)
    }

    pub fn delegates(&self,from: &Wallet, stake_pool: &StakePool, ledger: Ledger) -> Result<Ledger,LedgerError>{
        let cert = build_stake_delegation_cert(&stake_pool.info(),&from.as_account_data());
        self.apply_transaction_with_cert(&from,cert,ledger)
    }

    pub fn owner_delegates(&self,from: &Wallet, stake_pool: &StakePool, ledger: Ledger) -> Result<Ledger,LedgerError>{
        let cert = build_stake_owner_delegation_cert(&stake_pool.id());
        self.apply_transaction_with_cert(&from,cert,ledger)
    }

    pub fn retire(&self,owners: Vec<&Wallet>, stake_pool: &StakePool, ledger: Ledger) -> Result<Ledger,LedgerError>{
        let funder = owners.iter().next().unwrap();
        let owners_address_data: Vec<AddressData> = owners.iter().map(|x| x.as_account_data()).collect();
        let certificate = build_stake_pool_retirement_cert(&stake_pool.info(),0,&owners_address_data);
        let fees = ledger.get_ledger_parameters().fees;
        let fee_value = Value(fees.constant + fees.coefficient + fees.certificate);
        let fragment = TransactionCertBuilder::new()
            .with_input(funder.make_input_with_value(fee_value))
            .with_certificate(certificate)
            .authenticate()
            .with_witnesses(&self.block0_hash, &owners_address_data)
            .as_message();
        ledger.apply_fragment(&ledger.get_ledger_parameters(),&fragment,&Self::empty_context())
    }

    fn apply_transaction_with_cert(&self, funder: &Wallet, certificate: Certificate, ledger: Ledger) -> Result<Ledger,LedgerError>{
        let fees = ledger.get_ledger_parameters().fees;
        let fee_value = Value(fees.constant + fees.coefficient + fees.certificate);
        let fragment = TransactionCertBuilder::new()
            .with_input(funder.make_input_with_value(fee_value))
            .with_certificate(certificate)
            .authenticate()
            .with_witness(&self.block0_hash, &funder.as_account_data())
            .as_message();

        ledger.apply_fragment(&ledger.get_ledger_parameters(),&fragment,&Self::empty_context())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        testing::{
            ledger::ConfigBuilder,
            verifiers::LedgerStateVerifier
        },
        value::Value,
        fee::LinearFee,
    };

    #[test]
    pub fn build_scenario_example() {
        let (mut ledger,controller) = prepare_scenario()
            .with_config(
                ConfigBuilder::new()
                    .with_discrimination(Discrimination::Test)
                    .with_fee(LinearFee::new(1,1,1))
            )
            .with_initials(
                vec![
                    wallet("Alice").with(1_000).delegates_to("stake_pool"),
                    wallet("Bob").with(1_000),
                    wallet("Clarice").with(1_000).owns("stake_pool"),
                ]
            ).build().unwrap();
        
        let mut alice = controller.wallet("Alice").unwrap();
        let mut bob  = controller.wallet("Bob").unwrap();
        let mut clarice = controller.wallet("Clarice").unwrap();
        let stake_pool = controller.stake_pool("stake_pool").unwrap();

        
        ledger = controller.transfer_funds(&alice,&bob,ledger,100).unwrap();
        alice.confirm_transaction();
        ledger = controller.delegates(&bob,&stake_pool,ledger).unwrap();
        bob.confirm_transaction();
        ledger = controller.retire(vec![&clarice],&stake_pool,ledger).unwrap();
        clarice.confirm_transaction();
        
        // unassigned = clarice - fee (becaue thus clarise is an onwer of the stake she did not delegates any stakes)
        // dangling = bob and alice funds (minus fees for transactions and certs)
        // total pool = 0, because stake pool was retired

        LedgerStateVerifier::new(ledger).distribution()
            .unassigned_is(Value(997))
            .and()
            .dangling_is(Value(1994))
            .and()
            .pools_total_stake_is(Value::zero());
    }
}
