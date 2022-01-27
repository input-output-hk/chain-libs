use crate::packer::Codec;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("not enough bytes: expected {0} but got {1}")]
    NotEnoughBytes(usize, usize),
    #[error("unconsumed data: {0} bytes left")]
    UnconsumedData(usize),
    #[error("too much bytes: expected {0} but got {1}")]
    SizeTooBig(usize, usize),
    #[error("invalid structure: {0}")]
    StructureInvalid(String),
    #[error("unknown tag: {0}")]
    UnknownTag(u32),
    #[error("invalid structure: {0}")]
    InvalidData(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

/// Define that an object can be written to a `Write` object.
pub trait Serialize {
    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), std::io::Error>;

    /// Convenience method to serialize into a byte vector.
    fn serialize_as_vec(&self) -> Result<Vec<u8>, std::io::Error> {
        let mut data = Vec::new();
        self.serialize(&mut Codec::new(&mut data))?;
        Ok(data)
    }
}

impl<T: Serialize> Serialize for &T {
    fn serialize<W: std::io::Write>(&self, codec: &mut Codec<W>) -> Result<(), std::io::Error> {
        (*self).serialize(codec)
    }
}

pub trait Deserialize: Sized {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError>;

    fn deserialize_validate<R: std::io::Read>(codec: &mut Codec<R>) -> Result<(), ReadError> {
        Self::deserialize(codec).map(|_| ())
    }
}

pub trait DeserializeFromSlice: Sized {
    fn deserialize_from_slice(codec: &mut Codec<&[u8]>) -> Result<Self, ReadError>;

    fn deserialize_validate_from_slice(codec: &mut Codec<&[u8]>) -> Result<(), ReadError> {
        Self::deserialize_from_slice(codec).map(|_| ())
    }
}

impl<T: Deserialize> DeserializeFromSlice for T {
    fn deserialize_from_slice(codec: &mut Codec<&[u8]>) -> Result<Self, ReadError> {
        Self::deserialize(codec)
    }
}

impl Deserialize for () {
    fn deserialize<R: std::io::Read>(_: &mut Codec<R>) -> Result<(), ReadError> {
        Ok(())
    }
}

impl<const N: usize> Deserialize for [u8; N] {
    fn deserialize<R: std::io::Read>(codec: &mut Codec<R>) -> Result<Self, ReadError> {
        let mut buf = [0u8; N];
        codec.copy_to_slice(&mut buf)?;
        Ok(buf)
    }
}
