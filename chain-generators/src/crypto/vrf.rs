use chain_crypto::algorithms::vrf::vrf;
use smoke::{Generator, R};

pub struct VRFSecretKeyGenerator {}

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
    use chain_crypto::algorithms::vrf::vrf::Scalar;

    #[test]
    fn check_generates() {
        let mut seed = smoke::Seed::from(1_000_0000u128);
        let mut r = smoke::R::from_seed(seed);
        let gen = VRFSecretKeyGenerator {};
        let sk = gen.gen(&mut r);
        // sk.
    }
}
