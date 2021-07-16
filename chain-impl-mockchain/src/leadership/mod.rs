use crate::{
    block::{BlockDate, BlockVersion, Header},
    certificate::PoolId,
    chaintypes::ConsensusType,
    date::Epoch,
    key::BftLeaderId,
    ledger::{Ledger, LedgerParameters},
    stake::StakeDistribution,
};
use chain_crypto::{Ed25519, RistrettoGroup2HashDh, SecretKey, SumEd25519_12};
use chain_time::era::TimeEra;

pub mod bft;
pub mod genesis;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    Failure,
    NoLeaderForThisSlot,
    IncompatibleBlockVersion,
    IncompatibleLeadershipMode,
    InvalidLeader,
    InvalidLeaderSignature,
    InvalidLeaderProof,
    InvalidBlockMessage,
    InvalidStateUpdate,
    VrfNonceIsEmptyButNotSupposedTo,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    cause: Option<Box<dyn std::error::Error + Send + Sync>>,
}

/// Verification type for when validating a block
#[derive(Debug)]
pub enum Verification {
    Success,
    Failure(Error),
}

macro_rules! try_check {
    ($x:expr) => {
        if $x.failure() {
            return $x;
        }
    };
}

pub struct BftLeader {
    pub sig_key: SecretKey<Ed25519>,
}

pub struct GenesisLeader {
    pub node_id: PoolId,
    pub sig_key: SecretKey<SumEd25519_12>,
    pub vrf_key: SecretKey<RistrettoGroup2HashDh>,
}

pub struct Leader {
    pub bft_leader: Option<BftLeader>,
    pub genesis_leader: Option<GenesisLeader>,
}

#[allow(clippy::large_enum_variant)]
pub enum LeaderOutput {
    None,
    Bft(BftLeaderId),
    GenesisPraos(PoolId, genesis::Witness),
}

pub enum LeadershipConsensus {
    Bft(bft::LeadershipData),
    GenesisPraos(genesis::LeadershipData),
}

/// Leadership represent a given epoch and their associated leader or metadata.
pub struct Leadership {
    // Specific epoch where the leadership apply
    epoch: Epoch,
    // Give the closest parameters associated with date keeping given a leadership
    era: TimeEra,
    // Consensus specific metadata required for verifying/evaluating leaders
    inner: LeadershipConsensus,
    // Ledger evaluation parameters fixed for a given epoch
    ledger_parameters: LedgerParameters,
}

impl LeadershipConsensus {
    #[inline]
    fn verify_version(&self, block_version: BlockVersion) -> Verification {
        match self {
            LeadershipConsensus::Bft(_) if block_version == BlockVersion::Ed25519Signed => {
                Verification::Success
            }
            LeadershipConsensus::GenesisPraos(_) if block_version == BlockVersion::KesVrfproof => {
                Verification::Success
            }
            _ => Verification::Failure(Error::new(ErrorKind::IncompatibleBlockVersion)),
        }
    }

    #[inline]
    fn verify_leader(&self, block_header: &Header) -> Verification {
        match self {
            LeadershipConsensus::Bft(bft) => bft.verify(block_header),
            LeadershipConsensus::GenesisPraos(genesis_praos) => genesis_praos.verify(block_header),
        }
    }

    #[inline]
    fn is_leader(&self, leader: &Leader, date: BlockDate) -> LeaderOutput {
        match self {
            LeadershipConsensus::Bft(bft) => match leader.bft_leader {
                Some(ref bft_leader) => {
                    let bft_leader_id = bft.get_leader_at(date);
                    if bft_leader_id == bft_leader.sig_key.to_public().into() {
                        LeaderOutput::Bft(bft_leader_id)
                    } else {
                        LeaderOutput::None
                    }
                }
                None => LeaderOutput::None,
            },
            LeadershipConsensus::GenesisPraos(genesis_praos) => match leader.genesis_leader {
                None => LeaderOutput::None,
                Some(ref gen_leader) => {
                    match genesis_praos.leader(&gen_leader.node_id, &gen_leader.vrf_key, date) {
                        Ok(Some(witness)) => {
                            LeaderOutput::GenesisPraos(gen_leader.node_id.clone(), witness)
                        }
                        _ => LeaderOutput::None,
                    }
                }
            },
        }
    }
}

impl Leadership {
    pub fn new(epoch: Epoch, ledger: &Ledger) -> Self {
        let inner = match ledger.settings.consensus_version {
            ConsensusType::Bft => {
                LeadershipConsensus::Bft(bft::LeadershipData::new(ledger).unwrap())
            }
            ConsensusType::GenesisPraos => {
                LeadershipConsensus::GenesisPraos(genesis::LeadershipData::new(epoch, ledger))
            }
        };
        Leadership {
            epoch,
            era: ledger.era.clone(),
            inner,
            ledger_parameters: ledger.get_ledger_parameters(),
        }
    }

    /// get the epoch associated to the `Leadership`
    #[inline]
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn stake_distribution(&self) -> Option<&StakeDistribution> {
        match &self.inner {
            LeadershipConsensus::Bft(_) => None,
            LeadershipConsensus::GenesisPraos(inner) => Some(inner.distribution()),
        }
    }

    /// Create a Block date given a leadership and a relative epoch slot
    ///
    /// # Panics
    ///
    /// If the slot index is not valid given the leadership, out of bound date
    pub fn date_at_slot(&self, slot_id: u32) -> BlockDate {
        assert!(slot_id < self.era.slots_per_epoch());
        BlockDate {
            epoch: self.epoch(),
            slot_id,
        }
    }

    /// get the TimeEra associated to the `Leadership`
    #[inline]
    pub fn era(&self) -> &TimeEra {
        &self.era
    }

    /// get the consensus associated with the `Leadership`
    pub fn consensus(&self) -> &LeadershipConsensus {
        &self.inner
    }

    /// access the ledger parameter for the current leadership
    #[inline]
    pub fn ledger_parameters(&self) -> &LedgerParameters {
        &self.ledger_parameters
    }

    /// Verify whether this header has been produced by a leader that fits with the leadership
    ///
    pub fn verify(&self, block_header: &Header) -> Verification {
        try_check!(self.inner.verify_version(block_header.block_version()));

        try_check!(self.inner.verify_leader(block_header));
        Verification::Success
    }

    /// Test that the given leader object is able to create a valid block for the leadership
    /// at a given date.
    pub fn is_leader_for_date(&self, leader: &Leader, date: BlockDate) -> LeaderOutput {
        self.inner.is_leader(leader, date)
    }
}

impl Verification {
    #[inline]
    pub fn into_error(self) -> Result<(), Error> {
        match self {
            Verification::Success => Ok(()),
            Verification::Failure(err) => Err(err),
        }
    }
    #[inline]
    pub fn success(&self) -> bool {
        matches!(self, Verification::Success)
    }
    #[inline]
    pub fn failure(&self) -> bool {
        !self.success()
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Error { kind, cause: None }
    }

    pub fn new_<E>(kind: ErrorKind, cause: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Error {
            kind,
            cause: Some(Box::new(cause)),
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ErrorKind::Failure => write!(f, "The current state of the leader selection is invalid"),
            ErrorKind::NoLeaderForThisSlot => write!(f, "No leader available for this block date"),
            ErrorKind::IncompatibleBlockVersion => {
                write!(f, "The block Version is incompatible with LeaderSelection.")
            }
            ErrorKind::IncompatibleLeadershipMode => {
                write!(f, "Incompatible leadership mode (the proof is invalid)")
            }
            ErrorKind::InvalidLeader => write!(f, "Block has unexpected block leader"),
            ErrorKind::InvalidLeaderSignature => write!(f, "Block signature is invalid"),
            ErrorKind::InvalidLeaderProof => write!(f, "Block proof is invalid"),
            ErrorKind::InvalidBlockMessage => write!(f, "Invalid block message"),
            ErrorKind::InvalidStateUpdate => write!(f, "Invalid State Update"),
            ErrorKind::VrfNonceIsEmptyButNotSupposedTo => write!(f, "Vrf Nonce is empty"),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(cause) = &self.cause {
            write!(f, "{}: {}", self.kind, cause)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl std::error::Error for Error {
    fn cause(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.cause
            .as_ref()
            .map(|cause| -> &(dyn std::error::Error + 'static) { cause.as_ref() })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::testing::TestGen;

    #[test]
    fn convensus_verify_version() {
        let ledger = TestGen::ledger();

        let data = bft::LeadershipData::new(&ledger).expect("Couldn't build leadership data");

        let bft_leadership_consensus = LeadershipConsensus::Bft(data);

        assert!(bft_leadership_consensus
            .verify_version(BlockVersion::Ed25519Signed)
            .success());
        assert!(bft_leadership_consensus
            .verify_version(BlockVersion::KesVrfproof)
            .failure());
        assert!(bft_leadership_consensus
            .verify_version(BlockVersion::Genesis)
            .failure());

        let ledger = TestGen::ledger();

        let data = genesis::LeadershipData::new(0, &ledger);

        let gen_leadership_consensus = LeadershipConsensus::GenesisPraos(data);

        assert!(gen_leadership_consensus
            .verify_version(BlockVersion::Ed25519Signed)
            .failure());
        assert!(gen_leadership_consensus
            .verify_version(BlockVersion::KesVrfproof)
            .success());
        assert!(gen_leadership_consensus
            .verify_version(BlockVersion::Genesis)
            .failure());
    }
}
