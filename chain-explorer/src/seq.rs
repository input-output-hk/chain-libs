use super::endian::B64;
use sanakirja::{direct_repr, Storable, UnsizedStorable};
use zerocopy::{AsBytes, FromBytes};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, FromBytes, AsBytes)]
#[repr(C)]
pub struct SeqNum(B64);

direct_repr!(SeqNum);

impl SeqNum {
    pub const MAX: SeqNum = SeqNum(B64(zerocopy::U64::<byteorder::BigEndian>::MAX_VALUE));
    pub const MIN: SeqNum = SeqNum(B64(zerocopy::U64::<byteorder::BigEndian>::ZERO));

    pub fn new(n: u64) -> Self {
        Self(B64::new(n))
    }

    pub fn next(self) -> SeqNum {
        Self::new(self.0.get() + 1)
    }
}

impl From<SeqNum> for u64 {
    fn from(n: SeqNum) -> Self {
        n.0.get()
    }
}

impl From<u64> for SeqNum {
    fn from(n: u64) -> Self {
        SeqNum::new(n)
    }
}
