use crate::key::Hash;
use chain_core::property::{Deserialize, Serialize};
use chain_ser::mempack::{ReadBuf, ReadError};

pub type FragmentId = Hash;
pub const FRAGMENT_SIZE_BYTES_LEN: usize = 4;

/// A serialized Message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentRaw(pub(super) Vec<u8>);

impl FragmentRaw {
    pub fn size_bytes_plus_size(&self) -> usize {
        FRAGMENT_SIZE_BYTES_LEN + self.0.len()
    }

    pub fn id(&self) -> FragmentId {
        FragmentId::hash_bytes(self.0.as_ref())
    }
}

impl AsRef<[u8]> for FragmentRaw {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Deserialize for FragmentRaw {
    fn deserialize(buf: &mut ReadBuf) -> Result<Self, ReadError> {
        let size = buf.get_u32()?;
        let mut v = vec![0u8; size as usize];
        buf.copy_to_slice_mut(&mut v)?;
        Ok(FragmentRaw(v))
    }
}

impl Serialize for FragmentRaw {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;

        let mut codec = Codec::new(writer);
        codec.put_u32(self.0.len() as u32)?;
        codec.into_inner().write_all(&self.0)?;
        Ok(())
    }
}
