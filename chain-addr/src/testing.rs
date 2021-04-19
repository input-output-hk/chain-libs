use crate::{Address, AddressReadable, Discrimination, Kind, KindType};
use chain_crypto::{Ed25519, KeyPair, PublicKey};
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for Discrimination {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match u8::arbitrary(g) % 2 {
            0 => Discrimination::Production,
            1 => Discrimination::Test,
            _ => unreachable!(),
        }
    }
}

impl Arbitrary for KindType {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match u8::arbitrary(g) % 5 {
            0 => KindType::Single,
            1 => KindType::Group,
            2 => KindType::Account,
            3 => KindType::Multisig,
            4 => KindType::Script,
            _ => unreachable!(),
        }
    }
}

const ARBITRARY_PREFIX: &str = "qaddr";

impl Arbitrary for AddressReadable {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        AddressReadable::from_address(ARBITRARY_PREFIX, &Arbitrary::arbitrary(g))
    }
}

fn arbitrary_public_key<G: Gen>(g: &mut G) -> PublicKey<Ed25519> {
    let kp: KeyPair<Ed25519> = Arbitrary::arbitrary(g);
    kp.into_keys().1
}

fn arbitrary_32bytes<G: Gen>(g: &mut G) -> [u8; 32] {
    let mut h = [0u8; 32];
    for i in h.iter_mut() {
        *i = Arbitrary::arbitrary(g)
    }
    h
}

impl Arbitrary for Address {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let discrimination = Discrimination::arbitrary(g);
        let kind = Kind::arbitrary(g);
        Self(discrimination, kind)
    }
}

impl Arbitrary for Kind {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        match u8::arbitrary(g) % 5 {
            0 => Kind::Single(arbitrary_public_key(g)),
            1 => Kind::Group(arbitrary_public_key(g), arbitrary_public_key(g)),
            2 => Kind::Account(arbitrary_public_key(g)),
            3 => {
                let h = arbitrary_32bytes(g);
                Kind::Multisig(h)
            }
            4 => Kind::Script(arbitrary_32bytes(g)),
            _ => unreachable!(),
        }
    }
}
