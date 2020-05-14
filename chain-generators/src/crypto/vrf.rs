use chain_crypto::algorithms::vrf::vrf;
use smoke::{Generator, R};

/// This type implements a  `vrf::SecretKey` generator.
/// It is **not guaranteed** that the generated keys will be unique.
pub struct VRFSecretKeyGenerator();

impl VRFSecretKeyGenerator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for VRFSecretKeyGenerator {
    fn default() -> Self {
        Self()
    }
}

impl Generator for VRFSecretKeyGenerator {
    type Item = vrf::SecretKey;

    fn gen(&self, r: &mut R) -> Self::Item {
        let scalar = vrf::Scalar::from(R::num::<u128>(r));
        // It should be safe to unwrap here since we are using the bytes from a valid Scalar
        vrf::SecretKey::from_bytes(*scalar.as_bytes()).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::random;

    #[test]
    fn generates_vrf_secret_key() {
        let n: u128 = random();
        let seed = smoke::Seed::from(n);
        let mut r = smoke::R::from_seed(seed);
        let gen = VRFSecretKeyGenerator::new();
        for _ in 0..100 {
            gen.gen(&mut r);
        }
    }
}
