use crate::packer::Codec;

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

#[derive(Debug, PartialEq, Eq)]
pub enum ReadError {
    /// Return the number of bytes left and the number of bytes demanded
    NotEnoughBytes(usize, usize),
    /// Data is left in the buffer
    UnconsumedData(usize),
    /// Expecting a size that is above the limit
    SizeTooBig(usize, usize),
    /// Structure of data is not what it should be
    StructureInvalid(String),
    /// Unknown enumeration tag
    UnknownTag(u32),
    /// Structure is correct but data is not valid,
    /// for example because an invariant does not hold
    InvalidData(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ReadError::NotEnoughBytes(left, demanded) => write!(
                f,
                "NotEnoughBytes: demanded {} bytes but got {}",
                demanded, left
            ),
            ReadError::UnconsumedData(len) => write!(f, "Unconsumed data: {} bytes left", len),
            ReadError::SizeTooBig(e, limit) => write!(
                f,
                "Ask for number of elements {} above expected limit value: {}",
                e, limit
            ),
            ReadError::StructureInvalid(s) => write!(f, "Structure invalid: {}", s),
            ReadError::UnknownTag(t) => write!(f, "Unknown tag: {}", t),
            ReadError::InvalidData(s) => write!(f, "Invalid data: {}", s),
        }
    }
}

impl From<std::io::Error> for ReadError {
    fn from(err: std::io::Error) -> Self {
        ReadError::InvalidData(err.to_string())
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
        (*self).serialize(writer)
    }
}

pub trait Deserialize: Sized {
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, ReadError>;

    fn deserialize_validate<R: std::io::BufRead>(reader: R) -> Result<(), ReadError> {
        Self::deserialize(reader).map(|_| ())
    }
}

impl Deserialize for () {
    fn deserialize<R: std::io::BufRead>(_: R) -> Result<(), ReadError> {
        Ok(())
    }
}

macro_rules! read_array_impls {
    ($($N: expr)+) => {
        $(
        impl Deserialize for [u8; $N] {
            fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, ReadError> {
                let mut buf = [0u8; $N];
                let mut codec = Codec::new(reader);
                codec.get_slice(&mut buf)?;
                Ok(buf)
            }
        }
        )+
    };
}

read_array_impls! {
    4 8 12 16 20 24 28 32 64 96 128
}
