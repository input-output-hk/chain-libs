use chain_crypto::hash;
use chain_test_utils::generators::utils::{Generator, R};

pub struct Blake2b256Generator<T, Gen>
where
    T: AsRef<[u8]>,
    Gen: Generator<Item = T>,
{
    gen: Gen,
}

impl<T, Gen: Generator<Item = T>> Blake2b256Generator<T, Gen>
where
    T: AsRef<[u8]>,
    Gen: Generator<Item = T>,
{
    pub fn new(g: Gen) -> Self {
        Self { gen: g }
    }
}

impl<T, Gen> Generator for Blake2b256Generator<T, Gen>
where
    T: AsRef<[u8]>,
    Gen: Generator<Item = T>,
{
    type Item = hash::Blake2b256;

    fn gen(&self, r: &mut R) -> Self::Item {
        hash::Blake2b256::new(self.gen.gen(r).as_ref())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chain_test_utils::generators::utils::ConstantGenerator;
    use rand::random;
    #[test]
    fn generates_ed25519_secret_key() {
        let n: u128 = random();
        let seed = smoke::Seed::from(n);
        let mut r = smoke::R::from_seed(seed);
        let value = vec![255u8; 1000];
        let const_gen = ConstantGenerator::new(value);
        let gen = Blake2b256Generator::new(const_gen);
        for _ in 0..100 {
            gen.gen(&mut r);
        }
    }
}
