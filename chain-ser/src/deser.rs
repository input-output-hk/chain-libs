use crate::mempack::{ReadBuf, ReadError};

/// Define that an object can be written to a `Write` object.
pub trait Serialize {
    type Error: std::error::Error + From<std::io::Error>;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error>;

    /// Convenience method to serialize into a byte vector.
    fn serialize_as_vec(&self) -> Result<Vec<u8>, Self::Error> {
        let mut data = vec![];
        self.serialize(&mut data)?;
        Ok(data)
    }
}

impl<T: Serialize> Serialize for &T {
    type Error = T::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), T::Error> {
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
