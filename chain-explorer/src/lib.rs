pub mod chain_storable;
mod endian;
pub mod error;
mod helpers;
pub mod pagination;
mod pair;
pub mod schema;
mod seq;
mod state_ref;

use self::error::DbError;
use chain_core::property::Block as _;
use chain_impl_mockchain::block::Block;
use chain_impl_mockchain::block::HeaderId as HeaderHash;
use sanakirja::btree;
use std::path::Path;
use std::sync::Arc;

pub use seq::SeqNum;

pub(crate) type P<K, V> = btree::page::Page<K, V>;
type Db<K, V> = btree::Db<K, V>;

type SanakirjaMutTx = ::sanakirja::MutTxn<Arc<::sanakirja::Env>, ()>;
type SanakirjaTx = ::sanakirja::Txn<Arc<::sanakirja::Env>>;

#[derive(Clone)]
pub struct ExplorerDb {
    pub env: Arc<::sanakirja::Env>,
}

pub enum OpenDb {
    Initialized {
        db: ExplorerDb,
        last_stable_block: HeaderHash,
        branches: Vec<HeaderHash>,
    },
    NeedsBootstrap(NeedsBootstrap),
}

pub struct NeedsBootstrap(ExplorerDb);

pub struct Batch {
    txn: schema::MutTxn<()>,
}

impl Batch {
    /// Try to add a new block to the indexes, this can fail if the parent of the block is not
    /// processed. This doesn't perform any validation on the given block and the previous state,
    /// it is assumed that the Block is valid
    pub fn apply_block(&mut self, block: Block) -> Result<(), DbError> {
        self.txn.add_block(
            &block.parent_id().into(),
            &block.id().into(),
            block.chain_length().into(),
            block.header.block_date().into(),
            block.fragments(),
        )?;

        Ok(())
    }

    pub fn commit(self) -> Result<(), DbError> {
        self.txn.commit()
    }
}

impl NeedsBootstrap {
    pub fn add_block0(self, block0: Block) -> Result<ExplorerDb, DbError> {
        let db = self.0;
        let mut mut_tx = db.mut_txn_begin()?;

        let parent_id = block0.parent_id();
        let block_id = block0.id();

        mut_tx.add_block0(&parent_id.into(), &block_id.into(), block0.contents.iter())?;

        mut_tx.commit()?;

        Ok(db)
    }
}

impl ExplorerDb {
    pub fn open<P: AsRef<Path>>(storage: Option<P>) -> Result<OpenDb, DbError> {
        let db = match storage {
            Some(path) => ExplorerDb::new(path),
            None => ExplorerDb::new_anon(),
        }?;

        let txn = db.txn_begin();

        match txn {
            Ok(txn) => {
                let chain_length = txn.get_stable_chain_length();
                let block = txn
                    .get_blocks_by_chain_length(&chain_length)?
                    .next()
                    .transpose()?
                    .ok_or(DbError::MissingBlock)?;

                let branches = txn
                    .get_branches()?
                    .map(|b| b.map(|id| HeaderHash::from(*id)))
                    .collect::<Result<_, DbError>>()?;

                Ok(OpenDb::Initialized {
                    last_stable_block: HeaderHash::from(*block),
                    branches,
                    db,
                })
            }
            Err(DbError::UnitializedDatabase) => Ok(OpenDb::NeedsBootstrap(NeedsBootstrap(db))),
            Err(e) => Err(e),
        }
    }

    /// Try to add a new block to the indexes, this can fail if the parent of the block is not
    /// processed. This doesn't perform any validation on the given block and the previous state,
    /// it is assumed that the Block is valid
    pub fn apply_block(&self, block: Block) -> Result<(), DbError> {
        let db = self.clone();
        let mut_tx = db.mut_txn_begin()?;

        let mut batch = Batch { txn: mut_tx };

        batch.apply_block(block)?;

        batch.commit()?;

        Ok(())
    }

    pub fn start_batch(&self) -> Result<Batch, DbError> {
        let mut_tx = self.mut_txn_begin()?;

        Ok(Batch { txn: mut_tx })
    }

    pub fn set_tip(&self, hash: HeaderHash) -> Result<bool, DbError> {
        let mut mut_tx = self.mut_txn_begin()?;

        let status = mut_tx.set_tip(&hash.into())?;

        if status {
            mut_tx.commit()?;
        }

        Ok(status)
    }

    fn new<P: AsRef<Path>>(name: P) -> Result<Self, DbError> {
        Self::new_with_size(name, 1 << 20)
    }

    fn new_with_size<P: AsRef<Path>>(name: P, size: u64) -> Result<Self, DbError> {
        let env = ::sanakirja::Env::new(name, size, 2);
        match env {
            Ok(env) => Ok(Self { env: Arc::new(env) }),
            Err(e) => Err(DbError::SanakirjaError(e)),
        }
    }

    fn new_anon() -> Result<Self, DbError> {
        Self::new_anon_with_size(1 << 20)
    }

    fn new_anon_with_size(size: u64) -> Result<Self, DbError> {
        Ok(Self {
            env: Arc::new(::sanakirja::Env::new_anon(size, 2)?),
        })
    }
}
