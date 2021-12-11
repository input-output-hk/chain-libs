//! Tooling for packing and unpacking from streams
//!
//! This will allow us to expose some standard way of serializing
//! data.

use crate::deser::{ReadError, WriteError};
use std::num::{NonZeroU32, NonZeroU64};

pub struct Codec<I> {
    inner: I,
}
impl<I> Codec<I> {
    pub fn new(inner: I) -> Self {
        Codec { inner }
    }

    pub fn into_inner(self) -> I {
        self.inner
    }
}

impl<R: std::io::Read> Codec<R> {
    #[inline]
    pub fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, ReadError> {
        let res = self.inner.read_to_end(buf)?;
        Ok(res)
    }
}

impl<R: std::io::BufRead> Codec<R> {
    #[inline]
    pub fn get_u8(&mut self) -> Result<u8, ReadError> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    #[inline]
    pub fn get_u16(&mut self) -> Result<u16, ReadError> {
        let mut buf = [0u8; 2];
        self.inner.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_u32(&mut self) -> Result<u32, ReadError> {
        let mut buf = [0u8; 4];
        self.inner.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_u64(&mut self) -> Result<u64, ReadError> {
        let mut buf = [0u8; 8];
        self.inner.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_u128(&mut self) -> Result<u128, ReadError> {
        let mut buf = [0u8; 16];
        self.inner.read_exact(&mut buf)?;
        Ok(u128::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_nz_u32(&mut self) -> Result<NonZeroU32, ReadError> {
        let val = self.get_u32()?;
        NonZeroU32::new(val)
            .ok_or_else(|| ReadError::StructureInvalid("received zero u32".to_string()))
    }
    #[inline]
    pub fn get_nz_u64(&mut self) -> Result<NonZeroU64, ReadError> {
        let val = self.get_u64()?;
        NonZeroU64::new(val)
            .ok_or_else(|| ReadError::StructureInvalid("received zero u64".to_string()))
    }
    #[inline]
    pub fn get_bytes(&mut self, n: usize) -> Result<Vec<u8>, ReadError> {
        let mut buf = vec![0u8; n];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }
    #[inline]
    /// This is a wrapper over the std::io::BufRead::fill_buf() function,
    /// so be aware of that you need also execute consume() function to move the reader position
    pub fn get_slice(&mut self, n: usize) -> Result<&[u8], ReadError> {
        let data = self.inner.fill_buf()?;
        if data.len() < n {
            return Err(ReadError::NotEnoughBytes(data.len(), n));
        }
        Ok(&data[..n])
    }
    #[inline]
    pub fn copy_to_slice(&mut self, slice: &mut [u8]) -> Result<(), ReadError> {
        self.inner.read_exact(slice)?;
        Ok(())
    }
}

impl<W: std::io::Write> Codec<W> {
    #[inline]
    pub fn put_u8(&mut self, v: u8) -> Result<(), WriteError> {
        self.inner.write_all(&[v]).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u16(&mut self, v: u16) -> Result<(), WriteError> {
        self.inner.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u32(&mut self, v: u32) -> Result<(), WriteError> {
        self.inner.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u64(&mut self, v: u64) -> Result<(), WriteError> {
        self.inner.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u128(&mut self, v: u128) -> Result<(), WriteError> {
        self.inner.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_bytes(&mut self, v: &[u8]) -> Result<(), WriteError> {
        self.inner.write_all(v).map_err(|e| e.into())
    }
}

impl<T> Codec<std::io::Cursor<T>> {
    #[inline]
    pub fn position(&mut self) -> usize {
        self.inner.position() as usize
    }
    #[inline]
    pub fn set_position(&mut self, pos: usize) {
        self.inner.set_position(pos as u64)
    }
}

impl<R: std::io::Read> std::io::Read for Codec<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<BR: std::io::BufRead> std::io::BufRead for Codec<BR> {
    #[inline]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.inner.fill_buf()
    }
    #[inline]
    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

impl<W: std::io::Write> std::io::Write for Codec<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }
    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
