use crate::block::ConsensusVersion;
use crate::block::HeaderHash;
use crate::config::ConfigParam;
use crate::fragment::config::ConfigParams;
use crate::fragment::Fragment;
use crate::leadership::bft::LeaderId;
use crate::ledger::Error;
use crate::ledger::Ledger;
use crate::milli::Milli;
use crate::transaction::*;
use chain_addr::{Address, Discrimination};
use chain_crypto::*;
use std::vec::Vec;

use crate::testing::{
    tx_builder::TransactionBuilder,
    data::AddressDataValue,
};

pub struct ConfigBuilder {
    slot_duration: u8,
    slots_per_epoch: u32,
    active_slots_coeff: Milli,
    discrimination: Discrimination,
    leaders: Vec<LeaderId>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            slot_duration: 20,
            slots_per_epoch: 21600,
            active_slots_coeff: Milli::HALF,
            discrimination: Discrimination::Test,
            leaders: Vec::new(),
        }
    }

    pub fn with_discrimination(&mut self, discrimination: Discrimination) -> &mut Self {
        self.discrimination = discrimination;
        self
    }

    pub fn with_slot_duration(&mut self, slot_duration: u8) -> &mut Self {
        self.slot_duration = slot_duration;
        self
    }

    pub fn with_leaders(&mut self, leaders: &Vec<LeaderId>) -> &mut Self {
        self.leaders.extend(leaders.iter().cloned());
        self
    }

    pub fn with_slots_per_epoch(&mut self, slots_per_epoch: u32) -> &mut Self {
        self.slots_per_epoch = slots_per_epoch;
        self
    }

    pub fn with_active_slots_coeff(&mut self, active_slots_coeff: Milli) -> &mut Self {
        self.active_slots_coeff = active_slots_coeff;
        self
    }

    fn create_single_bft_leader() -> LeaderId {
        let leader_prv_key: SecretKey<Ed25519Extended> =
            SecretKey::generate(rand_os::OsRng::new().unwrap());
        let leader_pub_key = leader_prv_key.to_public();
        leader_pub_key.into()
    }

    pub fn build(&mut self) -> ConfigParams {
        let mut ie = ConfigParams::new();
        ie.push(ConfigParam::Discrimination(self.discrimination));
        ie.push(ConfigParam::ConsensusVersion(ConsensusVersion::Bft));

        // TODO remove rng: make this creation deterministic
        if self.leaders.is_empty() {
            self.leaders.push(Self::create_single_bft_leader());
        }
        for leader_id in self.leaders.iter().cloned() {
            ie.push(ConfigParam::AddBftLeader(leader_id));
        }

        ie.push(ConfigParam::Block0Date(crate::config::Block0Date(0)));
        ie.push(ConfigParam::SlotDuration(self.slot_duration));
        ie.push(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(
            self.active_slots_coeff,
        ));
        ie.push(ConfigParam::SlotsPerEpoch(self.slots_per_epoch));
        ie.push(ConfigParam::KESUpdateSpeed(3600 * 12));
        ie
    }
}

// create an initial fake ledger with the non-optional parameter setup
pub fn create_initial_fake_ledger(
    initial_msgs: &[Fragment],
    config_params: ConfigParams,
) -> Result<(HeaderHash, Ledger), Error> {
    let block0_hash = HeaderHash::hash_bytes(&[1, 2, 3]);

    let mut messages = Vec::new();
    messages.push(Fragment::Initial(config_params));
    messages.extend_from_slice(initial_msgs);

    let ledger_init_result = Ledger::new(block0_hash, &messages);
    match ledger_init_result {
        Ok(ledger) => Ok((block0_hash, ledger)),
        Err(error) => Err(error),
    }
}

pub fn create_initial_transaction(output: Output<Address>) -> Fragment {
    let mut builder = TransactionBuilder::new();
    let authenticator = builder.with_output(output).authenticate();
    authenticator.as_message()
}

pub fn create_initial_transactions(outputs: &Vec<Output<Address>>) -> Fragment {
    let mut builder = TransactionBuilder::new();
    let authenticator = builder.with_outputs(outputs.to_vec()).authenticate();
    authenticator.as_message()
}

pub fn create_fake_ledger_with_faucet(faucets: &[AddressDataValue], config_params: ConfigParams) -> Result<(HeaderHash, Ledger), Error> {
    let outputs: Vec<Output<Address>> = faucets.iter().map(|x| x.make_output()).collect();
    let message = create_initial_transactions(&outputs);
    create_initial_fake_ledger(&[message], config_params)
}
