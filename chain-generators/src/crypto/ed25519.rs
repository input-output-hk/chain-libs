use chain_crypto;
use chain_test_utils::generators::utils as generator_utils;
use smoke::{Generator, R};

pub struct Ed25519Generator();

impl Ed25519Generator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Ed25519Generator {
    fn default() -> Self {
        Self()
    }
}

impl Generator for Ed25519Generator {
    type Item = chain_crypto::KeyPair<chain_crypto::Ed25519>;

    fn gen(&self, r: &mut R) -> Self::Item {
        let mut non_secure_rng = generator_utils::testing_rng::NonSecureRng::new(r);
        chain_crypto::KeyPair::generate(&mut non_secure_rng)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::random;

    #[test]
    fn check_generates_ed25519_secret_key() {
        let n: u128 = random();
        let seed = smoke::Seed::from(n);
        let mut r = smoke::R::from_seed(seed);
        let gen = Ed25519Generator::new();
        for _ in 0..100 {
            gen.gen(&mut r);
        }
    }
}
