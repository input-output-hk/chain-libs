use chain_crypto::hash;
use chain_test_utils::generators::utils::{Generator, R};

/// A Blake2b256 hash generator. It consumes another generator of values that can be transformed
/// into a `&[u8]`. For example, a constant hash generator:
/// ```
/// use smoke::generator::constant;
/// use chain_generators::crypto::hash::Blake2b256Generator;
/// use chain_test_utils::generators::utils::R;
///
/// let (_, mut r) = R::new();
/// let value = vec![255u8; 1000];
/// let const_gen = constant(value);
/// let gen = Blake2b256Generator::new(const_gen);
/// ```
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
    use chain_test_utils::generators::utils::R;
    use smoke::generator::constant;
    #[test]
    fn generates_ed25519_secret_key() {
        let (_, mut r) = R::new();
        let value = vec![255u8; 1000];
        let const_gen = constant(value);
        let gen = Blake2b256Generator::new(const_gen);
        for _ in 0..100 {
            gen.gen(&mut r);
        }
    }
}
