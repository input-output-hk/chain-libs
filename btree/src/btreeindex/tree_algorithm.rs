use super::node::internal_node::InternalDeleteStatus;
use super::node::leaf_node::LeafDeleteStatus;
use super::node::{
    InternalInsertStatus, LeafInsertStatus, Node, NodeRef, NodeRefMut, RebalanceResult, SiblingsArg,
};
use super::pages::Pages;
use super::version_management::transaction::{
    PageRef, PageRefMut, ReadTransaction, WriteTransaction,
};
use super::version_management::TreeIdentifier;
use crate::mem_page::MemPage;
use crate::BTreeStoreError;
use std::borrow::Borrow;

use crate::FixedSize;

use super::backtrack::{DeleteBacktrack, InsertBacktrack};
use super::page_manager::PageIdGenerator;
use super::PageId;

type SplitKeyNodePair<K> = (K, Node<K, MemPage>);

pub(crate) fn insert<K: FixedSize, V: FixedSize, G: PageIdGenerator>(
    tx: &mut WriteTransaction<'_, G>,
    key: K,
    value: V,
    page_size: usize,
) -> Result<(), BTreeStoreError> {
    let mut backtrack = InsertBacktrack::new_search_for(tx, &key);

    let needs_recurse = {
        let leaf = backtrack.get_next()?.unwrap();
        let leaf_id = leaf.id();
        insert_in_leaf(leaf, key, value, page_size)?
            .map(|(split_key, new_node)| (leaf_id, split_key, new_node))
    };

    if let Some((leaf_id, split_key, new_node)) = needs_recurse {
        let id = backtrack.add_new_node(new_node.into_page())?;

        if backtrack.has_next() {
            insert_in_internals(split_key, id, &mut backtrack, page_size)?;
        } else {
            let new_root = create_internal_node(leaf_id, id, split_key, page_size);
            backtrack.new_root(new_root.into_page())?;
        }
    }

    Ok(())
}

fn insert_in_leaf<K: FixedSize, V: FixedSize>(
    mut leaf: PageRefMut,
    key: K,
    value: V,
    page_size: usize,
) -> Result<Option<SplitKeyNodePair<K>>, BTreeStoreError> {
    let update = {
        let mut allocate = || {
            let uninit = MemPage::new(page_size);
            Node::<K, MemPage>::new_leaf::<V>(uninit)
        };

        let insert_status = leaf.as_node_mut(move |mut node: Node<K, &mut [u8]>| {
            node.as_leaf_mut().insert(key, value, &mut allocate)
        });

        match insert_status {
            LeafInsertStatus::Ok => None,
            LeafInsertStatus::DuplicatedKey(_k) => {
                return Err(crate::BTreeStoreError::DuplicatedKey)
            }
            LeafInsertStatus::Split(split_key, node) => Some((split_key, node)),
        }
    };

    Ok(update)
}

// this function recurses on the backtrack splitting internal nodes as needed
fn insert_in_internals<'a, K: FixedSize, G: PageIdGenerator>(
    key: K,
    to_insert: PageId,
    backtrack: &'a mut InsertBacktrack<K, G>,
    page_size: usize,
) -> Result<(), BTreeStoreError> {
    let mut split_key = key;
    let mut right_id = to_insert;
    loop {
        let (current_id, new_split_key, new_node) = {
            let mut node = backtrack.get_next()?.unwrap();
            let node_id = node.id();
            let mut allocate = || {
                let uninit = MemPage::new(page_size);
                Node::new_internal(uninit)
            };

            match node.as_node_mut(|mut node| {
                node.as_internal_mut()
                    .insert(split_key, right_id, &mut allocate)
            }) {
                InternalInsertStatus::Ok => return Ok(()),
                InternalInsertStatus::Split(split_key, new_node) => (node_id, split_key, new_node),
                _ => unreachable!(),
            }
        };

        let new_id = backtrack.add_new_node(new_node.into_page())?;

        if backtrack.has_next() {
            // set values to insert in next iteration (recurse on parent)
            split_key = new_split_key;
            right_id = new_id;
        } else {
            let left_id = current_id;
            let right_id = new_id;
            let new_root = create_internal_node(left_id, right_id, new_split_key, page_size);

            backtrack.new_root(new_root.into_page())?;
            return Ok(());
        }
    }
}

// Used when the current root needs a split
fn create_internal_node<K: FixedSize>(
    left_child: PageId,
    right_child: PageId,
    key: K,
    page_size: usize,
) -> Node<K, MemPage> {
    let page = MemPage::new(page_size);
    let mut node = Node::new_internal(page);

    node.as_internal_mut()
        .insert_first(key, left_child, right_child);

    node
}

pub(crate) fn search<'a, T, K, Q, P>(tx: &'a ReadTransaction<T, P>, key: &Q) -> PageRef<'a>
where
    Q: Ord,
    K: FixedSize + Borrow<Q>,
    P: Borrow<Pages>,
    T: TreeIdentifier,
{
    let mut current = tx.get_page(tx.root()).unwrap();

    loop {
        let new_current = current.as_node(|node: Node<K, &[u8]>| {
            node.try_as_internal().map(|inode| {
                let upper_pivot = match inode.keys().binary_search(key) {
                    Ok(pos) => Some(pos + 1),
                    Err(pos) => Some(pos),
                }
                .filter(|pos| pos < &inode.children().len());

                let new_current_id = if let Some(upper_pivot) = upper_pivot {
                    inode.children().get(upper_pivot)
                } else {
                    let last = inode.children().len().checked_sub(1).unwrap();
                    inode.children().get(last)
                };

                tx.get_page(new_current_id).unwrap()
            })
        });

        if let Some(new_current) = new_current {
            current = new_current;
        } else {
            // found leaf
            break;
        }
    }

    current
}

pub(crate) fn delete<K: FixedSize, V: FixedSize, G: PageIdGenerator>(
    key: &K,
    tx: &mut WriteTransaction<G>,
) -> Result<(), BTreeStoreError> {
    let mut backtrack = DeleteBacktrack::new_search_for(tx, key);

    // we can unwrap safely because there is always a leaf in the path
    // delete will return Ok if the key is not in the given leaf
    use super::backtrack::DeleteNextElement;
    let DeleteNextElement {
        mut next_element,
        mut_context,
    } = backtrack.get_next()?.unwrap();

    let delete_result = next_element
        .next
        .as_node_mut(|mut node| node.as_leaf_mut::<V>().delete(key))?;

    match delete_result {
        LeafDeleteStatus::Ok => return Ok(()),
        LeafDeleteStatus::NeedsRebalance => (),
    };

    // this allows us to get mutable references to out parent and siblings, we only need those when we need to rebalance
    let mut mut_context = match mut_context {
        super::backtrack::MutableContext::NonRoot(mut_context) => mut_context,
        // this means we are processing the root node, it is not possible to do any rebalancing because we don't have siblings
        // I think we don't need to do anything here, in theory, we could change the tree height to 0, but we are not tracking the height
        super::backtrack::MutableContext::Root(_) => return Ok(()),
    };

    let next = &mut next_element.next;
    let left = next_element.left.as_ref();
    let right = next_element.right.as_ref();
    // we need this to know which child we are (what position does this node have in the parent)
    let anchor = next_element.anchor;

    let should_recurse_on_parent: Option<usize> = next.as_node_mut(
        |mut node: Node<K, &mut [u8]>| -> Result<Option<usize>, BTreeStoreError> {
            let siblings = SiblingsArg::new_from_options(left, right);

            match node.as_leaf_mut::<V>().rebalance(siblings)? {
                RebalanceResult::TakeFromLeft(add_sibling) => {
                    let (sibling, parent) = mut_context.mut_left_sibling();
                    add_sibling.take_key_from_left(parent, anchor, sibling);
                    Ok(None)
                }
                RebalanceResult::TakeFromRight(add_sibling) => {
                    let (sibling, parent) = mut_context.mut_right_sibling();
                    add_sibling.take_key_from_right(parent, anchor, sibling);
                    Ok(None)
                }
                RebalanceResult::MergeIntoLeft(add_sibling) => {
                    let (sibling, _) = mut_context.mut_left_sibling();
                    add_sibling.merge_into_left(sibling);
                    mut_context.delete_node();
                    // the anchor is the the index of the key that splits the left sibling and the node, it's only None if the current node
                    // it's the leftmost (and thus has no left sibling)
                    Ok(Some(
                        anchor.expect("merged into left sibling, but anchor is None"),
                    ))
                }
                RebalanceResult::MergeIntoSelf(add_sibling) => {
                    let (sibling, _) = mut_context.mut_right_sibling();
                    add_sibling.merge_into_self(sibling);
                    mut_context
                        .delete_right_sibling()
                        .expect("can't mutate right sibling");
                    Ok(Some(anchor.map_or(0, |a| a + 1)))
                }
            }
        },
    )?;

    // we need to do this because `mut_context` has a mutable borrow of the parent, which is the next node to process
    // I don't think adding an additional scope and indentation level is worth it in that case. Geting rid of the closure above may be a better solution
    drop(mut_context);

    if let Some(anchor) = should_recurse_on_parent {
        delete_internal(anchor, &mut backtrack)?;
    }

    Ok(())
}

fn delete_internal<'a, 'b: 'a, K: FixedSize, G: PageIdGenerator>(
    anchor: usize,
    tx: &'a mut DeleteBacktrack<'b, 'b, K, G>,
) -> Result<(), BTreeStoreError> {
    let mut anchor_to_delete = anchor;
    while let Some(next_element) = tx.get_next()? {
        let super::backtrack::DeleteNextElement {
            mut next_element,
            mut_context,
        } = next_element;

        let last_value = match next_element
            .next
            .as_node_mut(|mut node: Node<K, &mut [u8]>| {
                let mut node = node.as_internal_mut();
                node.delete_key_children(anchor_to_delete)
            }) {
            InternalDeleteStatus::Ok => return Ok(()),
            InternalDeleteStatus::NeedsRebalance => None,
            InternalDeleteStatus::LastValue(id) => Some(id),
        };

        match mut_context {
            super::backtrack::MutableContext::Root(context) => {
                // here we are dealing with the root
                // the root is not rebalanced, but if it is empty then it can
                // be deleted, and unlike the leaf case, we need to promote it's only remainining child as the new root
                if let Some(new_root) = last_value {
                    next_element.set_root(new_root);
                }

                context.delete_node()
            }
            super::backtrack::MutableContext::NonRoot(mut mut_context) => {
                // non-root node
                // let parent = next_element.parent.unwrap();
                let anchor = next_element.anchor;
                let left = next_element.left;
                let right = next_element.right;

                // as in the leaf case, the value in the Option is the 'anchor' (pointer) to the deleted node
                let recurse_on_parent: Option<usize> = next_element.next.as_node_mut(
                    |mut node: Node<K, &mut [u8]>| -> Result<Option<usize>, BTreeStoreError> {
                        let siblings = SiblingsArg::new_from_options(left, right);

                        match node.as_internal_mut().rebalance(siblings)? {
                            RebalanceResult::TakeFromLeft(add_params) => {
                                let (sibling, parent) = mut_context.mut_left_sibling();
                                add_params.take_key_from_left(
                                    parent,
                                    anchor.expect("left sibling seems to exist but anchor is none"),
                                    sibling,
                                );
                                Ok(None)
                            }
                            RebalanceResult::TakeFromRight(add_params) => {
                                let (sibling, parent) = mut_context.mut_right_sibling();
                                add_params.take_key_from_right(parent, anchor, sibling);
                                Ok(None)
                            }
                            RebalanceResult::MergeIntoLeft(add_params) => {
                                let (sibling, parent) = mut_context.mut_left_sibling();
                                add_params.merge_into_left(parent, anchor, sibling)?;
                                mut_context.delete_node();
                                Ok(Some(
                                    anchor
                                        .clone()
                                        .expect("merged into left sibling, but anchor is none"),
                                ))
                            }
                            RebalanceResult::MergeIntoSelf(add_params) => {
                                let (sibling, parent) = mut_context.mut_right_sibling();
                                add_params.merge_into_self(parent, anchor, sibling)?;
                                let new_anchor = anchor.map_or(0, |n| n + 1);
                                mut_context
                                    .delete_right_sibling()
                                    .expect("right sibling doesn't exist");
                                Ok(Some(new_anchor))
                            }
                        }
                    },
                )?;

                // (there is no recursive call, we just go the next loop iteration)
                if let Some(anchor) = recurse_on_parent {
                    anchor_to_delete = anchor;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}
