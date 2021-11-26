use std::num::{NonZeroU32, NonZeroU64};

use crate::deser::ReadError;

/// A local memory slice to read from memory
pub struct ReadBuf<'a> {
    offset: usize,
    data: &'a [u8],
    //trace: Vec<(usize, String)>,
}

impl<'a> ReadBuf<'a> {
    /// Create a readbuf from a slice
    pub fn from(slice: &'a [u8]) -> Self {
        ReadBuf {
            offset: 0,
            data: slice,
            //trace: Vec::new(),
        }
    }

    pub fn position(&self) -> usize {
        self.offset
    }

    fn left(&self) -> usize {
        self.data.len() - self.offset
    }

    fn assure_size(&self, expected: usize) -> Result<(), ReadError> {
        let left = self.left();
        if left >= expected {
            Ok(())
        } else {
            Err(ReadError::NotEnoughBytes(left, expected))
        }
    }

    /// Check if everything has been properly consumed
    pub fn expect_end(&mut self) -> Result<(), ReadError> {
        let l = self.left();
        if l == 0 {
            Ok(())
        } else {
            Err(ReadError::UnconsumedData(l))
        }
    }

    /// Check if we reach the end of the buffer
    pub fn is_end(&self) -> bool {
        self.left() == 0
    }

    /// Skip a number of bytes from the buffer.
    pub fn skip_bytes(&mut self, sz: usize) -> Result<(), ReadError> {
        self.assure_size(sz)?;
        self.offset += sz;
        Ok(())
    }

    /// Return a slice of the next bytes from the buffer
    pub fn get_slice(&mut self, sz: usize) -> Result<&'a [u8], ReadError> {
        self.assure_size(sz)?;
        let s = &self.data[self.offset..self.offset + sz];
        self.offset += sz;
        Ok(s)
    }

    pub fn get_slice_end(&mut self) -> &'a [u8] {
        let s = &self.data[self.offset..];
        self.offset = self.data.len();
        s
    }

    pub fn copy_to_slice_mut(&mut self, slice: &mut [u8]) -> Result<(), ReadError> {
        let s = self.get_slice(slice.len())?;
        slice.copy_from_slice(s);
        Ok(())
    }

    /// Return a sub-buffer ending at the given byte offset
    pub fn split_to(&mut self, sz: usize) -> Result<ReadBuf<'a>, ReadError> {
        let slice = self.get_slice(sz)?;
        Ok(ReadBuf::from(slice))
    }

    /// Peek at the next u8 from the buffer. the cursor is **not** advanced to the next byte.
    pub fn peek_u8(&mut self) -> Result<u8, ReadError> {
        self.assure_size(1)?;
        let v = self.data[self.offset];
        Ok(v)
    }

    /// Return the next u8 from the buffer
    pub fn get_u8(&mut self) -> Result<u8, ReadError> {
        self.assure_size(1)?;
        let v = self.data[self.offset];
        self.offset += 1;
        Ok(v)
    }

    /// Return the next u16 from the buffer
    pub fn get_u16(&mut self) -> Result<u16, ReadError> {
        const SIZE: usize = 2;
        let mut buf = [0u8; SIZE];
        buf.copy_from_slice(self.get_slice(SIZE)?);
        Ok(u16::from_be_bytes(buf))
    }

    /// Return the next u32 from the buffer
    pub fn get_u32(&mut self) -> Result<u32, ReadError> {
        const SIZE: usize = 4;
        let mut buf = [0u8; SIZE];
        buf.copy_from_slice(self.get_slice(SIZE)?);
        Ok(u32::from_be_bytes(buf))
    }

    pub fn get_nz_u32(&mut self) -> Result<NonZeroU32, ReadError> {
        let v = self.get_u32()?;
        NonZeroU32::new(v)
            .ok_or_else(|| ReadError::StructureInvalid("received zero u32".to_string()))
    }

    /// Return the next u64 from the buffer
    pub fn get_u64(&mut self) -> Result<u64, ReadError> {
        const SIZE: usize = 8;
        let mut buf = [0u8; SIZE];
        buf.copy_from_slice(self.get_slice(SIZE)?);
        Ok(u64::from_be_bytes(buf))
    }

    pub fn get_nz_u64(&mut self) -> Result<NonZeroU64, ReadError> {
        let v = self.get_u64()?;
        NonZeroU64::new(v)
            .ok_or_else(|| ReadError::StructureInvalid("received zero u64".to_string()))
    }

    /// Return the next u128 from the buffer
    pub fn get_u128(&mut self) -> Result<u128, ReadError> {
        const SIZE: usize = 16;
        let mut buf = [0u8; SIZE];
        buf.copy_from_slice(self.get_slice(SIZE)?);
        Ok(u128::from_be_bytes(buf))
    }

    pub fn debug(&self) -> String {
        let mut s = String::new();
        for (i, x) in self.data.iter().enumerate() {
            //self.trace.iter().find(|(ofs,_)| ofs == &i).map(|(_,name)| { s.push_str(&name); s.push(' ') });
            if i == self.offset {
                s.push_str(".. ");
            }
            let bytes = format!("{:02x} ", x);
            s.push_str(&bytes);
        }
        s
    }
}
