use super::hash::{Hash, HashedKey, Hasher};
use super::node::{
    insert_rec, lookup_one, remove_eq_rec, remove_rec, replace_rec, replace_with_rec, size_rec,
    update_rec, Entry, LookupRet, Node, NodeIter,
};
pub use super::operation::{InsertError, RemoveError, ReplaceError, UpdateError};
use std::borrow::Borrow;
use std::error::Error;
use std::fmt::Debug;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::mem::swap;
use std::slice;

#[derive(Debug, Clone)]
pub struct Hamt<H: Hasher + Default, K: PartialEq + Eq + Hash, V> {
    root: Node<K, V>,
    hasher: PhantomData<H>,
}

pub struct HamtIter<'a, K, V> {
    stack: Vec<NodeIter<'a, K, V>>,
    content: Option<slice::Iter<'a, (K, V)>>,
}

impl<H: Hasher + Default, K: Eq + Hash, V> Default for Hamt<H, K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: Hasher + Default, K: Eq + Hash, V> Hamt<H, K, V> {
    pub fn new() -> Self {
        Hamt {
            root: Node::new(),
            hasher: PhantomData,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    pub fn size(&self) -> usize {
        size_rec(&self.root)
    }
}

impl<H: Hasher + Default, K: Clone + Eq + Hash, V: Clone> Hamt<H, K, V> {
    pub fn insert(&self, k: K, v: V) -> Result<Self, InsertError> {
        let h = HashedKey::compute(self.hasher, &k);
        let newroot = insert_rec(&self.root, h, 0, k, v)?;
        Ok(Hamt {
            root: newroot,
            hasher: PhantomData,
        })
    }
}

impl<H: Hasher + Default, K: Eq + Hash + Clone, V: PartialEq + Clone> Hamt<H, K, V> {
    pub fn remove_match<Q>(&self, k: &Q, v: &V) -> Result<Self, RemoveError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let h = HashedKey::compute(self.hasher, &k);
        let newroot = remove_eq_rec(&self.root, h, 0, k, v)?;
        match newroot {
            None => Ok(Self::new()),
            Some(r) => Ok(Hamt {
                root: r,
                hasher: PhantomData,
            }),
        }
    }
}

impl<H: Hasher + Default, K: Clone + Eq + Hash, V: Clone> Hamt<H, K, V> {
    pub fn remove<Q>(&self, k: &Q) -> Result<Self, RemoveError>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let h = HashedKey::compute(self.hasher, k);
        let newroot = remove_rec(&self.root, h, 0, k)?;
        match newroot {
            None => Ok(Self::new()),
            Some(r) => Ok(Hamt {
                root: r,
                hasher: PhantomData,
            }),
        }
    }
}

impl<H: Hasher + Default, K: Eq + Hash + Clone, V: Clone> Hamt<H, K, V> {
    /// Replace the element at the key by the v and return the new tree
    /// and the old value.
    pub fn replace(&self, k: &K, v: V) -> Result<(Self, V), ReplaceError> {
        let h = HashedKey::compute(self.hasher, &k);
        let (newroot, oldv) = replace_rec(&self.root, h, 0, k, v)?;
        Ok((
            Hamt {
                root: newroot,
                hasher: PhantomData,
            },
            oldv,
        ))
    }

    /// Replace the element at the key by the v and return the new tree
    /// and the old value.
    pub fn replace_with<F>(&self, k: &K, f: F) -> Result<Self, ReplaceError>
    where
        F: FnOnce(&V) -> V,
    {
        let h = HashedKey::compute(self.hasher, &k);
        let newroot = replace_with_rec(&self.root, h, 0, k, f)?;
        Ok(Hamt {
            root: newroot,
            hasher: PhantomData,
        })
    }
}

impl<H: Hasher + Default, K: Eq + Hash + Clone, V: Clone> Hamt<H, K, V> {
    /// Update the element at the key K.
    ///
    /// If the closure F in parameter returns None, then the key is deleted.
    ///
    /// If the key is not present then UpdateError::KeyNotFound is returned
    pub fn update<F, U>(&self, k: &K, f: F) -> Result<Self, UpdateError<U>>
    where
        F: FnOnce(&V) -> Result<Option<V>, U>,
        U: Error + Debug + 'static,
    {
        let h = HashedKey::compute(self.hasher, &k);
        let newroot = update_rec(&self.root, h, 0, k, f)?;
        match newroot {
            None => Ok(Self::new()),
            Some(r) => Ok(Hamt {
                root: r,
                hasher: PhantomData,
            }),
        }
    }

    /// Update or insert the element at the key K
    ///
    /// If the element is not present, then V is added, otherwise the closure F is apply
    /// to the found element. If the closure returns None, then the key is deleted
    pub fn insert_or_update<F, E>(&self, k: K, v: V, f: F) -> Result<Self, E>
    where
        F: FnOnce(&V) -> Result<Option<V>, E>,
        V: Clone,
        E: Error + Debug + 'static,
    {
        match self.update(&k, f) {
            Ok(new_self) => Ok(new_self),
            Err(UpdateError::KeyNotFound) =>
            // unwrap is safe: only error than can be raised is an EntryExist which is fundamentally impossible in this error case handling
            {
                Ok(self.insert(k, v).unwrap())
            }
            Err(UpdateError::ValueCallbackError(x)) => Err(x),
        }
    }

    /// Update or insert the element at the key K
    ///
    /// If the element is not present, then V is added, otherwise the closure F is apply
    /// to the found element. If the closure returns None, then the key is deleted.
    ///
    /// This is similar to 'insert_or_update' except the closure shouldn't be failing
    pub fn insert_or_update_simple<F>(&self, k: K, v: V, f: F) -> Self
    where
        F: for<'a> FnOnce(&'a V) -> Option<V>,
        V: Clone,
    {
        use std::convert::Infallible;
        match self.update(&k, |x| Ok::<_, Infallible>(f(x))) {
            Ok(new_self) => new_self,
            Err(UpdateError::ValueCallbackError(_)) => unreachable!(), // callback always wrapped in Ok
            Err(UpdateError::KeyNotFound) => {
                // unwrap is safe: only error than can be raised is an EntryExist which is fundamentally impossible in this error case handling
                self.insert(k, v).unwrap()
            }
        }
    }
}

impl<H: Hasher + Default, K: Hash + Eq, V> Hamt<H, K, V> {
    /// Try to get the element related to key K
    pub fn lookup<Q>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let h = HashedKey::compute(self.hasher, k);
        let mut n = &self.root;
        let mut lvl = 0;
        loop {
            match lookup_one(n, &h, lvl, k) {
                LookupRet::NotFound => return None,
                LookupRet::Found(v) => return Some(v),
                LookupRet::ContinueIn(subnode) => {
                    lvl += 1;
                    n = subnode;
                }
            }
        }
    }

    /// Check if the key is contained into the HAMT
    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.lookup(k).is_some()
    }

    pub fn iter(&self) -> HamtIter<K, V> {
        HamtIter {
            stack: vec![self.root.iter()],
            content: None,
        }
    }
}

impl<'a, K, V> Iterator for HamtIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut x = None;
            swap(&mut self.content, &mut x);
            match x {
                Some(mut iter) => match iter.next() {
                    None => self.content = None,
                    Some(o) => {
                        self.content = Some(iter);
                        return Some((&o.0, &o.1));
                    }
                },
                None => match self.stack.last_mut() {
                    None => return None,
                    Some(last) => match last.next() {
                        None => {
                            self.stack.pop();
                        }
                        Some(next) => match next.as_ref() {
                            Entry::SubNode(ref sub) => self.stack.push(sub.iter()),
                            Entry::Leaf(_, k, v) => return Some((k, v)),
                            Entry::LeafMany(_, ref col) => self.content = Some(col.iter()),
                        },
                    },
                },
            }
        }
    }
}

impl<H: Default + Hasher, K: Eq + Hash + Clone, V: Clone> FromIterator<(K, V)> for Hamt<H, K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut h = Hamt::new();
        for (k, v) in iter {
            match h.insert(k, v) {
                Err(_) => {}
                Ok(newh) => h = newh,
            }
        }
        h
    }
}

impl<H: Default + Hasher, K: Eq + Hash, V: PartialEq> PartialEq for Hamt<H, K, V> {
    fn eq(&self, other: &Self) -> bool {
        // optimised the obvious cases first
        if self.is_empty() && other.is_empty() {
            return true;
        }
        if self.is_empty() != other.is_empty() {
            return false;
        }
        // then compare key and values
        // TODO : optimise by comparing nodes directly
        for (k, v) in self.iter() {
            if let Some(v2) = other.lookup(k) {
                if v != v2 {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

impl<H: Default + Hasher, K: Eq + Hash, V: Eq> Eq for Hamt<H, K, V> {}

impl<'a, H: Default + Hasher, K: Eq + Hash, V> IntoIterator for &'a Hamt<H, K, V> {
    type Item = (&'a K, &'a V);

    type IntoIter = HamtIter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
