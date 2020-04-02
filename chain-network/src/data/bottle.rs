/// A "bottle in the sea" raw message.
#[derive(Clone)]
pub struct BottleInSea(Box<[u8]>);

impl BottleInSea {
    #[inline]
    pub fn from_bytes<B: Into<Box<[u8]>>>(bytes: B) -> Self {
        Self(bytes.into())
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[inline]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0.into()
    }
}

impl AsRef<[u8]> for BottleInSea {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl From<BottleInSea> for Vec<u8> {
    #[inline]
    fn from(block: BottleInSea) -> Self {
        block.into_bytes()
    }
}
