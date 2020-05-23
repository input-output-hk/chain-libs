use chain_crypto::algorithms::vrf::vrf;
use chain_test_utils::generators::utils::{Generator, R};

/// This type implements a  `vrf::SecretKey` generator.
/// It is **not guaranteed** that the generated keys will be unique.
pub struct VRFSecretKeyGenerator<Gen: Generator<Item = u128>> {
    inner_gen: Gen,
}

impl<Gen: Generator<Item = u128>> VRFSecretKeyGenerator<Gen> {
    pub fn new(inner_gen: Gen) -> Self {
        Self { inner_gen }
    }
}

impl<Gen: Generator<Item = u128>> Generator for VRFSecretKeyGenerator<Gen> {
    type Item = vrf::SecretKey;

    fn gen(&self, r: &mut R) -> Self::Item {
        let scalar = vrf::Scalar::from(self.inner_gen.gen(r));
        // It should be safe to unwrap here since we are using the bytes from a valid Scalar
        vrf::SecretKey::from_bytes(*scalar.as_bytes()).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chain_test_utils::generators::unique::Unique;
    use smoke::generator::num;

    #[test]
    fn generates_vrf_secret_key() {
        let (_, mut r) = smoke::R::new();
        let gen = VRFSecretKeyGenerator::new(num());
        for _ in 0..100 {
            gen.gen(&mut r);
        }
    }

    #[test]
    fn generates_unique_vrf_secret_key() {
        let (_, mut r) = smoke::R::new();
        let vrf_generator = VRFSecretKeyGenerator::new(Unique::new(num()));
        let vrf_set: Vec<vrf::SecretKey> = (0..100).map(|_| vrf_generator.gen(&mut r)).collect();
        assert_eq!(vrf_set.len(), 100);
    }
}
