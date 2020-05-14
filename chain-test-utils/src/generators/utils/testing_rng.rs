use crate::generators::utils::R;
use rand::{CryptoRng, Error, RngCore};

/// `NonSecureRng` is a wrapper around `smoke::R`
/// This type implements `rand::CryptoRng` and `rand::RngCore` but it is not cryptographically secure,
/// hence ⚠️it **is not meant to be use in production**, it is ⚠️**just for testing** porpoises.
pub struct NonSecureRng<'a> {
    r: &'a mut R,
}

impl<'a> NonSecureRng<'a> {
    pub fn new(r: &'a mut R) -> Self {
        Self { r }
    }
}

impl RngCore for NonSecureRng<'_> {
    fn next_u32(&mut self) -> u32 {
        self.r.num()
    }

    fn next_u64(&mut self) -> u64 {
        self.r.num()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.r.next_bytes(dest)
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.r.next_bytes(dest);
        Ok(())
    }
}

impl CryptoRng for NonSecureRng<'_> {}
