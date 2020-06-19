use super::{BTreeStoreError, PageId};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::convert::TryInto;
use std::io::{Read, Write};

pub(crate) const FIRST_PAGE_ID: PageId = 1;
pub(crate) const NULL_PAGE_ID: PageId = 0;

pub(crate) trait PageIdGenerator {
    fn next_id(&self) -> PageId;
    fn new_id(&mut self) -> PageId;
}

/// struct that keeps track of the next page id we can (re) use
#[derive(Debug, Clone)]
pub(crate) struct PageManager {
    pub next_page: PageId,
    pub free_pages: Vec<PageId>,
}

impl<T: std::ops::DerefMut<Target = PageManager>> PageIdGenerator for T {
    fn next_id(&self) -> PageId {
        self.next_page
    }

    fn new_id(&mut self) -> PageId {
        PageManager::new_id(self)
    }
}

impl PageManager {
    #[cfg(test)]
    pub(crate) fn free_pages(&self) -> &Vec<PageId> {
        &self.free_pages
    }

    pub fn new_id(&mut self) -> PageId {
        self.free_pages.pop().unwrap_or_else(|| {
            let result = self.next_page;
            self.next_page += 1;
            result
        })
    }

    pub(crate) fn write(&self, writer: &mut impl Write) -> Result<(), BTreeStoreError> {
        writer.write_u32::<LittleEndian>(self.next_page)?;
        writer.write_u32::<LittleEndian>(self.free_pages.len().try_into().unwrap())?;

        for page_number in self.free_pages.iter() {
            writer.write_u32::<LittleEndian>(*page_number)?;
        }

        Ok(())
    }

    pub(crate) fn read(reader: &mut impl Read) -> Result<PageManager, BTreeStoreError> {
        let next_page = reader.read_u32::<LittleEndian>()?;
        let free_pages_len = reader.read_u32::<LittleEndian>()?;

        let mut free_pages = vec![];
        for _i in 0..free_pages_len {
            free_pages.push(reader.read_u32::<LittleEndian>()?);
        }

        Ok(PageManager {
            free_pages,
            next_page,
        })
    }

    pub(crate) fn remove_page(&mut self, id: PageId) {
        self.free_pages.push(id)
    }
}
