//! Mostly Immutable / Shareable Time sequence of T
//!
//! This is a collection of T, design around handling ~million of sequential Ts.
//!
//! The collection is wrapped around the following design decisions:
//! * for older parts of the sequence to be efficiently
//!   shared and efficiently be written/be readable from colder storage in chunk
//! * for older parts to be droppable when this is old enough that we don't need it
//! * to append elements to this collection cheaply
//!
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::Arc;

// This lead of array of contiguous 512kb of size for 32 bytes content
const CHUNK_CAPACITY: usize = 16384;

/// An immutable chunk of a sequence
pub struct Chunk<T> {
    start_depth: u64,
    data: Box<[T]>,
}

impl<T> Chunk<T> {
    pub fn from_iterator<I>(&self, start_depth: u64, iter: I) -> Self
    where
        I: Iterator<Item = T>,
    {
        Chunk {
            start_depth,
            data: iter.collect::<Vec<_>>().into(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the depth of the element above the last in this chunk
    ///
    /// If this chunk is empty, then it returns the start_depth.
    pub fn end_depth(&self) -> u64 {
        self.start_depth + self.data.len() as u64
    }

    pub fn iter<'a>(&'a self) -> ChunkIter<'a, T> {
        ChunkIter {
            depth: self.start_depth,
            iter: self.data.iter(),
        }
    }

    pub fn get(&self, depth: u64) -> Option<&T> {
        if depth >= self.start_depth {
            let m = (depth - self.start_depth) as usize;
            if m < self.data.len() {
                Some(&self.data[m])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn find(&self, element: &T) -> Option<u64>
    where
        T: Eq,
    {
        for (idx, e) in self.data.iter().enumerate() {
            if e == element {
                return Some(self.start_depth + idx as u64);
            }
        }
        None
    }
}

pub struct ChunkIter<'a, T> {
    depth: u64,
    iter: std::slice::Iter<'a, T>,
}

impl<'a, T> Iterator for ChunkIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().and_then(|x| {
            self.depth += 1;
            Some(x)
        })
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let left = self.iter.len();
        if n > left {
            panic!("nth on ChunkIter that contains less item")
        } else {
            self.depth += n as u64;
            self.iter.nth(n)
        }
    }
}

impl<'a, T> ExactSizeIterator for ChunkIter<'a, T> {
    fn len(&self) -> usize {
        self.iter.len()
    }
}

/// A Mutable chunk of a sequence
#[derive(Clone)]
struct ChunkMut<T> {
    start_depth: u64,
    data: Vec<T>,
}

impl<T> ChunkMut<T> {
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn end_depth(&self) -> u64 {
        self.start_depth + self.data.len() as u64
    }

    pub fn new(start_depth: u64) -> Self {
        ChunkMut {
            start_depth,
            data: Vec::with_capacity(CHUNK_CAPACITY),
        }
    }

    pub fn append(&mut self, elem: T) {
        self.data.push(elem)
    }

    pub fn find(&self, element: &T) -> Option<u64>
    where
        T: Eq,
    {
        for (idx, e) in self.data.iter().enumerate() {
            if e == element {
                return Some(self.start_depth + idx as u64);
            }
        }
        None
    }
}

impl<T> From<ChunkMut<T>> for Chunk<T> {
    fn from(cm: ChunkMut<T>) -> Self {
        Chunk {
            start_depth: cm.start_depth,
            data: cm.data.into(),
        }
    }
}

#[derive(Clone)]
pub struct Sequence<T> {
    spine: VecDeque<Arc<Chunk<T>>>,
    current: ChunkMut<T>,
}

impl<T> Sequence<T> {
    /// Create a new Sequence of T
    pub fn new(start_depth: u64) -> Self {
        Self {
            spine: VecDeque::with_capacity(0),
            current: ChunkMut::new(start_depth),
        }
    }

    /// Get the length of the spine
    ///
    /// note we use u64 explicitely as usize might overflow on 32 bits platform
    fn spine_len(&self) -> u64 {
        match self.spine.front() {
            None => 0,
            Some(o) => self.current.start_depth - o.start_depth,
        }
    }

    /// Get the earlier depth recorded
    pub fn start_depth(&self) -> u64 {
        self.spine
            .get(0)
            .map(|c| c.start_depth)
            .unwrap_or(self.current.start_depth)
    }

    /// Get the end depth (which is not in this collection)
    ///
    /// The last element of the collection is at depth `self.end_depth() - 1`
    pub fn end_depth(&self) -> u64 {
        self.current.end_depth()
    }

    /// Get the number of T in this sequence
    pub fn len(&self) -> u64 {
        self.spine_len() + self.current.len() as u64
    }

    /// Drop the oldest chunk of T
    pub fn drop_oldest(&mut self) -> Option<Arc<Chunk<T>>> {
        self.spine.pop_front()
    }

    /// Append an element into the chunk
    pub fn append(&mut self, element: T) {
        if self.current.len() >= CHUNK_CAPACITY {
            self.advance_freeze();
        }
        self.current.append(element)
    }

    /// Return the depth range of this sequence
    pub fn range(&self) -> Range<u64> {
        Range {
            start: self.start_depth(),
            end: self.end_depth(),
        }
    }

    /// Check whether the range of depth provided if in range
    pub fn in_range(&self, range: Range<u64>) -> bool {
        range.start >= self.start_depth() && range.end <= self.end_depth()
    }

    /// Return the common depth range between 2 sequences
    ///
    /// If none, then there's no depth range in common
    /// If Some, the range of valid common data is returned;
    /// it's guaranteed that the data between [start_depth..end_depth) exists.
    pub fn common_range(&self, other: &Self) -> Option<Range<u64>> {
        let b1 = std::cmp::max(self.start_depth(), other.start_depth());
        let b2 = std::cmp::min(self.end_depth(), other.end_depth());

        let common_range = Range { start: b1, end: b2 };
        if b1 < b2 && self.in_range(common_range.clone()) && other.in_range(common_range.clone()) {
            Some(common_range)
        } else {
            None
        }
    }

    /// Return the depth prefix between left and right sequences.
    ///
    /// if this returns Err, then the two sequences do not overlap in
    /// any way.
    ///
    /// If this returns Ok, then the two sequences do overlap,
    /// and then on Some it means the highest common prefix has been found
    pub fn highest_prefix(&self, other: &Self) -> Result<Option<u64>, ()>
    where
        T: Eq,
    {
        match self.common_range(other) {
            None => Err(()),
            Some(range) => {
                // no common prefix for sure
                if self.get(range.start) != other.get(range.start) {
                    return Ok(None);
                }

                // check the end match already
                if self.get(range.end - 1) == other.get(range.end - 1) {
                    return Ok(Some(range.end - 1));
                }

                // otherwise binary search of the highest depth that is equal
                let mut base = range.start;
                let mut size = range.end - range.start;

                while size > 1 {
                    // mid: [base..size)
                    let half = size / 2;
                    let mid = base + half;
                    // if equal we move the base to analyse the right side of the partition
                    if self.get(mid) == other.get(mid) {
                        base = mid
                    } else {
                    }
                    size -= half;
                }

                if self.get(base) == other.get(base) {
                    Ok(Some(base))
                } else {
                    Ok(Some(base - 1))
                }
            }
        }
    }

    /// Find an element
    pub fn get_index(&self, element: &T) -> Option<u64>
    where
        T: Eq,
    {
        self.current.find(element).or_else(|| {
            for c in self.spine.iter() {
                if let Some(idx) = c.find(element) {
                    return Some(idx);
                }
            }
            None
        })
    }

    /// Advance the frozen chunk by one, and push a new empty mutable chunk
    pub fn advance_freeze(&mut self) {
        let mut chunk = ChunkMut::new(self.current.end_depth());
        std::mem::swap(&mut self.current, &mut chunk);
        self.spine.push_back(Arc::new(chunk.into()));
    }

    /// Add chunk at the beginning of the sequence
    ///
    ///
    pub fn prepend_chunk(&mut self, chunk: Chunk<T>) {
        assert_eq!(chunk.end_depth(), self.start_depth());
        self.spine.push_front(Arc::new(chunk))
    }

    /// Get the element by depth if it exists
    pub fn get(&self, depth: u64) -> Option<&T> {
        if depth >= self.end_depth() {
            return None;
        }
        if depth < self.start_depth() {
            return None;
        }
        if depth < self.current.start_depth {
            match self.spine.get(0) {
                None => return None,
                Some(f) => {
                    if depth < f.start_depth {
                        return None;
                    }
                }
            };
            for c in self.spine.iter() {
                if depth >= c.end_depth() {
                    continue;
                }
                let idx = (depth - c.start_depth) as usize;
                return Some(&c.data[idx]);
            }
            // we should never reach this: unreachable!()
            None
        } else {
            let idx = (depth - self.current.start_depth) as usize;
            Some(&self.current.data[idx])
        }
    }

    /// Create a new iterator over the whole sequence of elements
    pub fn into_iter<'a>(&'a self) -> SequenceIterator<'a, T> {
        SequenceIterator {
            depth: self.start_depth(),
            seq: self,
        }
    }

    /// Create a new iterator that starts at a specific depth
    ///
    /// if the depth specified doesn't exists, this will panic
    pub fn into_iter_from<'a>(&'a self, depth: u64) -> SequenceIterator<'a, T> {
        assert!(self.start_depth() <= depth && depth < self.end_depth());
        SequenceIterator { depth, seq: self }
    }
}

/// Iterator over the sequence
pub struct SequenceIterator<'a, T> {
    /// Depth of the next element
    depth: u64,
    /// Reference to the whole sequence
    seq: &'a Sequence<T>,
}

impl<'a, T> Iterator for SequenceIterator<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.seq.get(self.depth).and_then(|x| {
            self.depth += 1;
            Some(x)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn it_works() {
        {
            let mut seq = Sequence::new(1000);
            for i in 0..100000u32 {
                seq.append(i)
            }

            assert_eq!(seq.get(0), None);
            assert_eq!(seq.get(999), None);
            for i in 0..100000u32 {
                assert_eq!(seq.get(1000 + i as u64), Some(&i));
            }

            assert_eq!(seq.in_range(1000..100000), true);
            assert_eq!(seq.in_range(1001..1003), true);
            assert_eq!(seq.in_range(1001..10000), true);
            assert_eq!(seq.in_range(999..10000), false);
            assert_eq!(seq.in_range(999..1000), false);
            assert_eq!(seq.in_range(100000..101001), false);
            assert_eq!(seq.in_range(100000..101000), true);
            assert_eq!(seq.in_range(101000..101001), false);
        }

        {
            // create a new sequence from 1000 to 2000
            let mut seq1 = Sequence::new(1000);
            let mut seq2 = Sequence::new(1100);

            for i in 1000u32..1200 {
                seq1.append(i * 10)
            }
            for i in 1100u32..1200 {
                if i > 1150 {
                    seq2.append(i * 100)
                } else {
                    seq2.append(i * 10)
                }
            }
            assert_eq!(seq1.common_range(&seq2), Some(1100..1200));
            assert_eq!(seq1.highest_prefix(&seq2), Ok(Some(1150)));
            assert_eq!(seq2.highest_prefix(&seq1), Ok(Some(1150)));
        }

        {
            let mut seq1 = Sequence::new(1000);
            let mut seq2 = Sequence::new(1300);

            for i in 1000u32..1200 {
                seq1.append(i * 10)
            }
            for i in 1200u32..1400 {
                seq2.append(i * 100)
            }
            assert_eq!(seq1.highest_prefix(&seq2), Err(()));
            assert_eq!(seq2.highest_prefix(&seq1), Err(()));
        }

        {
            let mut seq1 = Sequence::new(1000);
            let mut seq2 = Sequence::new(1100);

            for i in 1000u32..1200 {
                seq1.append(i * 10)
            }
            for i in 1100u32..1200 {
                seq2.append(i * 100)
            }
            assert_eq!(seq1.highest_prefix(&seq2), Ok(None));
            assert_eq!(seq2.highest_prefix(&seq1), Ok(None));
        }
    }
}
