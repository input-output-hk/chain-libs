use chain_crypto::hash;
use chain_test_utils::generators::utils::{Generator, R};

const DEFAULT_BUFFER_SIZE: usize = 128;

pub struct Blake2b256Generator {
    buffer_size: usize,
}

impl Blake2b256Generator {
    pub fn new(buffer_size: usize) -> Self {
        Self { buffer_size }
    }
}

impl Default for Blake2b256Generator {
    fn default() -> Self {
        Self::new(DEFAULT_BUFFER_SIZE)
    }
}

impl Generator for Blake2b256Generator {
    type Item = hash::Blake2b256;

    fn gen(&self, r: &mut R) -> Self::Item {
        let mut buff = vec![0u8; self.buffer_size];
        r.next_bytes(buff.as_mut());
        hash::Blake2b256::new(buff.as_slice())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::random;

    #[test]
    fn generates_ed25519_secret_key() {
        let n: u128 = random();
        let seed = smoke::Seed::from(n);
        let mut r = smoke::R::from_seed(seed);
        let gen = Blake2b256Generator::default();
        for _ in 0..100 {
            gen.gen(&mut r);
        }
    }
}
