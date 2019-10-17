use crate::{
    testing::{
        data::{Wallet,StakePool},
        ledger::{create_initial_fake_ledger,ConfigBuilder},
        builders::{
            create_initial_transactions,
            create_initial_stake_pool_registrations,
            create_initial_stake_pool_delegation,
            StakePoolBuilder
        }
    },
    ledger::ledger::Ledger,
    fragment::{Fragment,config::ConfigParams},
};

use super::{
    template::{StakePoolTemplateBuilder,WalletTemplateBuilder,WalletTemplate,StakePoolTemplate},
    Controller
};

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub ScenarioBuilderError
        UndefinedConfig = " no config defined",
        UndefinedInitials =  "no initials defined",
        NoOwnersForStakePool{ alias: String} = "stake pool '{alias}' must have at least one owner",
        UndefinedValueForWallet { alias: String }= "with(...) method must be used for '{alias}' wallet in scenario builder. "
}


pub struct ScenarioBuilder {
    config: Option<ConfigParams>,
    initials: Option<Vec<WalletTemplateBuilder>>
}


pub fn prepare_scenario() -> ScenarioBuilder {
    ScenarioBuilder{
        config: None,
        initials: None
    }
}

impl ScenarioBuilder {

    pub fn with_config(&mut self, config: &mut ConfigBuilder) -> &mut Self {
        self.config = Some(config.build());
        self
    }

    pub fn with_initials(&mut self, initials: Vec<&mut WalletTemplateBuilder>) -> &mut Self {
        self.initials = Some(initials.iter().map(|x| (**x).clone()).collect());
        self
    }

    pub fn build(&self) -> Result<(Ledger,Controller),ScenarioBuilderError> {
       
        if self.initials.is_none() {
           return Err(ScenarioBuilderError::UndefinedInitials)
        }

        let initials: Result<Vec<WalletTemplate>,ScenarioBuilderError> = self.initials.clone().unwrap().iter().cloned().map(|x| x.build()).collect();
        let config = self.config.clone().ok_or(ScenarioBuilderError::UndefinedConfig)?;
        let initials: Vec<WalletTemplate> = initials?;
        let wallets: Vec<Wallet> = initials.iter().cloned().map(|x| self.build_wallet(x)).collect();
        let stake_pools_wallet_map = StakePoolTemplateBuilder::new(&initials);
        let stake_pool_templates: Vec<StakePoolTemplate> = stake_pools_wallet_map.build_stake_pool_templates(wallets.clone())?;

        let stake_pools = self.build_stake_pools(stake_pool_templates);

        let outputs = wallets.iter().cloned().map(|x| x.make_output()).collect();
        
        let mut messages = vec![create_initial_transactions(&outputs)];
        messages.extend(create_initial_stake_pool_registrations(&stake_pools));
        messages.extend(self.build_delegation_fragments(&initials,&stake_pools,&wallets));

        let (block0_hash,ledger) = create_initial_fake_ledger(&messages, config.clone()).unwrap();
        Ok((ledger,
           Controller {
            block0_hash: block0_hash,
            declared_wallets: wallets,
            declared_stake_pools: stake_pools
        }))
    }

    fn build_delegation_fragments(&self,initials: &Vec<WalletTemplate>, stake_pools: &Vec<StakePool>, wallets: &Vec<Wallet> ) -> Vec<Fragment> {
        initials.iter().cloned().filter(|x| x.delegates_stake_pool().is_some())
            .map(|wallet_template|
                {
                    let stake_pool_alias = wallet_template.delegates_stake_pool().unwrap();
                    let stake_pool = stake_pools.iter().find(|sp| sp.alias() == stake_pool_alias).unwrap();
                    let wallet_allias = wallet_template.alias();
                    let wallet = wallets.iter().find(|w| w.alias() == wallet_allias).unwrap();
                    create_initial_stake_pool_delegation(&stake_pool,&wallet)
                })
            .collect()
    }

    fn build_wallet(&self,template: WalletTemplate) -> Wallet {
        Wallet::new(&template.alias(),template.initial_value)
    }

    fn build_stake_pools(&self, stake_pool_templates: Vec<StakePoolTemplate>) -> Vec<StakePool> {
        stake_pool_templates.iter().cloned().map(|x| self.build_stake_pool(x)).collect()
    }

    fn build_stake_pool(&self, template: StakePoolTemplate) -> StakePool {
        StakePoolBuilder::new()
            .with_owners(template.owners())
            .with_alias(&template.alias())
            .build()
    }
}

pub fn wallet(alias: &str) -> WalletTemplateBuilder {
    WalletTemplateBuilder::new(alias)
}

