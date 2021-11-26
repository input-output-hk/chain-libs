//! Tooling for packing and unpacking from streams
//!
//! This will allow us to expose some standard way of serializing
//! data.

use crate::deser::{ReadError, WriteError};

pub struct Codec<I>(I);
impl<I> Codec<I> {
    pub fn new(inner: I) -> Self {
        Codec(inner)
    }

    pub fn into_inner(self) -> I {
        self.0
    }
}

impl<R: std::io::BufRead> Codec<R> {
    #[inline]
    pub fn get_u8(&mut self) -> Result<u8, ReadError> {
        let mut buf = [0u8; 1];
        self.0.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    #[inline]
    pub fn get_u16(&mut self) -> Result<u16, ReadError> {
        let mut buf = [0u8; 2];
        self.0.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_u32(&mut self) -> Result<u32, ReadError> {
        let mut buf = [0u8; 4];
        self.0.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_u64(&mut self) -> Result<u64, ReadError> {
        let mut buf = [0u8; 8];
        self.0.read_exact(&mut buf)?;
        Ok(u64::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_u128(&mut self) -> Result<u128, ReadError> {
        let mut buf = [0u8; 16];
        self.0.read_exact(&mut buf)?;
        Ok(u128::from_be_bytes(buf))
    }
    #[inline]
    pub fn get_bytes(&mut self, n: usize) -> Result<Vec<u8>, ReadError> {
        let mut buf = vec![0u8; n];
        self.0.read_exact(&mut buf)?;
        Ok(buf)
    }
    #[inline]
    pub fn get_slice(&mut self, slice: &mut [u8]) -> Result<(), ReadError> {
        self.0.read_exact(slice)?;
        Ok(())
    }
}

impl<W: std::io::Write> Codec<W> {
    #[inline]
    pub fn put_u8(&mut self, v: u8) -> Result<(), WriteError> {
        self.0.write_all(&[v]).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u16(&mut self, v: u16) -> Result<(), WriteError> {
        self.0.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u32(&mut self, v: u32) -> Result<(), WriteError> {
        self.0.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u64(&mut self, v: u64) -> Result<(), WriteError> {
        self.0.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_u128(&mut self, v: u128) -> Result<(), WriteError> {
        self.0.write_all(&v.to_be_bytes()).map_err(|e| e.into())
    }
    #[inline]
    pub fn put_bytes(&mut self, v: &[u8]) -> Result<(), WriteError> {
        self.0.write_all(v).map_err(|e| e.into())
    }
}

impl<R: std::io::Read> std::io::Read for Codec<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl<BR: std::io::BufRead> std::io::BufRead for Codec<BR> {
    #[inline]
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.0.fill_buf()
    }
    #[inline]
    fn consume(&mut self, amt: usize) {
        self.0.consume(amt)
    }
}

impl<W: std::io::Write> std::io::Write for Codec<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}
