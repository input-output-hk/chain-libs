use chain_test_utils::{
    generators::utils as generator_utils,
    generators::utils::{Generator, R},
};

/// This type implements a [`ed25519`](https://ed25519.cr.yp.to/) key generator
/// The generated keys are random but **it does not use a cryptographically secure generator**.
/// Due to that ⚠️**it is not meant to be use in production**, it is ⚠️**just for testing** porpoises.
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

    #[test]
    fn generates_ed25519_secret_key() {
        let (_, mut r) = smoke::R::new();
        let gen = Ed25519Generator::new();
        for _ in 0..100 {
            gen.gen(&mut r);
        }
    }
}
