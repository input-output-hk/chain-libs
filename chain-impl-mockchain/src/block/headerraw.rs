use chain_core::property::{Deserialize, Serialize};

/// Block Header Bytes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderRaw(pub(super) Vec<u8>);

impl AsRef<[u8]> for HeaderRaw {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Serialize for HeaderRaw {
    type Error = std::io::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;

        let mut codec = Codec::new(writer);
        codec.put_u16(self.0.len() as u16)?;
        codec.write_all(&self.0)?;
        Ok(())
    }
}

impl Deserialize for HeaderRaw {
    fn deserialize(
        buf: &mut chain_core::mempack::ReadBuf,
    ) -> Result<Self, chain_core::mempack::ReadError> {
        let header_size = buf.get_u16()? as usize;
        let mut v = vec![0u8; header_size];
        buf.copy_to_slice_mut(&mut v)?;
        Ok(HeaderRaw(v))
    }
}
