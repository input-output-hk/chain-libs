/// Helpers to keep track of parent pointers and siblings when traversing the tree.
use super::transaction;
use super::transaction::{MutablePage, PageRef, PageRefMut, WriteTransaction};
use crate::btreeindex::node::{InternalNode, NodeRefMut};
use crate::btreeindex::{node::NodeRef, page_manager::PageIdGenerator, Node, PageId};
use crate::mem_page::MemPage;
use crate::FixedSize;
use std::marker::PhantomData;

/// this is basically a stack, but it will rename pointers and interact with the transaction in order to reuse
/// already cloned pages
pub(crate) struct InsertBacktrack<'txbuilder, 'txmanager: 'txbuilder, K, G: PageIdGenerator>
where
    K: FixedSize,
{
    tx: &'txbuilder mut transaction::WriteTransaction<'txmanager, G>,
    backtrack: Vec<PageId>,
    new_root: Option<PageId>,
    phantom_key: PhantomData<[K]>,
}

/// this is basically a stack, but it will rename pointers and interact with the transaction in order to reuse
/// already cloned pages
pub(crate) struct DeleteBacktrack<'txbuilder, 'txmanager: 'txbuilder, K, G: PageIdGenerator>
where
    K: FixedSize,
{
    tx: &'txbuilder transaction::WriteTransaction<'txmanager, G>,
    backtrack: Vec<PageId>,
    // The first parameter is the anchor used to get from the parent to the node in the top of the stack
    // the other two are both its siblings. The parent id can be found after the top of the stack, of course.
    parent_info: Vec<(Option<usize>, Option<PageId>, Option<PageId>)>,
    new_root: Option<PageId>,
    phantom_key: PhantomData<[K]>,
}

pub(crate) struct DeleteNextElement<'a, 'b: 'a, 'c: 'b, K, G: PageIdGenerator>
where
    K: FixedSize,
{
    pub(crate) next_element: NextElement<'a, 'b, K, G>,
    pub(crate) mut_context: MutableContext<'a, 'b, 'c, K, G>,
}

/// this is basically a stack, but it will rename pointers and interact with the transaction in order to reuse
/// already cloned pages
pub(crate) struct UpdateBacktrack<'txbuilder, 'txmanager: 'txbuilder, K, G: PageIdGenerator>
where
    K: FixedSize,
{
    tx: &'txbuilder mut transaction::WriteTransaction<'txmanager, G>,
    backtrack: Vec<PageId>,
    key_to_update: K,
    new_root: Option<PageId>,
    phantom_key: PhantomData<[K]>,
}

// lifetimes on this are a bit bothersome with four 'linear' (re) borrows, there may be some way of refactoring this things, but that would probably need to be done higher in
// the hierarchy
/// type to operate on the current element in the stack (branch) of nodes. This borrows the backtrack and acts as a proxy, in order to make borrowing simpler, because
// XXX: having a left sibling means anchor is not None, and having a sibling in general means parent is not None also, maybe this invariants could be expressed in the type structure
pub(crate) struct NextElement<'a, 'b: 'a, K, G: PageIdGenerator>
where
    K: FixedSize,
{
    pub(crate) next: PageRefMut<'a>,
    // anchor is an index into the keys array of a node used to find the current node in the parent without searching. The leftmost(lowest) child has None as anchor
    // this means it's inmediate right sibling would have anchor of 0, and so on.
    pub(crate) anchor: Option<usize>,
    pub(crate) left: Option<PageRef<'a>>,
    pub(crate) right: Option<PageRef<'a>>,
    backtrack: &'a DeleteBacktrack<'a, 'b, K, G>,
}

pub(crate) enum MutableContext<'a, 'b: 'a, 'c: 'b, K: FixedSize, G: PageIdGenerator> {
    NonRoot(DeleteContext<'a, 'b, 'c, K, NonRoot<'a>, G>),
    Root(DeleteContext<'a, 'b, 'c, K, RootNode, G>),
}

pub(crate) struct NonRoot<'a> {
    parent: PageRefMut<'a>,
    left_id: Option<PageId>,
    right_id: Option<PageId>,
}

pub(crate) struct RootNode {}

pub(crate) struct DeleteContext<'a, 'b: 'a, 'c: 'b, K, Neighbourhood, G: PageIdGenerator>
where
    K: FixedSize,
{
    current_id: PageId,
    backtrack: &'a DeleteBacktrack<'b, 'c, K, G>,
    neighbourhood: Neighbourhood,
}

impl<'a, 'b: 'a, 'c: 'b, K, Neighbourhood, G: PageIdGenerator>
    DeleteContext<'a, 'b, 'c, K, Neighbourhood, G>
where
    K: FixedSize,
{
    /// delete current node, this just adds the id to the list of free pages *after* the transaction is confirmed
    pub fn delete_node(&self) {
        self.backtrack.delete_node(self.current_id)
    }
}

impl<'a, 'b: 'a, 'c: 'b, K, G: PageIdGenerator> DeleteContext<'a, 'b, 'c, K, NonRoot<'a>, G>
where
    K: FixedSize,
{
    pub fn mut_left_sibling(&mut self) -> (PageRefMut<'a>, &mut PageRefMut<'a>) {
        let sibling = match self
            .backtrack
            .tx
            .mut_page(self.neighbourhood.left_id.unwrap())
            .unwrap()
        {
            MutablePage::InTransaction(handle) => handle,
            MutablePage::NeedsParentRedirect(redirect_pointers) => {
                redirect_pointers.redirect_parent_in_tx::<K>(&mut self.neighbourhood.parent)
            }
        };

        (sibling, &mut self.neighbourhood.parent)
    }

    pub fn mut_right_sibling(&mut self) -> (PageRefMut<'a>, &mut PageRefMut<'a>) {
        let sibling = match self
            .backtrack
            .tx
            .mut_page(self.neighbourhood.right_id.unwrap())
            .unwrap()
        {
            MutablePage::InTransaction(handle) => handle,
            MutablePage::NeedsParentRedirect(redirect_pointers) => {
                redirect_pointers.redirect_parent_in_tx::<K>(&mut self.neighbourhood.parent)
            }
        };

        (sibling, &mut self.neighbourhood.parent)
    }

    /// delete right sibling of current node, this just adds the id to the list of free pages *after* the transaction is confirmed
    pub fn delete_right_sibling(&self) -> Result<(), ()> {
        match self.neighbourhood.right_id {
            None => Err(()),
            Some(right_id) => {
                self.backtrack.delete_node(right_id);
                Ok(())
            }
        }
    }
}

impl<'a, 'b: 'a, K, G: PageIdGenerator> NextElement<'a, 'b, K, G>
where
    K: FixedSize,
{
    pub fn set_root(&self, id: PageId) {
        self.backtrack.tx.current_root.set(id)
    }
}

enum Step<'a, K> {
    Leaf(PageId),
    Internal(PageId, &'a InternalNode<'a, K, &'a [u8]>, Option<usize>),
}

fn search<F, K, G: PageIdGenerator>(key: &K, tx: &WriteTransaction<G>, mut f: F)
where
    F: FnMut(Step<K>),
    K: FixedSize,
{
    let mut current = tx.root();

    loop {
        let page = tx.get_page(current).unwrap();

        let found_leaf = page.as_node(|node: Node<K, &[u8]>| {
            if let Some(inode) = node.try_as_internal() {
                let upper_pivot = match inode.keys().binary_search(key) {
                    Ok(pos) => Some(pos + 1),
                    Err(pos) => Some(pos),
                }
                .filter(|pos| pos < &inode.children().len());

                f(Step::Internal(page.id(), &inode, upper_pivot));

                if let Some(upper_pivot) = upper_pivot {
                    current = inode.children().get(upper_pivot);
                } else {
                    let last = inode.children().len().checked_sub(1).unwrap();
                    current = inode.children().get(last);
                }
                false
            } else {
                f(Step::Leaf(page.id()));
                true
            }
        });

        if found_leaf {
            return;
        }
    }
}

impl<'txbuilder, 'txmanager: 'txbuilder, 'storage: 'txmanager, K, G: PageIdGenerator>
    DeleteBacktrack<'txbuilder, 'txmanager, K, G>
where
    K: FixedSize,
{
    pub(crate) fn new_search_for(
        tx: &'txbuilder mut WriteTransaction<'txmanager, G>,
        key: &K,
    ) -> Self {
        let mut backtrack = vec![];
        let mut parent_info = vec![];

        search(key, tx, |step| match step {
            Step::Leaf(page_id) => backtrack.push(page_id),
            Step::Internal(page_id, inode, upper_pivot) => {
                backtrack.push(page_id);
                let anchor = upper_pivot
                    .or_else(|| inode.keys().len().checked_sub(1))
                    .and_then(|up| up.checked_sub(1));

                let left_sibling_id = anchor.and_then(|pos| inode.children().try_get(pos));

                let right_sibling_id = anchor
                    .map(|pos| pos + 2)
                    .or(Some(1))
                    .and_then(|pos| inode.children().try_get(pos));

                parent_info.push((anchor, left_sibling_id, right_sibling_id));
            }
        });

        DeleteBacktrack {
            tx,
            backtrack,
            parent_info,
            new_root: None,
            phantom_key: PhantomData,
        }
    }

    pub fn get_next<'this>(
        &'this mut self,
    ) -> Result<Option<DeleteNextElement<'this, 'txbuilder, 'txmanager, K, G>>, std::io::Error>
    {
        let id = match self.backtrack.pop() {
            Some(id) => id,
            None => return Ok(None),
        };

        if self.backtrack.is_empty() {
            assert!(self.new_root.is_none());
            self.new_root = Some(id);
        }

        let parent_info = match self.backtrack.last() {
            Some(parent) => {
                // we need the parent id, which is the next node in the stack, but we should not pop, because it would be the next node to process
                let (anchor, left, right) = self.parent_info.pop().expect("missing parent info");
                Some((parent, anchor, left, right))
            }
            None => None,
        };

        let next = match self.tx.mut_page(id)? {
            transaction::MutablePage::NeedsParentRedirect(rename_in_parents) => {
                // recursively clone(if they are not already used for some operation in the same transaction)
                // and redirect the whole path to this node.
                // Here redirect means clone the nodes and point the parents to the clone of its child
                let mut rename_in_parents = Some(rename_in_parents);
                let mut finished = None;
                for id in self.backtrack.iter().rev() {
                    let result = rename_in_parents
                        .take()
                        .unwrap()
                        .redirect_parent_pointer::<K>(*id)?;

                    match result {
                        MutablePage::NeedsParentRedirect(rename) => {
                            rename_in_parents = Some(rename)
                        }
                        MutablePage::InTransaction(handle) => {
                            finished = Some(handle);
                            break;
                        }
                    }
                }
                match finished {
                    Some(handle) => handle,
                    // None means we got to the root of the tree
                    None => rename_in_parents.unwrap().finish(),
                }
            }
            transaction::MutablePage::InTransaction(handle) => handle,
        };

        let neighbourhood = parent_info
            .map(
                |(parent, _anchor, left_id, right_id)| -> Result<NonRoot, std::io::Error> {
                    let parent = match self.tx.mut_page(*parent)? {
                        MutablePage::InTransaction(handle) => handle,
                        _ => unreachable!(),
                    };
                    Ok(NonRoot {
                        parent,
                        left_id,
                        right_id,
                    })
                },
            )
            .transpose()?;

        let (left, right) = match parent_info {
            Some((_parent, _anchor, left, right)) => {
                let left = left.and_then(|id| self.tx.get_page(id));
                let right = right.and_then(|id| self.tx.get_page(id));

                (left, right)
            }
            None => (None, None),
        };

        let anchor = parent_info.and_then(|(_, anchor, _, _)| anchor);

        let mut_context = match neighbourhood {
            Some(neighbourhood) => MutableContext::NonRoot(DeleteContext {
                neighbourhood,
                current_id: id,
                backtrack: self,
            }),
            None => MutableContext::Root(DeleteContext {
                neighbourhood: RootNode {},
                current_id: id,
                backtrack: self,
            }),
        };
        let next_element = NextElement {
            next,
            anchor,
            left,
            right,
            backtrack: self,
        };

        Ok(Some(DeleteNextElement {
            next_element,
            mut_context,
        }))
    }

    pub fn delete_node(&self, page_id: PageId) {
        self.tx.delete_node(page_id)
    }
}

impl<'txbuilder, 'txmanager: 'txbuilder, 'index: 'txmanager, K, G: PageIdGenerator>
    InsertBacktrack<'txbuilder, 'txmanager, K, G>
where
    K: FixedSize,
{
    pub(crate) fn new_search_for(
        tx: &'txbuilder mut WriteTransaction<'txmanager, G>,
        key: &K,
    ) -> Self {
        let mut backtrack = vec![];
        search(key, tx, |step| match step {
            Step::Leaf(page_id) => backtrack.push(page_id),
            Step::Internal(page_id, _, _) => backtrack.push(page_id),
        });

        InsertBacktrack {
            tx,
            backtrack,
            new_root: None,
            phantom_key: PhantomData,
        }
    }

    pub fn get_next(&mut self) -> Result<Option<PageRefMut<'_>>, std::io::Error> {
        let id = match self.backtrack.pop() {
            Some(id) => id,
            None => return Ok(None),
        };

        if self.backtrack.is_empty() {
            assert!(self.new_root.is_none());
            self.new_root = Some(dbg!(id));
        }

        match self.tx.mut_page(dbg!(id))? {
            transaction::MutablePage::NeedsParentRedirect(rename_in_parents) => {
                // this part may be tricky, we need to recursively clone and redirect all the path
                // from the root to the node we are writing to. We need the backtrack stack, because
                // that's the only way to get the parent of a node (because there are no parent pointers)
                // so we iterate it in reverse but without consuming the stack (as we still need it for the
                // rest of the insertion algorithm)
                let mut rename_in_parents = rename_in_parents;
                for id in self.backtrack.iter().rev() {
                    let result = rename_in_parents.redirect_parent_pointer::<K>(*id)?;

                    match result {
                        MutablePage::NeedsParentRedirect(rename) => rename_in_parents = rename,
                        MutablePage::InTransaction(handle) => return Ok(Some(handle)),
                    }
                }
                Ok(Some(rename_in_parents.finish()))
            }
            transaction::MutablePage::InTransaction(handle) => Ok(Some(handle)),
        }
    }

    pub fn has_next(&self) -> bool {
        self.backtrack.last().is_some()
    }

    pub fn add_new_node(&mut self, mem_page: MemPage) -> Result<PageId, std::io::Error> {
        self.tx.add_new_node(mem_page)
    }

    pub fn new_root(&mut self, mem_page: MemPage) -> Result<(), std::io::Error> {
        let id = self.tx.add_new_node(mem_page)?;
        self.tx.current_root.set(id);

        Ok(())
    }
}

impl<'txbuilder, 'txmanager: 'txbuilder, K, G: PageIdGenerator>
    UpdateBacktrack<'txbuilder, 'txmanager, K, G>
where
    K: FixedSize,
{
    pub(crate) fn new_search_for(
        tx: &'txbuilder mut WriteTransaction<'txmanager, G>,
        key: &K,
    ) -> Self {
        let mut backtrack = vec![];
        search(key, tx, |step| match step {
            Step::Leaf(page_id) => backtrack.push(page_id),
            Step::Internal(page_id, _, _) => backtrack.push(page_id),
        });

        UpdateBacktrack {
            tx,
            backtrack,
            key_to_update: key.clone(),
            new_root: None,
            phantom_key: PhantomData,
        }
    }

    pub fn update<V: FixedSize>(&mut self, new_value: V) -> Result<(), std::io::Error> {
        let leaf = match self.backtrack.pop() {
            Some(id) => id,
            None => return Ok(()),
        };

        let position_to_update =
            match self
                .tx
                .get_page(leaf)
                .unwrap()
                .as_node(|node: Node<K, &[u8]>| {
                    node.as_leaf::<V>()
                        .keys()
                        .binary_search(&self.key_to_update)
                }) {
                Ok(pos) => pos,
                Err(_) => return Ok(()),
            };

        let mut page_handle = match self.tx.mut_page(leaf)? {
            transaction::MutablePage::NeedsParentRedirect(rename_in_parents) => {
                let mut rename_in_parents = Some(rename_in_parents);
                let handle = loop {
                    let id = match self.backtrack.pop() {
                        Some(id) => id,
                        None => {
                            break None;
                        }
                    };

                    if self.backtrack.is_empty() {
                        self.new_root = Some(id);
                    }

                    let result = rename_in_parents
                        .take()
                        .unwrap()
                        .redirect_parent_pointer::<K>(id)?;

                    match result {
                        MutablePage::NeedsParentRedirect(rename) => {
                            rename_in_parents = Some(rename)
                        }
                        MutablePage::InTransaction(handle) => break Some(handle),
                    };
                };

                handle.unwrap_or_else(|| rename_in_parents.take().unwrap().finish())
            }
            transaction::MutablePage::InTransaction(handle) => handle,
        };

        page_handle.as_node_mut(|mut node: Node<K, &mut [u8]>| {
            node.as_leaf_mut()
                .values_mut()
                .update(position_to_update, &new_value)
                .expect("position to update was not in range")
        });

        Ok(())
    }
}
