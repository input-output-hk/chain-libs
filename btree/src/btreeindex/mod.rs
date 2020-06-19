mod backtrack;
mod metadata;
// FIXME: allow dead code momentarily, because all of the delete algorithms are unused, and placing the directive with more granularity would be too troublesome
mod iter;
pub mod multitree;
mod node;
mod page_manager;
mod pages;
mod tree_algorithm;
mod version_management;

use version_management::transaction::{PageRef, ReadTransaction};
use version_management::*;

use crate::BTreeStoreError;
use metadata::{Metadata, StaticSettings};
use node::{Node, NodeRef};
use pages::{Pages, PagesInitializationParams};
use std::borrow::Borrow;

use crate::FixedSize;

use backtrack::UpdateBacktrack;
use std::convert::{TryFrom, TryInto};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::marker::PhantomData;
use std::ops::RangeBounds;
use std::path::Path;
use std::sync::Mutex;

use iter::BTreeIterator;

pub type PageId = u32;
const NODES_PER_PAGE: u64 = 2000;

pub struct BTree<K, V> {
    // The metadata file contains the latests confirmed version of the tree
    // this is, the root node, and the list of free pages
    metadata: Mutex<(Metadata, File)>,
    static_settings: StaticSettings,
    pages: Pages,
    transaction_manager: TransactionManager,
    phantom_keys: PhantomData<[K]>,
    phantom_values: PhantomData<[V]>,
}

/// Views over continous arrays of data. The buffer represents the total capacity
/// but they keep track of the current actual length of items
use crate::arrayview::ArrayView;
pub(crate) type Children<'a> = ArrayView<'a, &'a [u8], PageId>;
pub(crate) type ChildrenMut<'a> = ArrayView<'a, &'a mut [u8], PageId>;
pub(crate) type Values<'a, V> = ArrayView<'a, &'a [u8], V>;
pub(crate) type ValuesMut<'a, V> = ArrayView<'a, &'a mut [u8], V>;
pub(crate) type Keys<'a, K> = ArrayView<'a, &'a [u8], K>;
pub(crate) type KeysMut<'a, K> = ArrayView<'a, &'a mut [u8], K>;

impl<'me, K: 'me, V> BTree<K, V>
where
    K: FixedSize,
    V: FixedSize,
{
    // TODO: add a builder with defaults?
    pub fn new(
        metadata_file: File,
        tree_file: File,
        mut static_settings_file: File,
        page_size: u16,
        key_buffer_size: u32,
    ) -> Result<BTree<K, V>, BTreeStoreError> {
        let mut metadata = Metadata::new();

        let pages_storage =
            crate::storage::MmapStorage::new(tree_file, page_size as u64 * NODES_PER_PAGE)?;

        let pages = Pages::new(PagesInitializationParams {
            storage: pages_storage,
            page_size: page_size.try_into().unwrap(),
        });

        let first_page_id = metadata.page_manager.new_id();

        let mut root_page = pages.mut_page(first_page_id)?;

        root_page.as_slice(|page| {
            Node::<K, &mut [u8]>::new_leaf::<V>(page);
        });

        metadata.set_root(first_page_id);

        let static_settings = StaticSettings {
            page_size,
            key_buffer_size,
        };

        static_settings.write(&mut static_settings_file)?;

        let transaction_manager = TransactionManager::new(&metadata);

        Ok(BTree {
            metadata: Mutex::new((metadata, metadata_file)),
            pages,
            static_settings,
            transaction_manager,
            phantom_keys: PhantomData,
            phantom_values: PhantomData,
        })
    }

    pub fn open(
        metadata_file: impl AsRef<Path>,
        tree_file: impl AsRef<Path>,
        static_settings_file: impl AsRef<Path>,
    ) -> Result<BTree<K, V>, BTreeStoreError> {
        let mut static_settings_file = OpenOptions::new()
            .write(true)
            .read(true)
            .open(static_settings_file)?;

        let mut metadata_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(metadata_file)?;

        let metadata = Metadata::read(&mut metadata_file)?;

        let static_settings = StaticSettings::read(&mut static_settings_file)?;

        let tree_file = OpenOptions::new().write(true).read(true).open(tree_file)?;
        let pages_storage = crate::storage::MmapStorage::new(
            tree_file,
            static_settings.page_size as u64 * NODES_PER_PAGE,
        )?;

        let pages = Pages::new(PagesInitializationParams {
            storage: pages_storage,
            page_size: static_settings.page_size,
        });

        let transaction_manager = TransactionManager::new(&metadata);

        Ok(BTree {
            metadata: Mutex::new((metadata, metadata_file)),
            pages,
            static_settings,
            transaction_manager,
            phantom_keys: PhantomData,
            phantom_values: PhantomData,
        })
    }

    // sync files to disk and collect old transactions pages
    pub(crate) fn checkpoint(&self) -> Result<(), BTreeStoreError> {
        if let Some(checkpoint) = self.transaction_manager.collect_pending() {
            let new_metadata = checkpoint.new_metadata;

            self.pages.sync_file()?;

            let mut guard = self.metadata.lock().unwrap();
            let (_metadata, metadata_file) = &mut *guard;

            metadata_file.seek(SeekFrom::Start(0))?;

            new_metadata.write(metadata_file)?;
            metadata_file.sync_all()?;

            // this part is not actually important
            guard.0 = new_metadata;
        }
        Ok(())
    }

    pub fn insert_async(
        &self,
        iter: impl IntoIterator<Item = (K, V)>,
    ) -> Result<(), BTreeStoreError> {
        self.transaction_manager
            .with_write_transaction(&self.pages, |mut tx| {
                let page_size = usize::try_from(self.static_settings.page_size).unwrap();

                for (key, value) in iter {
                    tree_algorithm::insert(&mut tx, key, value, page_size)?;
                }

                Ok(tx.commit::<K>())
            })
    }

    pub fn insert_one(&self, key: K, value: V) -> Result<(), BTreeStoreError> {
        self.insert_async(vec![(key, value)])?;

        self.checkpoint()?;

        Ok(())
    }

    pub fn insert_many(
        &self,
        iter: impl IntoIterator<Item = (K, V)>,
    ) -> Result<(), BTreeStoreError> {
        self.insert_async(iter)?;

        self.checkpoint()?;
        Ok(())
    }

    // we use a function for the return value in order to avoid cloning the value, returning a direct reference is not possible because we need
    // the ReadTransaction to exist in order to keep the page from being reused.
    pub fn get<Q, F, R>(&self, key: &Q, f: F) -> R
    where
        Q: Ord,
        K: Borrow<Q>,
        F: FnOnce(Option<&V>) -> R,
    {
        let read_transaction = self.transaction_manager.read_transaction(&self.pages);

        let page_ref = self.search(&read_transaction, key);

        page_ref.as_node(|node: Node<K, &[u8]>| {
            match node.as_leaf::<V>().keys().binary_search::<Q>(key) {
                // TODO: Find if it is possible to avoid this clone (although it's only important if V is a big type, which should be avoided anyway)
                Ok(pos) => f(Some(node.as_leaf::<V>().values().get(pos).borrow())),
                Err(_) => f(None),
            }
        })
    }

    /// perform a range query. The returned iterator holds a read-only transaction for it's entire lifetime.
    /// This avoids pages to be collected, so it may better for it to not be long-lived.
    pub fn range<R, Q>(&self, range: R) -> BTreeIterator<R, Q, K, V>
    where
        K: Borrow<Q>,
        R: RangeBounds<Q>,
        Q: Ord,
    {
        let read_transaction = self.transaction_manager.read_transaction(&self.pages);

        BTreeIterator::new(read_transaction, range)
    }

    fn search<'a, Q>(&'a self, tx: &'a ReadTransaction, key: &Q) -> PageRef<'a>
    where
        Q: Ord,
        K: Borrow<Q>,
    {
        tree_algorithm::search::<K, Q>(tx, key)
    }

    pub fn update(&self, key: &K, value: V) -> Result<(), BTreeStoreError> {
        self.transaction_manager
            .with_write_transaction(&self.pages, |mut tx| {
                UpdateBacktrack::new_search_for(&mut tx, key).update(value)?;

                Ok(tx.commit::<K>())
            })
    }

    /// delete given key from the tree, this doesn't sync the file to disk
    pub fn delete<'a, 'b: 'a>(&'a self, key: &'b K) -> Result<(), BTreeStoreError> {
        self.transaction_manager
            .with_write_transaction(&self.pages, |mut tx| {
                tree_algorithm::delete::<K, V, _>(key, &mut tx)?;

                Ok(tx.commit::<K>())
            })
    }
}

impl<K, V> Drop for BTree<K, V> {
    fn drop(&mut self) {
        let mut guard = self.metadata.lock().unwrap();
        let (metadata, metadata_file) = &mut *guard;

        metadata_file.seek(SeekFrom::Start(0)).unwrap();
        metadata.write(metadata_file).unwrap();

        self.pages.sync_file().expect("tree file sync failed");
    }
}

#[cfg(test)]
mod tests {
    extern crate rand;
    extern crate tempfile;
    use super::*;
    use crate::tests::U64Key;
    use crate::FixedSize;
    use std::sync::Arc;
    use tempfile::tempfile;

    impl<K> BTree<K, u64>
    where
        K: FixedSize,
    {
        fn key_buffer_size(&self) -> u32 {
            self.static_settings.key_buffer_size
        }

        fn page_size(&self) -> u16 {
            self.static_settings.page_size
        }

        pub fn debug_print(&self) {
            let read_tx = self.transaction_manager.read_transaction(&self.pages);
            let root_id = read_tx.root();

            // TODO: get the next page but IN the read transaction
            for n in 1..self.metadata.lock().unwrap().0.page_manager.next_page {
                let pages = &self.pages;
                let page_ref = pages.get_page(n).unwrap();

                println!("-----------------------");
                println!("PageId: {}", n);

                if n == root_id {
                    println!("ROOT");
                }

                page_ref.as_node(|node: Node<K, &[u8]>| match node.get_tag() {
                    node::NodeTag::Internal => {
                        println!("Internal Node");
                        println!("keys: ");
                        for k in node.as_internal().keys().iter() {
                            println!("{:?}", k.borrow());
                        }
                        println!("children: ");
                        for c in node.as_internal().children().iter() {
                            println!("{:?}", c.borrow());
                        }
                    }
                    node::NodeTag::Leaf => {
                        println!("Leaf Node");
                        println!("keys: ");
                        for k in node.as_leaf::<u64>().keys().iter() {
                            println!("{:?}", k.borrow());
                        }
                        println!("values: ");
                        for v in node.as_leaf::<u64>().values().iter() {
                            println!("{:?}", v.borrow());
                        }
                    }
                });
                println!("-----------------------");
            }
        }
    }

    pub fn new_tree() -> BTree<U64Key, u64> {
        let metadata_file = tempfile().unwrap();
        let tree_file = tempfile().unwrap();
        let static_file = tempfile().unwrap();

        let page_size = 88;

        let tree: BTree<U64Key, u64> = BTree::new(
            metadata_file,
            tree_file,
            static_file,
            page_size,
            size_of::<U64Key>().try_into().unwrap(),
        )
        .unwrap();

        tree
    }

    use std::mem::size_of;
    #[test]
    fn insert_many() {
        let tree = new_tree();

        let n: u64 = 2000;

        tree.insert_many((0..n).map(|i| (U64Key(i), i))).unwrap();

        // tree.debug_print();

        for i in 0..n {
            assert_eq!(
                tree.get(&U64Key(i), |key| key.cloned())
                    .expect("Key not found"),
                i
            );
        }
    }

    #[quickcheck]
    fn qc_inserted_keys_are_found(xs: Vec<(u64, u64)>) -> bool {
        println!("start qc test");
        let mut reference = std::collections::BTreeMap::new();

        let tree = new_tree();

        // we insert first in the reference in order to get rid of duplicates
        for (xk, xv) in xs {
            reference.entry(xk.clone()).or_insert(xv);
        }

        tree.insert_many(reference.iter().map(|(k, v)| (U64Key(*k), *v)))
            .unwrap();

        reference
            .iter()
            .all(|(k, v)| match tree.get(&U64Key(*k), |v| v.cloned()) {
                Some(l) => *v == l,
                None => false,
            })
    }

    #[test]
    fn saves_and_restores_right() {
        let key_buffer_size: u32 = size_of::<U64Key>().try_into().unwrap();
        let page_size = 86u16;
        {
            let metadata_file = OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open("metadata")
                .expect("Couldn't create metadata file");

            let tree_file = OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open("tree")
                .expect("Couldn't create pages file");

            let static_file = OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open("static")
                .expect("Couldn't create pages file");

            BTree::<U64Key, u64>::new(
                metadata_file,
                tree_file,
                static_file,
                page_size,
                key_buffer_size,
            )
            .unwrap();
        }

        {
            let restored_tree =
                BTree::<U64Key, u64>::open("metadata", "tree", "static").expect("restore to work");
            assert_eq!(restored_tree.key_buffer_size(), key_buffer_size);
            assert_eq!(restored_tree.page_size(), page_size);
        }

        std::fs::remove_file("tree").unwrap();
        std::fs::remove_file("metadata").unwrap();
        std::fs::remove_file("static").unwrap();
    }

    #[test]
    fn multireads() {
        let tree = new_tree();
        let n: u64 = 2000;

        tree.insert_many((0u64..n).map(|i| (U64Key(i), i))).unwrap();

        for i in 0..n {
            assert_eq!(
                tree.get(&U64Key(i), |value| value.cloned())
                    .expect("Key not found"),
                i
            );
        }

        use rand::seq::SliceRandom;
        use std::sync::Barrier;
        use std::thread;

        let mut handles = Vec::with_capacity(10);
        let barrier = Arc::new(Barrier::new(10));
        let index = Arc::new(tree);

        for _ in 0..10 {
            let c = barrier.clone();

            let index = index.clone();

            handles.push(thread::spawn(move || {
                let mut queries: Vec<u64> = (0..n).collect();
                let mut rng = rand::thread_rng();

                queries.shuffle(&mut rng);
                c.wait();
                for i in queries {
                    assert_eq!(
                        index
                            .get(&U64Key(i), |v| v.cloned())
                            .expect("Key not found"),
                        i
                    );
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    #[ignore]
    fn multiwrites() {
        let tree = new_tree();

        use rand::seq::SliceRandom;
        use std::sync::{Arc, Barrier};
        use std::thread;

        let mut read_handles = Vec::with_capacity(3);
        let mut write_handles = Vec::with_capacity(3);
        let barrier = Arc::new(Barrier::new(3));
        let index = Arc::new(tree);

        let n = 3000;
        let num_write_threads = 3;
        for thread_num in 0..num_write_threads {
            let c = barrier.clone();
            let index = index.clone();

            write_handles.push(thread::spawn(move || {
                let mut inserts: Vec<u64> = ((n * thread_num)..n * (thread_num + 1)).collect();
                let mut rng = rand::thread_rng();
                inserts.shuffle(&mut rng);
                c.wait();

                for i in inserts {
                    index
                        .insert_async(Some((U64Key(i), i)))
                        .expect("duplicated insert in disjoint threads");
                }
            }));
        }

        for thread_num in 0..3 {
            let index = index.clone();

            read_handles.push(thread::spawn(move || {
                // just to make some noise
                while index
                    .get(&U64Key(thread_num * n + 500), |v| v.cloned())
                    .is_none()
                {}
            }));
        }

        for handle in write_handles {
            handle.join().unwrap();
        }

        for handle in read_handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn can_delete_key() {
        let tree = new_tree();
        let n: u64 = 2000;
        let delete: u64 = 50;

        tree.insert_many((0..n).map(|i| (U64Key(i), i))).unwrap();

        let key_to_delete = U64Key(delete);
        assert!(tree.get(&key_to_delete, |v| v.cloned()).is_some());

        tree.delete(&key_to_delete).unwrap();

        dbg!("tree after");
        tree.debug_print();

        assert!(dbg!(tree.get(&key_to_delete, |v| v.cloned())).is_none());

        for i in (0..n).filter(|n| *n != delete) {
            assert!(tree.get(&U64Key(i), |v| v.cloned()).is_some());
        }
    }

    #[quickcheck]
    #[ignore]
    fn qc_arbitrary_deletes(xs: Vec<u64>) -> bool {
        let mut reference = std::collections::BTreeMap::new();

        let tree = new_tree();
        let n: u64 = 2000;
        for i in 0..n {
            reference.entry(U64Key(i)).or_insert(i);
        }

        tree.insert_many(reference.iter().map(|(k, v)| (k.clone(), *v)))
            .unwrap();

        for k in xs {
            reference.remove(&U64Key(k));
            tree.delete(&U64Key(k)).unwrap_or(());
            assert!(tree.get(&U64Key(k), |v| v.cloned()).is_none());
        }

        reference
            .iter()
            .all(|(k, v)| match tree.get(k, |v| v.cloned()) {
                Some(l) => *v == l,
                None => false,
            })
    }

    #[test]
    fn test_update() {
        let tree = new_tree();

        let n: u64 = 2000;

        tree.insert_many((0..n).map(|i| (U64Key(i), i))).unwrap();

        assert_eq!(tree.get(&U64Key(100), |v| v.cloned()), Some(100));

        tree.update(&U64Key(100), 120).unwrap();

        assert_eq!(tree.get(&U64Key(100), |v| v.cloned()), Some(120));
    }

    use crate::Storeable;
    impl<'a> Storeable<'a> for () {
        type Error = std::io::Error;
        type Output = Self;

        fn write(&self, _buf: &mut [u8]) -> Result<(), Self::Error> {
            Ok(())
        }

        fn read(_buf: &'a [u8]) -> Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    impl FixedSize for () {
        fn max_size() -> usize {
            0
        }
    }

    #[test]
    fn zero_size_value() {
        let metadata_file = tempfile().unwrap();
        let tree_file = tempfile().unwrap();
        let static_file = tempfile().unwrap();

        let page_size = 88;

        let tree: BTree<U64Key, ()> = BTree::new(
            metadata_file,
            tree_file,
            static_file,
            page_size,
            size_of::<U64Key>().try_into().unwrap(),
        )
        .unwrap();

        let n: u64 = 2000;

        tree.insert_many((0..n).map(|i| (U64Key(i), ()))).unwrap();

        // tree.debug_print();

        for i in 0..n {
            assert_eq!(
                tree.get(&U64Key(i), |key| key.cloned())
                    .expect("Key not found"),
                ()
            );
        }

        for i in (0..n).step_by(2) {
            tree.delete(&U64Key(dbg!(i))).unwrap();
        }

        for i in (1..n).step_by(2) {
            assert_eq!(
                tree.get(&U64Key(dbg!(i)), |key| key.cloned())
                    .expect("Key not found"),
                ()
            );
        }
    }

    #[test]
    fn is_send() {
        // test (at compile time) that certain types implement the auto-trait Send, either directly for
        // pointer-wrapping types or transitively for types with all Send fields

        fn is_send<T: Send>() {
            // dummy function just used for its parameterized type bound
        }

        is_send::<BTree<U64Key, u64>>();
    }
    #[test]
    fn is_sync() {
        // test (at compile time) that certain types implement the auto-trait Sync

        fn is_sync<T: Sync>() {
            // dummy function just used for its parameterized type bound
        }

        is_sync::<BTree<U64Key, u64>>();
    }
}
