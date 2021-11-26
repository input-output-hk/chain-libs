use crate::mempack::{ReadBuf, ReadError};

#[derive(Debug)]
pub enum WriteError {
    CannotSerialize(std::io::Error),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WriteError::CannotSerialize(err) => write!(f, "CannotSerialize: err {}", err),
        }
    }
}

impl From<std::io::Error> for WriteError {
    fn from(err: std::io::Error) -> Self {
        Self::CannotSerialize(err)
    }
}

impl From<WriteError> for std::io::Error {
    fn from(err: WriteError) -> Self {
        match err {
            WriteError::CannotSerialize(err) => err,
        }
    }
}

/// Define that an object can be written to a `Write` object.
pub trait Serialize {
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), WriteError>;

    /// Convenience method to serialize into a byte vector.
    fn serialize_as_vec(&self) -> Result<Vec<u8>, WriteError> {
        let mut data = vec![];
        self.serialize(&mut data)?;
        Ok(data)
    }
}

impl<T: Serialize> Serialize for &T {
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), WriteError> {
        (**self).serialize(writer)
    }
}

pub trait Deserialize: Sized {
    fn deserialize(buf: &mut ReadBuf) -> Result<Self, ReadError>;

    fn deserialize_validate(buf: &mut ReadBuf) -> Result<(), ReadError> {
        Self::deserialize(buf).map(|_| ())
    }
}

impl Deserialize for () {
    fn deserialize(_: &mut ReadBuf) -> Result<(), ReadError> {
        Ok(())
    }
}

macro_rules! read_array_impls {
    ($($N: expr)+) => {
        $(
        impl Deserialize for [u8; $N] {
            fn deserialize<'a>(readbuf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
                let mut buf = [0u8; $N];
                buf.copy_from_slice(readbuf.get_slice($N)?);
                Ok(buf)
            }
        }
        )+
    };
}

read_array_impls! {
    4 8 12 16 20 24 28 32 64 96 128
}
