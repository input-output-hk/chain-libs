//! This module holds all the necessary methods to be able to serialize the ledger.
//! There are 2 methods for each type that needs to be serialize `pack_*` and `unpack_*`.
//!
//! The pack methods takes a mutable `chain-core::packer::Codec<std::io::Write>` reference
//! and a reference to the type and writes the selected serialize format to the writer, it returns
//! an `std::io::Error` wrapped in a Result if something goes wrong:
//!
//! ```ignore
//! fn pack_<W: std::io::Write>(
//!     type: &T,
//!     codec: &mut Codec<W>,
//! ) -> Result<(), std::io::Error> { ... }
//! ```
//!
//! The unpack method takes a mutable chain-core::packer::Codec<std::io::BufRead> reference
//! and returns an instance of a type wrapped in a `Result`
//!
//! ```ignore
//! fn unpack_<R: std::io::BufRead>(
//!     codec: &mut Codec<R>,
//! ) -> Result<T, std::io::Error> { ... }
//! ```
//!
//!
//! For serializing the Ledger the approach is simple:
//! * Iterate the Ledger
//! * Pack each entry
//! * Flag the end of packing
//!
//! For deserializing:
//! * Load all serialized `Entry` into a `Vec`
//! * Use the `from_iter` ledger method to load it from the newly created vector.
//!
//! Notice that the `ledger::iter::Entry` type holds references to the data types but when loading
//! them from the serialized object we need to hold them. That is why we use the `EntryOwned` type
//! instead for deserializing. This data is then cloned as necessary into the final deserialized ledger.

use super::pots;
use super::{Entry, EntryOwned};
use crate::accounting::account::{
    AccountState, DelegationRatio, DelegationType, LastRewards, SpendingCounter,
    SpendingCounterIncreasing,
};
use crate::certificate::{
    PoolId, PoolRegistration, Proposal, Proposals, UpdateProposal, UpdateProposalId, UpdateVoterId,
    VoteAction, VotePlan,
};
use crate::config::ConfigParam;
use crate::date::BlockDate;
use crate::fragment::FragmentId;
use crate::header::{ChainLength, HeaderId};
use crate::key::serialize_public_key;
use crate::ledger::{Globals, Ledger, LedgerStaticParameters};
use crate::legacy;
use crate::multisig::{DeclElement, Declaration};
use crate::stake::{PoolLastRewards, PoolState};
use crate::transaction::Output;
use crate::update::UpdateProposalState;
use crate::value::Value;
use crate::vote;
use crate::{config, key, multisig, utxo};
use chain_addr::{Address, Discrimination};
use chain_core::{
    mempack::{ReadBuf, ReadError},
    property::{Deserialize, Serialize, WriteError},
};
use chain_crypto::digest::{DigestAlg, DigestOf};
use chain_ser::packer::Codec;
use chain_time::era::{pack_time_era, unpack_time_era};
use imhamt::Hamt;
use std::convert::TryFrom;
use std::io::Write;
use std::sync::Arc;

#[cfg(test)]
use crate::{
    chaintypes::ConsensusVersion,
    fee::{LinearFee, PerCertificateFee, PerVoteCertificateFee},
    key::BftLeaderId,
};

fn pack_pool_id<W: std::io::Write>(
    pool_id: &PoolId,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_digestof(pool_id, codec)
}

fn unpack_pool_id(buf: &mut ReadBuf) -> Result<PoolId, ReadError> {
    unpack_digestof(buf)
}

fn pack_discrimination<W: std::io::Write>(
    discrimination: Discrimination,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    match discrimination {
        Discrimination::Production => {
            codec.put_u8(0)?;
        }
        Discrimination::Test => {
            codec.put_u8(1)?;
        }
    };
    Ok(())
}

fn unpack_discrimination(buf: &mut ReadBuf) -> Result<Discrimination, ReadError> {
    match buf.get_u8()? {
        0 => Ok(Discrimination::Production),
        1 => Ok(Discrimination::Test),
        code => Err(ReadError::InvalidData(format!(
            "Not recognize code {}",
            code
        ))),
    }
}

fn pack_digestof<H: DigestAlg, T, W: std::io::Write>(
    digestof: &DigestOf<H, T>,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    let inner_data = digestof.as_ref();
    codec.put_u64(inner_data.len() as u64)?;
    codec.put_bytes(inner_data)?;
    Ok(())
}

fn unpack_digestof<H: DigestAlg, T>(buf: &mut ReadBuf) -> Result<DigestOf<H, T>, ReadError> {
    let size = buf.get_u64()?;
    let bytes = buf.get_slice(size as usize)?;
    DigestOf::try_from(bytes).map_err(|e| ReadError::InvalidData(e.to_string()))
}

fn pack_account_identifier<W: std::io::Write>(
    identifier: &crate::account::Identifier,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    serialize_public_key(identifier.as_ref(), codec)
}

fn unpack_account_identifier(buf: &mut ReadBuf) -> Result<crate::account::Identifier, ReadError> {
    crate::account::Identifier::deserialize(buf)
}

fn pack_spending_strategy<W: std::io::Write>(
    spending_strategy: &SpendingCounterIncreasing,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    let counters = spending_strategy.get_valid_counters();
    for counter in counters {
        codec.put_u32(counter.into())?;
    }
    Ok(())
}

fn unpack_spending_strategy(buf: &mut ReadBuf) -> Result<SpendingCounterIncreasing, ReadError> {
    let mut counters = Vec::new();
    for _ in 0..SpendingCounterIncreasing::LANES {
        let counter = SpendingCounter(buf.get_u32()?);
        counters.push(counter);
    }
    let got_length = counters.len();
    SpendingCounterIncreasing::new_from_counters(counters).ok_or_else(|| {
        ReadError::InvalidData(format!(
            "wrong numbers of lanes, expecting {} but got {}",
            SpendingCounterIncreasing::LANES,
            got_length,
        ))
    })
}

fn pack_account_state<W: std::io::Write>(
    account_state: &AccountState<()>,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_spending_strategy(&account_state.spending, codec)?;
    pack_delegation_type(&account_state.delegation, codec)?;
    codec.put_u64(account_state.value.0)?;
    pack_last_rewards(&account_state.last_rewards, codec)?;
    Ok(())
}

fn unpack_account_state(buf: &mut ReadBuf) -> Result<AccountState<()>, ReadError> {
    let spending = unpack_spending_strategy(buf)?;
    let delegation = unpack_delegation_type(buf)?;
    let value = buf.get_u64()?;
    let last_rewards = unpack_last_rewards(buf)?;
    Ok(AccountState {
        spending,
        delegation,
        value: Value(value),
        last_rewards,
        extra: (),
    })
}

fn pack_delegation_ratio<W: std::io::Write>(
    delegation_ratio: &DelegationRatio,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u8(delegation_ratio.parts)?;
    // len of items in pools, for later use by the deserialize method
    codec.put_u64(delegation_ratio.pools.len() as u64)?;
    for (pool_id, u) in delegation_ratio.pools.iter() {
        codec.put_u8(*u)?;
        pack_pool_id(pool_id, codec)?;
    }
    Ok(())
}

fn unpack_delegation_ratio(buf: &mut ReadBuf) -> Result<DelegationRatio, ReadError> {
    let parts = buf.get_u8()?;
    let pool_size = buf.get_u64()?;
    let mut pools: Vec<(PoolId, u8)> = Vec::with_capacity(pool_size as usize);
    for _ in 0..pool_size {
        let u = buf.get_u8()?;
        pools.push((unpack_pool_id(buf)?, u));
    }

    DelegationRatio::new(parts, pools).ok_or_else(|| {
        ReadError::InvalidData("Error building DelegationRatio from serialized data".to_string())
    })
}

fn pack_delegation_type<W: std::io::Write>(
    delegation_type: &DelegationType,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    match delegation_type {
        DelegationType::NonDelegated => {
            codec.put_u8(0)?;
        }
        DelegationType::Full(pool_id) => {
            codec.put_u8(1)?;
            pack_pool_id(pool_id, codec)?;
        }
        DelegationType::Ratio(delegation_ratio) => {
            codec.put_u8(2)?;
            pack_delegation_ratio(delegation_ratio, codec)?;
        }
    }
    Ok(())
}

fn unpack_delegation_type(buf: &mut ReadBuf) -> Result<DelegationType, ReadError> {
    match buf.get_u8()? {
        0 => Ok(DelegationType::NonDelegated),
        1 => Ok(DelegationType::Full(unpack_pool_id(buf)?)),
        2 => Ok(DelegationType::Ratio(unpack_delegation_ratio(buf)?)),
        code => Err(ReadError::InvalidData(format!(
            "Invalid DelegationType type code {}",
            code
        ))),
    }
}

fn pack_last_rewards<W: std::io::Write>(
    last_rewards: &LastRewards,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u32(last_rewards.epoch)?;
    codec.put_u64(last_rewards.reward.0)?;
    Ok(())
}

fn unpack_last_rewards(buf: &mut ReadBuf) -> Result<LastRewards, ReadError> {
    Ok(LastRewards {
        epoch: buf.get_u32()?,
        reward: Value(buf.get_u64()?),
    })
}

#[cfg(test)]
fn pack_consensus_version<W: std::io::Write>(
    consensus_version: ConsensusVersion,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    match consensus_version {
        ConsensusVersion::Bft => {
            codec.put_u8(1)?;
        }
        ConsensusVersion::GenesisPraos => {
            codec.put_u8(2)?;
        }
    }
    Ok(())
}

#[cfg(test)]
fn unpack_consensus_version(buf: &mut ReadBuf) -> Result<ConsensusVersion, ReadError> {
    match buf.get_u8()? {
        1 => Ok(ConsensusVersion::Bft),
        2 => Ok(ConsensusVersion::GenesisPraos),
        code => Err(ReadError::InvalidData(format!(
            "Unrecognized code {} for ConsensusVersion",
            code
        ))),
    }
}

fn pack_pool_registration<W: std::io::Write>(
    pool_registration: &PoolRegistration,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    let byte_array = pool_registration.serialize();
    let bytes = byte_array.as_slice();
    let size = bytes.len() as u64;
    // TODO: do not store extra bytes
    codec.put_u64(size)?;
    codec.put_bytes(bytes)?;
    Ok(())
}

fn unpack_pool_registration(buf: &mut ReadBuf) -> Result<PoolRegistration, ReadError> {
    // TODO: do not store extra bytes
    buf.get_u64()?;
    PoolRegistration::deserialize(buf)
}

fn pack_config_param<W: Write>(
    config_param: &ConfigParam,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    config_param.serialize(codec)
}

fn unpack_config_param(buf: &mut ReadBuf) -> Result<ConfigParam, ReadError> {
    ConfigParam::deserialize(buf)
}

fn pack_block_date<W: std::io::Write>(
    block_date: BlockDate,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u32(block_date.epoch)?;
    codec.put_u32(block_date.slot_id)?;
    Ok(())
}

fn unpack_block_date(buf: &mut ReadBuf) -> Result<BlockDate, ReadError> {
    let epoch = buf.get_u32()?;
    let slot_id = buf.get_u32()?;
    Ok(BlockDate { epoch, slot_id })
}

#[cfg(test)]
fn pack_linear_fee<W: std::io::Write>(
    linear_fee: &LinearFee,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u64(linear_fee.constant)?;
    codec.put_u64(linear_fee.coefficient)?;
    codec.put_u64(linear_fee.certificate)?;
    pack_per_certificate_fee(&linear_fee.per_certificate_fees, codec)?;
    pack_per_vote_certificate_fee(&linear_fee.per_vote_certificate_fees, codec)?;
    Ok(())
}

#[cfg(test)]
fn unpack_linear_fee(buf: &mut ReadBuf) -> Result<LinearFee, ReadError> {
    let constant = buf.get_u64()?;
    let coefficient = buf.get_u64()?;
    let certificate = buf.get_u64()?;
    let per_certificate_fees = unpack_per_certificate_fee(buf)?;
    let per_vote_certificate_fees = unpack_per_vote_certificate_fee(buf)?;
    Ok(LinearFee {
        constant,
        coefficient,
        certificate,
        per_certificate_fees,
        per_vote_certificate_fees,
    })
}

#[cfg(test)]
fn pack_per_certificate_fee<W: std::io::Write>(
    per_certificate_fee: &PerCertificateFee,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u64(
        per_certificate_fee
            .certificate_pool_registration
            .map(|v| v.get())
            .unwrap_or(0),
    )?;
    codec.put_u64(
        per_certificate_fee
            .certificate_stake_delegation
            .map(|v| v.get())
            .unwrap_or(0),
    )?;
    codec.put_u64(
        per_certificate_fee
            .certificate_owner_stake_delegation
            .map(|v| v.get())
            .unwrap_or(0),
    )?;
    Ok(())
}

#[cfg(test)]
fn pack_per_vote_certificate_fee<W: std::io::Write>(
    per_vote_certificate_fee: &PerVoteCertificateFee,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u64(
        per_vote_certificate_fee
            .certificate_vote_plan
            .map(|v| v.get())
            .unwrap_or(0),
    )?;
    codec.put_u64(
        per_vote_certificate_fee
            .certificate_vote_cast
            .map(|v| v.get())
            .unwrap_or(0),
    )?;
    Ok(())
}

#[cfg(test)]
fn unpack_per_certificate_fee(buf: &mut ReadBuf) -> Result<PerCertificateFee, ReadError> {
    let certificate_pool_registration = std::num::NonZeroU64::new(buf.get_u64()?);
    let certificate_stake_delegation = std::num::NonZeroU64::new(buf.get_u64()?);
    let certificate_owner_stake_delegation = std::num::NonZeroU64::new(buf.get_u64()?);

    Ok(PerCertificateFee {
        certificate_pool_registration,
        certificate_stake_delegation,
        certificate_owner_stake_delegation,
    })
}

#[cfg(test)]
fn unpack_per_vote_certificate_fee(buf: &mut ReadBuf) -> Result<PerVoteCertificateFee, ReadError> {
    let certificate_vote_plan = std::num::NonZeroU64::new(buf.get_u64()?);
    let certificate_vote_cast = std::num::NonZeroU64::new(buf.get_u64()?);

    Ok(PerVoteCertificateFee {
        certificate_vote_plan,
        certificate_vote_cast,
    })
}

#[cfg(test)]
fn pack_leader_id<W: std::io::Write>(
    leader_id: &BftLeaderId,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    serialize_public_key(&leader_id.0, codec)
}

#[cfg(test)]
fn unpack_leader_id(buf: &mut ReadBuf) -> Result<BftLeaderId, ReadError> {
    BftLeaderId::deserialize(buf)
}

fn pack_header_id<W: std::io::Write>(
    header_id: &HeaderId,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    header_id.serialize(codec)
}

fn unpack_header_id(buf: &mut ReadBuf) -> Result<HeaderId, ReadError> {
    HeaderId::deserialize(buf)
}

fn pack_ledger_static_parameters<W: std::io::Write>(
    ledger_static_parameters: &LedgerStaticParameters,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_header_id(&ledger_static_parameters.block0_initial_hash, codec)?;
    codec.put_u64(ledger_static_parameters.block0_start_time.0)?;
    pack_discrimination(ledger_static_parameters.discrimination, codec)?;
    codec.put_u32(ledger_static_parameters.kes_update_speed)?;
    Ok(())
}

fn unpack_ledger_static_parameters(buf: &mut ReadBuf) -> Result<LedgerStaticParameters, ReadError> {
    let block0_initial_hash = unpack_header_id(buf)?;
    let block0_start_time = config::Block0Date(buf.get_u64()?);
    let discrimination = unpack_discrimination(buf)?;
    let kes_update_speed = buf.get_u32()?;
    Ok(LedgerStaticParameters {
        block0_initial_hash,
        block0_start_time,
        discrimination,
        kes_update_speed,
    })
}

fn pack_globals<W: std::io::Write>(
    globals: &Globals,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_block_date(globals.date, codec)?;
    codec.put_u32(globals.chain_length.0)?;
    pack_ledger_static_parameters(&globals.static_params, codec)?;
    pack_time_era(&globals.era, codec)?;
    Ok(())
}

fn unpack_globals(buf: &mut ReadBuf) -> Result<Globals, ReadError> {
    let date = unpack_block_date(buf)?;
    let chain_length = ChainLength(buf.get_u32()?);
    let static_params = unpack_ledger_static_parameters(buf)?;
    let era = unpack_time_era(buf)?;
    Ok(Globals {
        date,
        chain_length,
        static_params,
        era,
    })
}

fn pack_pot_entry<W: std::io::Write>(
    entry: &pots::Entry,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    match entry {
        pots::Entry::Fees(value) => {
            codec.put_u8(0)?;
            codec.put_u64(value.0)?;
        }
        pots::Entry::Treasury(value) => {
            codec.put_u8(1)?;
            codec.put_u64(value.0)?;
        }
        pots::Entry::Rewards(value) => {
            codec.put_u8(2)?;
            codec.put_u64(value.0)?;
        }
    }
    Ok(())
}

fn unpack_pot_entry(buf: &mut ReadBuf) -> Result<pots::Entry, ReadError> {
    match buf.get_u8()? {
        0 => Ok(pots::Entry::Fees(Value(buf.get_u64()?))),
        1 => Ok(pots::Entry::Treasury(Value(buf.get_u64()?))),
        2 => Ok(pots::Entry::Rewards(Value(buf.get_u64()?))),
        code => Err(ReadError::InvalidData(format!(
            "Invalid Entry type code {}",
            code
        ))),
    }
}

fn pack_multisig_identifier<W: std::io::Write>(
    identifier: &multisig::Identifier,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    identifier.0.serialize(codec)
}

fn unpack_multisig_identifier(buf: &mut ReadBuf) -> Result<multisig::Identifier, ReadError> {
    Ok(multisig::Identifier(key::Hash::deserialize(buf)?))
}

fn pack_declaration<W: std::io::Write>(
    declaration: &Declaration,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u8(declaration.threshold)?;
    codec.put_u64(declaration.owners.len() as u64)?;
    for owner in &declaration.owners {
        pack_decl_element(owner, codec)?;
    }
    Ok(())
}

fn unpack_declaration(buf: &mut ReadBuf) -> Result<Declaration, ReadError> {
    let threshold = buf.get_u8()?;
    let size = buf.get_u64()?;
    let mut owners: Vec<DeclElement> = Vec::with_capacity(size as usize);
    for _ in 0..size {
        let decl_element = unpack_decl_element(buf)?;
        owners.push(decl_element);
    }
    Ok(Declaration { threshold, owners })
}

fn pack_decl_element<W: std::io::Write>(
    decl_element: &DeclElement,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    match &decl_element {
        DeclElement::Sub(declaration) => {
            codec.put_u8(0)?;
            pack_declaration(declaration, codec)?;
        }
        DeclElement::Owner(hash) => {
            codec.put_u8(1)?;
            hash.serialize(codec)?;
        }
    }
    Ok(())
}

fn unpack_decl_element(buf: &mut ReadBuf) -> Result<DeclElement, ReadError> {
    match buf.get_u8()? {
        0 => Ok(DeclElement::Sub(unpack_declaration(buf)?)),
        1 => Ok(DeclElement::Owner(key::Hash::deserialize(buf)?)),
        code => Err(ReadError::InvalidData(format!(
            "Invalid DeclElement type code {}",
            code
        ))),
    }
}

fn pack_pool_last_rewards<W: std::io::Write>(
    pool_last_rewards: &PoolLastRewards,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u32(pool_last_rewards.epoch)?;
    codec.put_u64(pool_last_rewards.value_taxed.0)?;
    codec.put_u64(pool_last_rewards.value_for_stakers.0)?;
    Ok(())
}

fn unpack_pool_last_rewards(buf: &mut ReadBuf) -> Result<PoolLastRewards, ReadError> {
    let epoch = buf.get_u32()?;
    let value_taxed = Value(buf.get_u64()?);
    let value_for_stakers = Value(buf.get_u64()?);

    Ok(PoolLastRewards {
        epoch,
        value_taxed,
        value_for_stakers,
    })
}

fn pack_pool_state<W: std::io::Write>(
    pool_state: &PoolState,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_pool_last_rewards(&pool_state.last_rewards, codec)?;
    pack_pool_registration(&pool_state.registration, codec)?;
    Ok(())
}

fn unpack_pool_state(buf: &mut ReadBuf) -> Result<PoolState, ReadError> {
    let last_rewards = unpack_pool_last_rewards(buf)?;
    let registration = Arc::new(unpack_pool_registration(buf)?);

    Ok(PoolState {
        last_rewards,
        registration,
    })
}

fn pack_update_proposal_state<W: std::io::Write>(
    update_proposal_state: &UpdateProposalState,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_update_proposal(&update_proposal_state.proposal, codec)?;
    pack_block_date(update_proposal_state.proposal_date, codec)?;
    codec.put_u64(update_proposal_state.votes.size() as u64)?;
    {
        let mut codec = Codec::new(codec);
        for (voter, _) in &update_proposal_state.votes {
            voter.serialize(&mut codec)?;
        }
    }
    Ok(())
}

fn unpack_update_proposal_state(buf: &mut ReadBuf) -> Result<UpdateProposalState, ReadError> {
    let proposal = unpack_update_proposal(buf)?;
    let proposal_date = unpack_block_date(buf)?;
    let total_votes = buf.get_u64()?;
    let mut votes = Hamt::new();

    for _ in 0..total_votes {
        let id = UpdateVoterId::deserialize(buf)?;
        votes = votes
            .insert(id, ())
            .map_err(|e| ReadError::InvalidData(e.to_string()))?;
    }

    Ok(UpdateProposalState {
        proposal,
        proposal_date,
        votes,
    })
}

fn pack_update_proposal<W: std::io::Write>(
    update_proposal: &UpdateProposal,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    Serialize::serialize(update_proposal, codec)
}

fn unpack_update_proposal(buf: &mut ReadBuf) -> Result<UpdateProposal, ReadError> {
    UpdateProposal::deserialize(buf)
}

fn pack_update_proposal_id<W: std::io::Write>(
    update_proposal_id: &UpdateProposalId,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    update_proposal_id.serialize(codec)
}

fn unpack_update_proposal_id(buf: &mut ReadBuf) -> Result<UpdateProposalId, ReadError> {
    UpdateProposalId::deserialize(buf)
}

fn pack_utxo_entry<OutputAddress, F, W: std::io::Write>(
    entry: &utxo::Entry<'_, OutputAddress>,
    output_address_packer: &mut F,
    codec: &mut Codec<W>,
) -> Result<(), WriteError>
where
    F: FnMut(&OutputAddress, &mut Codec<W>) -> Result<(), WriteError>,
{
    let fragment_id_bytes = entry.fragment_id.as_ref();
    codec.put_bytes(fragment_id_bytes)?;
    codec.put_u8(entry.output_index)?;
    pack_output(entry.output, output_address_packer, codec)?;
    Ok(())
}

fn unpack_utxo_entry_owned<OutputAddress, F>(
    output_address_unpacker: &mut F,
    buf: &mut ReadBuf,
) -> Result<utxo::EntryOwned<OutputAddress>, ReadError>
where
    F: FnMut(&mut ReadBuf) -> Result<OutputAddress, ReadError>,
{
    let mut fragment_id_bytes: [u8; 32] = [0; 32];
    buf.copy_to_slice_mut(&mut fragment_id_bytes)?;
    let fragment_id = FragmentId::from_bytes(fragment_id_bytes);
    let output_index = buf.get_u8()?;
    let output: Output<OutputAddress> = unpack_output(output_address_unpacker, buf)?;
    Ok(utxo::EntryOwned {
        fragment_id,
        output_index,
        output,
    })
}

fn pack_output<OutputAddress, F, W: std::io::Write>(
    output: &Output<OutputAddress>,
    address_packer: &mut F,
    codec: &mut Codec<W>,
) -> Result<(), WriteError>
where
    F: FnMut(&OutputAddress, &mut Codec<W>) -> Result<(), WriteError>,
{
    address_packer(&output.address, codec)?;
    codec.put_u64(output.value.0)?;
    Ok(())
}

fn unpack_output<OutputAddress, F>(
    address_unpacker: &mut F,
    buf: &mut ReadBuf,
) -> Result<Output<OutputAddress>, ReadError>
where
    F: FnMut(&mut ReadBuf) -> Result<OutputAddress, ReadError>,
{
    let address = address_unpacker(buf)?;
    let value = Value(buf.get_u64()?);
    Ok(Output { address, value })
}

fn pack_old_addr<W: std::io::Write>(
    addr: &legacy::OldAddress,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    let bytes = addr.as_ref();
    codec.put_u64(bytes.len() as u64)?;
    codec.put_bytes(bytes)?;
    Ok(())
}

fn unpack_old_addr(buf: &mut ReadBuf) -> Result<legacy::OldAddress, ReadError> {
    let size = buf.get_u64()?;
    let v = buf.get_slice(size as usize)?;
    Ok(legacy::OldAddress::new(v.into()))
}

fn pack_address<W: std::io::Write>(
    address: &Address,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    let bytes = address.to_bytes();
    codec.put_u64(bytes.len() as u64)?;
    codec.put_bytes(&bytes)?;
    Ok(())
}

fn unpack_address(buf: &mut ReadBuf) -> Result<Address, ReadError> {
    let size = buf.get_u64()?;
    let v = buf.get_slice(size as usize)?;
    Address::from_bytes(v).map_err(|e| {
        ReadError::InvalidData(format!("Error reading address from packed bytes: {}", e))
    })
}

fn pack_vote_proposal<W: std::io::Write>(
    proposal: &Proposal,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_digestof(proposal.external_id(), codec)?;
    codec.put_u8(proposal.options().as_byte())?;
    Ok(())
}

fn unpack_proposal(buf: &mut ReadBuf) -> Result<Proposal, ReadError> {
    let external_id = unpack_digestof(buf)?;
    let options = vote::Options::new_length(buf.get_u8()?)
        .map_err(|e| ReadError::InvalidData(e.to_string()))?;
    let action = unpack_vote_action(buf)?;
    Ok(Proposal::new(external_id, options, action))
}

fn unpack_vote_action(_codec: &mut ReadBuf) -> Result<VoteAction, ReadError> {
    todo!()
}

fn pack_vote_proposals<W: std::io::Write>(
    proposals: &Proposals,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u64(proposals.len() as u64)?;
    for proposal in proposals.iter() {
        pack_vote_proposal(proposal, codec)?;
    }
    Ok(())
}

fn unpack_proposals(buf: &mut ReadBuf) -> Result<Proposals, ReadError> {
    let mut proposals = Proposals::new();
    let size = buf.get_u64()?;
    for _ in 0..size {
        let _ = proposals.push(unpack_proposal(buf)?);
    }
    Ok(proposals)
}

fn pack_payload_type<W: std::io::Write>(
    t: vote::PayloadType,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    codec.put_u8(t as u8)
}

fn unpack_payload_type(buf: &mut ReadBuf) -> Result<vote::PayloadType, ReadError> {
    use std::convert::TryFrom as _;

    let byte = buf.get_u8()?;
    vote::PayloadType::try_from(byte).map_err(|e| ReadError::InvalidData(e.to_string()))
}

fn pack_committee_public_keys<W: std::io::Write>(
    keys: &[chain_vote::MemberPublicKey],
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    use std::convert::TryInto;
    codec.put_u8(keys.len().try_into().unwrap())?;
    for k in keys {
        codec.write_all(&k.to_bytes())?;
    }
    Ok(())
}

fn unpack_committee_public_keys(
    buf: &mut ReadBuf,
) -> Result<Vec<chain_vote::MemberPublicKey>, ReadError> {
    let size = buf.get_u8()?;
    let mut result = Vec::new();
    for _ in 0..size {
        let bytes = buf.get_slice(chain_vote::MemberPublicKey::BYTES_LEN)?;
        let key = chain_vote::MemberPublicKey::from_bytes(bytes).ok_or_else(|| {
            ReadError::InvalidData("invalid committee member public key in a vote plan".to_string())
        })?;
        result.push(key);
    }
    Ok(result)
}

fn pack_vote_plan<W: std::io::Write>(
    vote_plan: &VotePlan,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    pack_block_date(vote_plan.vote_start(), codec)?;
    pack_block_date(vote_plan.vote_end(), codec)?;
    pack_block_date(vote_plan.committee_end(), codec)?;
    pack_payload_type(vote_plan.payload_type(), codec)?;
    pack_vote_proposals(vote_plan.proposals(), codec)?;
    pack_committee_public_keys(vote_plan.committee_public_keys(), codec)?;
    Ok(())
}

fn unpack_vote_plan(buf: &mut ReadBuf) -> Result<VotePlan, ReadError> {
    let vote_start = unpack_block_date(buf)?;
    let vote_end = unpack_block_date(buf)?;
    let committee_end = unpack_block_date(buf)?;
    let payload_type = unpack_payload_type(buf)?;
    let proposals = unpack_proposals(buf)?;
    let keys = unpack_committee_public_keys(buf)?;
    Ok(VotePlan::new(
        vote_start,
        vote_end,
        committee_end,
        proposals,
        payload_type,
        keys,
    ))
}

#[derive(Debug, Eq, PartialEq)]
enum EntrySerializeCode {
    Globals = 0,
    Pot = 1,
    Utxo = 2,
    OldUtxo = 3,
    Account = 4,
    ConfigParam = 5,
    UpdateProposal = 6,
    MultisigAccount = 7,
    MultisigDeclaration = 8,
    StakePool = 9,
    LeaderParticipation = 10,
    VotePlan = 11,
    SerializationEnd = 99,
}

impl EntrySerializeCode {
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(EntrySerializeCode::Globals),
            1 => Some(EntrySerializeCode::Pot),
            2 => Some(EntrySerializeCode::Utxo),
            3 => Some(EntrySerializeCode::OldUtxo),
            4 => Some(EntrySerializeCode::Account),
            5 => Some(EntrySerializeCode::ConfigParam),
            6 => Some(EntrySerializeCode::UpdateProposal),
            7 => Some(EntrySerializeCode::MultisigAccount),
            8 => Some(EntrySerializeCode::MultisigDeclaration),
            9 => Some(EntrySerializeCode::StakePool),
            10 => Some(EntrySerializeCode::LeaderParticipation),
            11 => Some(EntrySerializeCode::VotePlan),
            99 => Some(EntrySerializeCode::SerializationEnd),
            _ => None,
        }
    }
}

fn pack_entry<W: std::io::Write>(
    entry: &Entry<'_>,
    codec: &mut Codec<W>,
) -> Result<(), WriteError> {
    match entry {
        Entry::Globals(entry) => {
            codec.put_u8(EntrySerializeCode::Globals as u8)?;
            pack_globals(entry, codec)?;
        }
        Entry::Pot(entry) => {
            codec.put_u8(EntrySerializeCode::Pot as u8)?;
            pack_pot_entry(entry, codec)?;
        }
        Entry::Utxo(entry) => {
            codec.put_u8(EntrySerializeCode::Utxo as u8)?;
            pack_utxo_entry(entry, &mut pack_address, codec)?;
        }
        Entry::OldUtxo(entry) => {
            codec.put_u8(EntrySerializeCode::OldUtxo as u8)?;
            pack_utxo_entry(entry, &mut pack_old_addr, codec)?;
        }
        Entry::Account((identifier, account_state)) => {
            codec.put_u8(EntrySerializeCode::Account as u8)?;
            pack_account_identifier(identifier, codec)?;
            pack_account_state(account_state, codec)?;
        }
        Entry::ConfigParam(config_param) => {
            codec.put_u8(EntrySerializeCode::ConfigParam as u8)?;
            pack_config_param(config_param, codec)?;
        }
        Entry::UpdateProposal((proposal_id, proposal_state)) => {
            codec.put_u8(EntrySerializeCode::UpdateProposal as u8)?;
            pack_update_proposal_id(proposal_id, codec)?;
            pack_update_proposal_state(proposal_state, codec)?;
        }
        Entry::MultisigAccount((identifier, account_state)) => {
            codec.put_u8(EntrySerializeCode::MultisigAccount as u8)?;
            pack_multisig_identifier(identifier, codec)?;
            pack_account_state(account_state, codec)?;
        }
        Entry::MultisigDeclaration((identifier, declaration)) => {
            codec.put_u8(EntrySerializeCode::MultisigDeclaration as u8)?;
            pack_multisig_identifier(identifier, codec)?;
            pack_declaration(declaration, codec)?;
        }
        Entry::StakePool((pool_id, pool_state)) => {
            codec.put_u8(EntrySerializeCode::StakePool as u8)?;
            pack_digestof(pool_id, codec)?;
            pack_pool_state(pool_state, codec)?;
        }
        Entry::LeaderParticipation((pool_id, participation)) => {
            codec.put_u8(EntrySerializeCode::LeaderParticipation as u8)?;
            pack_digestof(pool_id, codec)?;
            codec.put_u32(**participation)?;
        }
        Entry::VotePlan(vote_plan) => {
            codec.put_u8(EntrySerializeCode::VotePlan as u8)?;
            pack_vote_plan(vote_plan, codec)?;
        }
    }
    Ok(())
}

fn unpack_entry_owned(buf: &mut ReadBuf) -> Result<EntryOwned, ReadError> {
    let code_u8 = buf.get_u8()?;
    let code = EntrySerializeCode::from_u8(code_u8).ok_or_else(|| {
        ReadError::InvalidData(format!(
            "Error reading Entry, not recognized type code {}",
            code_u8
        ))
    })?;
    match code {
        EntrySerializeCode::Globals => Ok(EntryOwned::Globals(unpack_globals(buf)?)),
        EntrySerializeCode::Pot => Ok(EntryOwned::Pot(unpack_pot_entry(buf)?)),
        EntrySerializeCode::Utxo => Ok(EntryOwned::Utxo(unpack_utxo_entry_owned(
            &mut unpack_address,
            buf,
        )?)),
        EntrySerializeCode::OldUtxo => Ok(EntryOwned::OldUtxo(unpack_utxo_entry_owned(
            &mut unpack_old_addr,
            buf,
        )?)),
        EntrySerializeCode::Account => {
            let identifier = unpack_account_identifier(buf)?;
            let account = unpack_account_state(buf)?;
            Ok(EntryOwned::Account((identifier, account)))
        }
        EntrySerializeCode::ConfigParam => Ok(EntryOwned::ConfigParam(unpack_config_param(buf)?)),
        EntrySerializeCode::UpdateProposal => {
            let proposal_id = unpack_update_proposal_id(buf)?;
            let proposal_state = unpack_update_proposal_state(buf)?;
            Ok(EntryOwned::UpdateProposal((proposal_id, proposal_state)))
        }
        EntrySerializeCode::MultisigAccount => {
            let identifier = unpack_multisig_identifier(buf)?;
            let account_state = unpack_account_state(buf)?;
            Ok(EntryOwned::MultisigAccount((identifier, account_state)))
        }
        EntrySerializeCode::MultisigDeclaration => {
            let identifier = unpack_multisig_identifier(buf)?;
            let declaration = unpack_declaration(buf)?;
            Ok(EntryOwned::MultisigDeclaration((identifier, declaration)))
        }
        EntrySerializeCode::StakePool => {
            let pool_id = unpack_digestof(buf)?;
            let pool_state = unpack_pool_state(buf)?;
            Ok(EntryOwned::StakePool((pool_id, pool_state)))
        }
        EntrySerializeCode::LeaderParticipation => {
            let pool_id = unpack_digestof(buf)?;
            let v = buf.get_u32()?;
            Ok(EntryOwned::LeaderParticipation((pool_id, v)))
        }
        EntrySerializeCode::VotePlan => {
            let vote_plan = unpack_vote_plan(buf)?;
            Ok(EntryOwned::VotePlan(vote_plan))
        }
        EntrySerializeCode::SerializationEnd => Ok(EntryOwned::StopEntry),
    }
}

fn unpack_entries(buf: &mut ReadBuf) -> Result<Vec<EntryOwned>, ReadError> {
    let mut res = Vec::new();
    loop {
        match unpack_entry_owned(buf)? {
            EntryOwned::StopEntry => {
                break;
            }
            entry => {
                res.push(entry);
            }
        };
    }
    Ok(res)
}

impl Serialize for Ledger {
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), WriteError> {
        let mut codec = Codec::new(writer);
        for entry in self.iter() {
            pack_entry(&entry, &mut codec)?;
        }
        // Write finish flag
        codec.put_u8(EntrySerializeCode::SerializationEnd as u8)?;
        Ok(())
    }
}

impl Deserialize for Ledger {
    fn deserialize(buf: &mut ReadBuf) -> Result<Self, chain_core::mempack::ReadError> {
        let owned_entries = unpack_entries(buf)?;
        let entries = owned_entries
            .iter()
            .map(|entry_owned| entry_owned.to_entry().unwrap());
        let ledger: Result<Ledger, crate::ledger::Error> = entries.collect();
        ledger.map_err(|e| ReadError::InvalidData(e.to_string()))
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::testing::{ConfigBuilder, LedgerBuilder, StakePoolBuilder};
    use cardano_legacy_address::Addr;
    use chain_crypto::Blake2b256;
    use quickcheck::{quickcheck, TestResult};
    use typed_bytes::{ByteArray, ByteSlice};

    #[test]
    pub fn addr_pack_unpack_bijection() {
        // set fake buffer
        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        let address : Addr = "DdzFFzCqrhsqTG4t3uq5UBqFrxhxGVM6bvF4q1QcZXqUpizFddEEip7dx5rbife2s9o2fRU3hVKhRp4higog7As8z42s4AMw6Pcu8vL4".parse().unwrap();
        pack_old_addr(&address, &mut codec).unwrap();
        // reset fake buffer
        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let new_address = unpack_old_addr(&mut buf).unwrap();
        assert_eq!(address, new_address);
    }

    #[test]
    pub fn discrimination_pack_unpack_bijection() {
        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        pack_discrimination(Discrimination::Test, &mut codec).unwrap();
        pack_discrimination(Discrimination::Production, &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let test = unpack_discrimination(&mut buf).unwrap();
        let production = unpack_discrimination(&mut buf).unwrap();
        assert_eq!(Discrimination::Test, test);
        assert_eq!(Discrimination::Production, production);
    }

    #[test]
    pub fn digestof_pack_unpack_bijection() {
        let data: [u8; 32] = [0u8; 32];
        let slice = &data[..];
        let byte_array: ByteArray<u8> = ByteArray::from(Vec::from(slice));
        let byte_slice: ByteSlice<u8> = byte_array.as_byteslice();
        let digest: DigestOf<Blake2b256, u8> = DigestOf::digest_byteslice(&byte_slice);

        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        pack_digestof(&digest, &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let deserialize_digest: DigestOf<Blake2b256, u8> = unpack_digestof(&mut buf).unwrap();
        assert_eq!(digest, deserialize_digest);
    }

    #[test]
    pub fn delegation_ratio_pack_unpack_bijection() {
        let fake_pool_id = StakePoolBuilder::new().build().id();
        let parts = 8u8;
        let pools: Vec<(PoolId, u8)> = vec![
            (fake_pool_id.clone(), 2u8),
            (fake_pool_id.clone(), 3u8),
            (fake_pool_id, 3u8),
        ];

        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        let delegation_ratio = DelegationRatio::new(parts, pools).unwrap();
        pack_delegation_ratio(&delegation_ratio, &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let deserialized_delegation_ratio = unpack_delegation_ratio(&mut buf).unwrap();
        assert_eq!(delegation_ratio, deserialized_delegation_ratio);
    }

    #[test]
    pub fn delegation_type_pack_unpack_bijection() {
        let fake_pool_id = StakePoolBuilder::new().build().id();

        let non_delegated = DelegationType::NonDelegated;
        let full = DelegationType::Full(fake_pool_id.clone());

        let parts = 8u8;
        let pools: Vec<(PoolId, u8)> = vec![
            (fake_pool_id.clone(), 2u8),
            (fake_pool_id.clone(), 3u8),
            (fake_pool_id, 3u8),
        ];
        let ratio = DelegationType::Ratio(DelegationRatio::new(parts, pools).unwrap());

        for delegation_type in [non_delegated, full, ratio].iter() {
            let vec = Vec::new();
            let mut codec = Codec::new(vec);
            pack_delegation_type(delegation_type, &mut codec).unwrap();

            let vec = codec.into_inner();
            let mut buf = ReadBuf::from(vec.as_slice());
            let deserialized_delegation_type = unpack_delegation_type(&mut buf).unwrap();
            assert_eq!(delegation_type, &deserialized_delegation_type);
        }
    }

    #[test]
    pub fn account_state_pack_unpack_bijection() {
        let account_state = AccountState::new(Value(256), ());
        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        pack_account_state(&account_state, &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let deserialized_account_state = unpack_account_state(&mut buf).unwrap();
        assert_eq!(account_state, deserialized_account_state);
    }

    #[test]
    pub fn last_rewards_pack_unpack_bijection() {
        let last_rewards = LastRewards {
            epoch: 0,
            reward: Value(1),
        };

        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        pack_last_rewards(&last_rewards, &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let deserialize_last_rewards = unpack_last_rewards(&mut buf).unwrap();
        assert_eq!(last_rewards, deserialize_last_rewards);
    }

    #[test]
    pub fn pots_entry_pack_unpack_bijection() {
        for entry_value in [
            pots::Entry::Fees(Value(10)),
            pots::Entry::Rewards(Value(10)),
            pots::Entry::Treasury(Value(10)),
        ]
        .iter()
        {
            let vec = Vec::new();
            let mut codec = Codec::new(vec);
            pack_pot_entry(entry_value, &mut codec).unwrap();

            let vec = codec.into_inner();
            let mut buf = ReadBuf::from(vec.as_slice());
            let other_value = unpack_pot_entry(&mut buf).unwrap();
            assert_eq!(entry_value, &other_value);
        }
    }

    #[test]
    pub fn multisig_identifier_pack_unpack_bijection() {
        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        let id_bytes: [u8; 32] = [0x1; 32];
        let identifier = crate::multisig::Identifier::from(id_bytes);
        pack_multisig_identifier(&identifier, &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let other_identifier = unpack_multisig_identifier(&mut buf).unwrap();
        assert_eq!(identifier, other_identifier);
    }

    #[test]
    pub fn decl_element_pack_unpack_bijection() {
        let id_bytes: [u8; 32] = [0x1; 32];

        for decl_element in [
            DeclElement::Sub(Declaration {
                owners: Vec::new(),
                threshold: 10,
            }),
            DeclElement::Owner(key::Hash::from_bytes(id_bytes)),
        ]
        .iter()
        {
            let vec = Vec::new();
            let mut codec = Codec::new(vec);
            pack_decl_element(&decl_element, &mut codec).unwrap();

            let vec = codec.into_inner();
            let mut buf = ReadBuf::from(vec.as_slice());
            let other_value = unpack_decl_element(&mut buf).unwrap();
            assert_eq!(decl_element, &other_value);
        }
    }

    #[test]
    pub fn declaration_pack_unpack_bijection() {
        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        let declaration = Declaration {
            owners: Vec::new(),
            threshold: 0,
        };
        pack_declaration(&declaration, &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let other_value = unpack_declaration(&mut buf).unwrap();
        assert_eq!(declaration, other_value);
    }

    #[test]
    pub fn output_pack_unpack_bijection() {
        let output: Output<()> = Output {
            address: (),
            value: Value(1000),
        };

        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        pack_output(&output, &mut |_, _| Ok(()), &mut codec).unwrap();

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        let other_output = unpack_output(&mut |_| Ok(()), &mut buf).unwrap();
        assert_eq!(output, other_output);
    }

    #[test]
    pub fn ledger_serialize_deserialize_bijection() {
        let test_ledger = LedgerBuilder::from_config(ConfigBuilder::new())
            .faucet_value(Value(42000))
            .build()
            .expect("cannot build test ledger");

        let ledger: Ledger = test_ledger.into();
        let mut vec = Vec::new();
        ledger.serialize(&mut vec).unwrap();

        let mut buf = ReadBuf::from(vec.as_slice());
        let other_ledger = Ledger::deserialize(&mut buf).unwrap();
        assert_eq!(ledger, other_ledger);
    }

    #[cfg(test)]
    fn pack_unpack_bijection<T, Pack, Unpack>(
        pack_method: &Pack,
        unpack_method: &Unpack,
        value: T,
    ) -> TestResult
    where
        Pack: Fn(&T, &mut Codec<Vec<u8>>) -> Result<(), WriteError>,
        Unpack: Fn(&mut ReadBuf) -> Result<T, ReadError>,
        T: Eq,
    {
        let vec = Vec::new();
        let mut codec = Codec::new(vec);
        match pack_method(&value, &mut codec) {
            Ok(_) => (),
            Err(e) => return TestResult::error(format!("{}", e)),
        };

        let vec = codec.into_inner();
        let mut buf = ReadBuf::from(vec.as_slice());
        match unpack_method(&mut buf) {
            Ok(other_value) => TestResult::from_bool(value == other_value),
            Err(e) => TestResult::error(format!("{}", e)),
        }
    }

    quickcheck! {
        fn account_identifier_pack_unpack_bijection(id: crate::account::Identifier) -> TestResult {
            pack_unpack_bijection(
                &pack_account_identifier,
                &unpack_account_identifier,
                id
            )
        }

        fn consensus_version_serialization_bijection(consensus_version: ConsensusVersion) -> TestResult {
            pack_unpack_bijection(
                 &|v, p| pack_consensus_version(*v, p),
                 &unpack_consensus_version,
                 consensus_version
             )
         }

        fn pool_registration_serialize_deserialize_biyection(pool_registration: PoolRegistration) -> TestResult {
            pack_unpack_bijection(
                &pack_pool_registration,
                &unpack_pool_registration,
                pool_registration
            )
        }

        fn config_param_pack_unpack_bijection(config_param: ConfigParam) -> TestResult {
            pack_unpack_bijection(
                &pack_config_param,
                &unpack_config_param,
                config_param
            )
        }

        fn blockdate_pack_unpack_bijection(block_date: BlockDate) -> TestResult {
            pack_unpack_bijection(
                &|v, p| pack_block_date(*v, p),
                &unpack_block_date,
                block_date
            )
        }

        fn per_certificate_fee_pack_unpack_bijection(per_certificate_fee: PerCertificateFee) -> TestResult {
            pack_unpack_bijection(
                &pack_per_certificate_fee,
                &unpack_per_certificate_fee,
                per_certificate_fee
            )
        }

        fn per_vote_certificate_fee_pack_unpack_bijection(per_vote_certificate_fee: PerVoteCertificateFee) -> TestResult {
            pack_unpack_bijection(
                &pack_per_vote_certificate_fee,
                &unpack_per_vote_certificate_fee,
                per_vote_certificate_fee
            )
        }

        fn linear_fee_pack_unpack_bijection(linear_fee: LinearFee) -> TestResult {
            pack_unpack_bijection(
                &pack_linear_fee,
                &unpack_linear_fee,
                linear_fee
            )
        }

        fn leader_id_pack_unpack_biyection(leader_id: BftLeaderId) -> TestResult {
            pack_unpack_bijection(
                &pack_leader_id,
                &unpack_leader_id,
                leader_id
            )
        }

        fn globals_pack_unpack_bijection(globals: Globals) -> TestResult {
            pack_unpack_bijection(
                &pack_globals,
                &unpack_globals,
                globals
            )
        }

        fn ledger_static_parameters_pack_unpack_bijection(ledger_static_parameters: LedgerStaticParameters) -> TestResult {
            pack_unpack_bijection(
                &pack_ledger_static_parameters,
                &unpack_ledger_static_parameters,
                ledger_static_parameters
            )
        }

        fn pool_state_pack_unpack_bijection(pool_state: PoolState) -> TestResult {
            pack_unpack_bijection(
                &pack_pool_state,
                &unpack_pool_state,
                pool_state
            )
        }

        fn pool_last_rewards_pack_unpack_bijection(pool_last_rewards: PoolLastRewards) -> TestResult {
            pack_unpack_bijection(
                &pack_pool_last_rewards,
                &unpack_pool_last_rewards,
                pool_last_rewards
            )
        }

        fn update_proposal_state_pack_unpack_bijection(update_proposal_state: UpdateProposalState) -> TestResult {
            pack_unpack_bijection(
                &pack_update_proposal_state,
                &unpack_update_proposal_state,
                update_proposal_state
            )
        }
    }
}
