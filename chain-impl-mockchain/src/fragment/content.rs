use crate::fragment::Fragment;
use crate::key::Hash;
use chain_core::{packer::Codec, property::Serialize};
use std::slice;

pub type BlockContentHash = Hash;
pub type BlockContentSize = u32;

/// Block Contents
///
/// To create this structure, make a ContentsBuilder and use into()
#[derive(Debug, Clone)]
pub struct Contents(pub(super) Box<[Fragment]>);

impl PartialEq for Contents {
    fn eq(&self, rhs: &Self) -> bool {
        self.compute_hash_size() == rhs.compute_hash_size()
    }
}
impl Eq for Contents {}

impl From<ContentsBuilder> for Contents {
    fn from(content_builder: ContentsBuilder) -> Self {
        Contents(content_builder.fragments.into())
    }
}

impl Contents {
    pub fn empty() -> Self {
        Contents(Vec::with_capacity(0).into())
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &'_ Fragment> {
        self.0.iter()
    }

    #[inline]
    pub fn iter_slice(&self) -> slice::Iter<'_, Fragment> {
        self.0.iter()
    }

    pub fn compute_hash_size(&self) -> (BlockContentHash, BlockContentSize) {
        let mut bytes = Vec::with_capacity(4096);

        for message in self.iter() {
            message.serialize(&mut Codec::new(&mut bytes)).unwrap();
        }

        let hash = Hash::hash_bytes(&bytes);
        (hash, bytes.len() as u32)
    }

    pub fn compute_hash(&self) -> BlockContentHash {
        self.compute_hash_size().0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Default)]
pub struct ContentsBuilder {
    fragments: Vec<Fragment>,
}

impl ContentsBuilder {
    pub fn new() -> Self {
        ContentsBuilder {
            fragments: Vec::new(),
        }
    }

    pub fn push(&mut self, fragment: Fragment) {
        self.fragments.push(fragment)
    }

    /// set multiple messages in the block to build
    pub fn push_many<I>(&mut self, fragments: I) -> &mut Self
    where
        I: IntoIterator<Item = Fragment>,
    {
        self.fragments.extend(fragments);
        self
    }
}
