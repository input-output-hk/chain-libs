use chain_addr::{Kind, KindType};
use quickcheck::{Arbitrary, Gen};
use std::iter;

#[derive(Clone, Debug)]
pub struct KindTypeWithoutMultisig(pub KindType);

impl Arbitrary for KindTypeWithoutMultisig {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        KindTypeWithoutMultisig(
            iter::from_fn(|| Some(KindType::arbitrary(g)))
                .find(|x| !matches!(x, KindType::Multisig | KindType::Script))
                .unwrap(),
        )
    }
}

pub mod pt {
    use chain_addr::KindType;
    use proptest::prelude::*;

    pub fn kind_type_without_multisig() -> impl Strategy<Value = KindType> {
        any::<u8>().prop_map(|i| match i % 3 {
            0 => KindType::Single,
            1 => KindType::Group,
            2 => KindType::Account,
            _ => unreachable!(),
        })
    }
}

impl KindTypeWithoutMultisig {
    pub fn kind_type(&self) -> KindType {
        self.0
    }
}

impl From<KindTypeWithoutMultisig> for KindType {
    fn from(kind_type_without_multisig: KindTypeWithoutMultisig) -> Self {
        kind_type_without_multisig.kind_type()
    }
}

#[derive(Clone, Debug)]
pub struct KindWithoutMultisig(pub Kind);

impl Arbitrary for KindWithoutMultisig {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        KindWithoutMultisig(
            iter::from_fn(|| Some(Kind::arbitrary(g)))
                .find(|x| !matches!(x, Kind::Multisig { .. }))
                .unwrap(),
        )
    }
}
