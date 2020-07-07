use super::page_manager::PageIdGenerator;
use super::transaction::{ReadTransaction, WriteTransaction};
use super::version_management::TreeIdentifier;
use super::{
    tree_algorithm, Node, PageId, Pages, PagesInitializationParams, StaticSettings, NODES_PER_PAGE,
};
use crate::btreeindex::node::NodeRef;
use crate::btreeindex::BTree;
use crate::{BTreeStoreError, FixedSize};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::borrow::Borrow;
use std::convert::{TryFrom, TryInto};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use thiserror::Error;

const STATIC_SETTINGS_FILE: &str = "static_settings";
const TREE_FILE: &str = "tree_file";

const ROOTS_FILE_PATH: &str = "roots_meta";

const NEXT_PAGE_FILE: &str = "next_page";
const FREE_PAGES_DIR_PATH: &str = "free_pages_meta";

#[derive(Error, Debug)]
pub enum TaggedTreeError {
    #[error("source tag not found")]
    SrcTagNotFound,
    #[error("destination tag is already used")]
    DstTagAlreadyExists,
}

pub struct TaggedTree<Tag, K, V>
where
    K: FixedSize,
    V: FixedSize,
    Tag: FixedSize,
{
    roots: BTree<Tag, PageId>,
    page_manager: SharedPageGenerator,
    static_settings: StaticSettings,
    pages: Arc<Pages>,
    phantom: PhantomData<(Tag, K, V)>,
}

struct PageGenerator {
    // the idea is to reuse pages somehow, but I don't know how to implement garbage collection
    // yet, and it is not really that trivial. For the moment, free pages is not used
    //
    free_pages: BTree<PageId, ()>,
    next_page: AtomicU32,
    next_page_file: File,
}

#[derive(Clone)]
struct SharedPageGenerator(pub Arc<PageGenerator>);

impl<Tag, K, V> TaggedTree<Tag, K, V>
where
    Tag: FixedSize,
    K: FixedSize,
    V: FixedSize,
{
    pub fn new(
        dir_path: impl AsRef<Path>,
        page_size: u16,
        initial_tag: Tag,
    ) -> Result<TaggedTree<Tag, K, V>, BTreeStoreError> {
        std::fs::create_dir_all(dir_path.as_ref())?;

        let mut static_settings_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(dir_path.as_ref().join(STATIC_SETTINGS_FILE))?;

        let tree_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(dir_path.as_ref().join(TREE_FILE))?;

        let pages_storage =
            crate::storage::MmapStorage::new(tree_file, page_size as u64 * NODES_PER_PAGE)?;

        let pages = Pages::new(PagesInitializationParams {
            storage: pages_storage,
            page_size: page_size.try_into().unwrap(),
        });

        let page_manager = Arc::new(PageGenerator::new(dir_path.as_ref())?);
        let roots = BTree::new(dir_path.as_ref().join(ROOTS_FILE_PATH), 4096)?;

        let first_page_id = page_manager.new_id();

        roots.insert_one(initial_tag, first_page_id)?;

        let mut root_page = pages.mut_page(first_page_id)?;

        root_page.as_slice(|page| {
            Node::<K, &mut [u8]>::new_leaf::<V>(page);
        });

        let static_settings = StaticSettings {
            page_size,
            key_buffer_size: K::max_size().try_into().unwrap(),
        };

        static_settings.write(&mut static_settings_file)?;

        Ok(TaggedTree {
            roots,
            page_manager: SharedPageGenerator(page_manager),
            static_settings,
            pages: Arc::new(pages),
            phantom: PhantomData,
        })
    }

    pub fn open(dir_path: impl AsRef<Path>) -> Result<TaggedTree<Tag, K, V>, BTreeStoreError> {
        let mut static_settings_file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(dir_path.as_ref().join(STATIC_SETTINGS_FILE))?;

        let tree_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir_path.as_ref().join(TREE_FILE))?;

        let static_settings = StaticSettings::read(&mut static_settings_file)?;

        let pages_storage = crate::storage::MmapStorage::new(
            tree_file,
            static_settings.page_size as u64 * NODES_PER_PAGE,
        )?;

        let pages = Pages::new(PagesInitializationParams {
            storage: pages_storage,
            page_size: static_settings.page_size.try_into().unwrap(),
        });

        let roots = BTree::open(dir_path.as_ref().join(ROOTS_FILE_PATH))?;

        let page_manager = Arc::new(PageGenerator::open(dir_path.as_ref())?);

        Ok(TaggedTree {
            roots,
            page_manager: SharedPageGenerator(page_manager),
            static_settings,
            pages: Arc::new(pages),
            phantom: PhantomData,
        })
    }

    pub fn write(&self, from: Tag, to: Tag) -> Result<WriteTx<Tag, K, V>, BTreeStoreError> {
        let root = self
            .roots
            .get(&from, |page_id| page_id.cloned())
            .clone()
            .ok_or(TaggedTreeError::SrcTagNotFound)?;

        let page_size = usize::try_from(self.static_settings.page_size).unwrap();

        Ok(WriteTx {
            tx: WriteTransaction::new(root, &self.pages, self.page_manager.clone()),
            roots: &self.roots,
            page_size,
            to,
            pages: Arc::clone(&self.pages),
            phantom: PhantomData,
        })
    }

    pub fn read(&self, tag: Tag) -> Result<Option<ReadTx<K>>, BTreeStoreError> {
        self.roots
            .get(&tag, |root| {
                root.map(|root| {
                    Ok(ReadTx {
                        tx: ReadTransaction::new(*root, Arc::clone(&self.pages)),
                        phantom_keys: PhantomData,
                    })
                })
            })
            .transpose()
    }

    pub fn sync(&self) -> Result<(), BTreeStoreError> {
        self.pages.sync_file()?;
        self.roots.checkpoint()?;
        self.page_manager.0.save()
    }
}

pub struct WriteTx<'a, 'b, Tag: FixedSize, K: FixedSize, V: FixedSize> {
    tx: WriteTransaction<'a, SharedPageGenerator>,
    page_size: usize,
    roots: &'b BTree<Tag, PageId>,
    to: Tag,
    pages: Arc<Pages>,
    phantom: PhantomData<(K, V)>,
}

pub struct ReadTx<K: FixedSize> {
    tx: ReadTransaction<PageId, Arc<Pages>>,
    phantom_keys: PhantomData<K>,
}

impl<'a, 'b, Tag: FixedSize, K: FixedSize, V: FixedSize> WriteTx<'a, 'b, Tag, K, V> {
    pub fn insert(&mut self, key: K, value: V) -> Result<(), BTreeStoreError> {
        tree_algorithm::insert(&mut self.tx, key, value, self.page_size)?;

        Ok(())
    }

    pub fn update(&mut self, key: &K, f: impl Fn(&V) -> V) -> Result<(), BTreeStoreError> {
        super::backtrack::UpdateBacktrack::new_search_for(&mut self.tx, key).update(f)?;

        Ok(())
    }

    pub fn update_or_default(
        &mut self,
        key: &K,
        default: V,
        update_with: impl Fn(&V) -> V,
    ) -> Result<(), BTreeStoreError> {
        let updated = self.update(key, update_with);
        if let Err(BTreeStoreError::KeyNotFound) = updated {
            self.insert(key.clone(), default)?;
        }

        Ok(())
    }

    pub fn commit(self) -> Result<ReadTx<K>, BTreeStoreError> {
        let delta = self.tx.commit::<K>();

        self.roots
            .insert_one(self.to, delta.new_root)
            .map_err(|err| match err {
                BTreeStoreError::DuplicatedKey => {
                    BTreeStoreError::TaggedTree(TaggedTreeError::DstTagAlreadyExists)
                }
                err => err,
            })?;

        Ok(ReadTx {
            tx: ReadTransaction::new(delta.new_root, self.pages),
            phantom_keys: PhantomData,
        })
    }
}

impl<K: FixedSize> ReadTx<K> {
    pub fn get<V, Q, F, R>(&self, key: &Q, f: F) -> R
    where
        Q: Ord,
        K: Borrow<Q>,
        V: FixedSize,
        F: FnOnce(Option<&V>) -> R,
    {
        let page_ref = tree_algorithm::search::<PageId, K, Q, Arc<Pages>>(&self.tx, key);

        page_ref.as_node(|node: Node<K, &[u8]>| {
            match node.as_leaf::<V>().keys().binary_search::<Q>(key) {
                Ok(pos) => f(Some(node.as_leaf::<V>().values().get(pos).borrow())),
                Err(_) => f(None),
            }
        })
    }
}

impl PageIdGenerator for SharedPageGenerator {
    fn next_id(&self) -> PageId {
        PageGenerator::next_id(&self.0)
    }

    fn new_id(&mut self) -> PageId {
        PageGenerator::new_id(&self.0)
    }
}

impl PageGenerator {
    fn new(dir_path: impl AsRef<Path>) -> Result<PageGenerator, BTreeStoreError> {
        let free_pages = BTree::new(dir_path.as_ref().join(FREE_PAGES_DIR_PATH), 4096)?;
        let next_page = AtomicU32::new(1);

        let mut next_page_file = OpenOptions::new()
            .read(true)
            .create(true)
            .write(true)
            .open(dir_path.as_ref().join(NEXT_PAGE_FILE))?;

        next_page_file.write_u32::<LittleEndian>(1)?;

        Ok(PageGenerator {
            free_pages,
            next_page,
            next_page_file,
        })
    }

    fn open(dir_path: impl AsRef<Path>) -> Result<PageGenerator, BTreeStoreError> {
        let free_pages = BTree::open(dir_path.as_ref().join(FREE_PAGES_DIR_PATH))?;

        let mut next_page_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(dir_path.as_ref().join(NEXT_PAGE_FILE))?;

        let next_page = next_page_file
            .read_u32::<LittleEndian>()
            .expect("Couldn't read next page id");

        Ok(PageGenerator {
            free_pages,
            next_page: AtomicU32::new(next_page),
            next_page_file,
        })
    }

    fn next_id(&self) -> PageId {
        self.next_page.load(std::sync::atomic::Ordering::Acquire)
    }

    fn new_id(&self) -> PageId {
        let next = self.free_pages.pop_max().expect("pop max shouldn't error");

        next.map(|(key, _)| key)
            .unwrap_or_else(|| self.next_page.fetch_add(1, Ordering::Relaxed))
    }

    fn save(&self) -> Result<(), BTreeStoreError> {
        let next_page = self.next_page.load(Ordering::SeqCst);

        self.next_page_file
            .try_clone()
            .unwrap()
            .write_all(&next_page.to_le_bytes())
            .expect("Can't save next_page");

        self.free_pages.checkpoint()?;

        Ok(())
    }
}

impl TreeIdentifier for PageId {
    fn root(&self) -> PageId {
        *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::U64Key;
    use tempfile::tempdir;

    extern crate rand;
    extern crate tempfile;

    const INITIAL_TAG: u64 = 0;

    fn new_tree() -> TaggedTree<U64Key, U64Key, U64Key> {
        let dir_path = tempdir().unwrap();

        let page_size = 88;

        let tree: TaggedTree<U64Key, U64Key, U64Key> =
            TaggedTree::new(dir_path.path(), page_size, U64Key(INITIAL_TAG)).unwrap();

        tree
    }

    use model::*;
    #[quickcheck]
    fn test_scalar_add(ops: Vec<Op>) -> bool {
        let db = new_tree();
        let mut reference = Reference::default();

        for op in ops {
            match op {
                Op::Write { from, to, op } => {
                    let ref_result = reference.write(from, to, |old| {
                        let mut new = old.clone();
                        match op {
                            WriteOp::InsertOrAdd { key, default, add } => {
                                if let Some(value) = new.get_mut(&key) {
                                    *value += add;
                                } else {
                                    new.insert(key, default);
                                }
                            }
                        }

                        new
                    });

                    let wtx = db.write(from.into(), to.into());

                    let mut wtx = if let Err(TaggedTreeError::SrcTagNotFound) = ref_result {
                        assert!(wtx.is_err());
                        continue;
                    } else {
                        wtx.unwrap()
                    };

                    match op {
                        WriteOp::InsertOrAdd { key, default, add } => {
                            wtx.update_or_default(&U64Key(key), default.into(), |old| {
                                U64Key::from(old.0 + add)
                            })
                            .unwrap();
                        }
                    }
                    let commit = wtx.commit();

                    if let Err(TaggedTreeError::DstTagAlreadyExists) = ref_result {
                        assert!(commit.is_err());
                        continue;
                    } else {
                        assert!(commit.is_ok());
                    };
                }
                Op::Read { from, keys } => {
                    let rtx = db.read(from.into()).unwrap();
                    let pair = reference.read(from).map(|from| (from, rtx.unwrap()));

                    if let Some((refversion, dbversion)) = pair {
                        for k in keys {
                            assert_eq!(
                                refversion.get(&k).cloned(),
                                dbversion.get(&U64Key(k), |iv| iv.map(|n: &U64Key| n.0))
                            );
                        }
                    }
                }
            }
        }

        true
    }

    #[test]
    fn is_send() {
        fn is_send<T: Send>() {}

        is_send::<TaggedTree<U64Key, U64Key, u64>>();
    }

    #[test]
    fn is_sync() {
        fn is_sync<T: Sync>() {}

        is_sync::<TaggedTree<U64Key, U64Key, u64>>();
    }

    mod model {
        use super::super::TaggedTreeError;
        use quickcheck::{Arbitrary, Gen};
        use rand::Rng;
        use std::collections::BTreeMap;

        pub struct Reference {
            versions: BTreeMap<u64, BTreeMap<u64, u64>>,
        }

        impl Reference {
            pub fn write<F>(
                &mut self,
                from: u64,
                to: u64,
                f: F,
            ) -> Result<&BTreeMap<u64, u64>, TaggedTreeError>
            where
                F: Fn(&BTreeMap<u64, u64>) -> BTreeMap<u64, u64>,
            {
                let base = &self
                    .versions
                    .get(&from)
                    .ok_or(TaggedTreeError::SrcTagNotFound)?;

                if self.versions.get(&to).is_some() {
                    return Err(TaggedTreeError::DstTagAlreadyExists);
                }

                let new = f(&base);
                self.versions.insert(to, new);

                Ok(&self.versions[&to])
            }

            pub fn read(&mut self, tag: u64) -> Option<&BTreeMap<u64, u64>> {
                self.versions.get(&tag)
            }
        }

        impl Default for Reference {
            fn default() -> Reference {
                let mut versions = <BTreeMap<u64, BTreeMap<u64, u64>>>::new();

                versions.insert(super::INITIAL_TAG, <BTreeMap<u64, u64>>::new());

                Reference { versions }
            }
        }

        const MAX_TAG: u64 = 25;
        const MAX_DEFAULT: u64 = 5;
        const MAX_ADD: u64 = 10;
        const MAX_KEY: u64 = 25;

        #[derive(Clone, Debug)]
        pub enum Op {
            Write { from: u64, to: u64, op: WriteOp },
            Read { from: u64, keys: Vec<u64> },
        }

        #[derive(Clone, Debug)]
        pub enum WriteOp {
            InsertOrAdd { key: u64, default: u64, add: u64 },
        }

        impl Arbitrary for Op {
            fn arbitrary<G: Gen>(g: &mut G) -> Op {
                match g.gen_range(0, 2) {
                    0 => {
                        let from = g.gen_range(0, MAX_TAG);
                        let to = g.gen_range(0, MAX_TAG);

                        let op = <WriteOp as Arbitrary>::arbitrary(g);

                        Op::Write { from, to, op }
                    }

                    1 => {
                        let from = g.gen_range(0, MAX_TAG);
                        let keys = Vec::<u64>::arbitrary(g);

                        Op::Read { from, keys }
                    }
                    _ => unreachable!(),
                }
            }
        }

        impl Arbitrary for WriteOp {
            fn arbitrary<G: Gen>(g: &mut G) -> WriteOp {
                let key = g.gen_range(0, MAX_KEY);
                let default = g.gen_range(0, MAX_DEFAULT);
                let add = g.gen_range(0, MAX_ADD);

                WriteOp::InsertOrAdd { key, default, add }
            }
        }
    }
}
