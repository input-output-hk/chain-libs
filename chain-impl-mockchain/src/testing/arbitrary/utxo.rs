use super::AverageValue;
use crate::{key::Hash, testing::data::AddressData, transaction::Output, utxo::Ledger};
use chain_addr::{Address, Discrimination};
use quickcheck::{Arbitrary, Gen};
use std::{collections::HashMap, iter};

#[derive(Debug, Clone)]
pub struct ArbitaryLedgerUtxo(pub Ledger<Address>);

impl Arbitrary for ArbitaryLedgerUtxo {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut ledger = Ledger::new();
        let size = usize::arbitrary(g) % 50 + 1;
        let arbitrary_utxos: HashMap<Hash, (u8, Output<Address>)> = iter::from_fn(|| {
            let outs = match u8::arbitrary(g) % 2 {
                0 => (
                    0u8,
                    AddressData::utxo(Discrimination::Test)
                        .make_output(AverageValue::arbitrary(g).into()),
                ),
                1 => (
                    0u8,
                    AddressData::delegation(Discrimination::Test)
                        .make_output(AverageValue::arbitrary(g).into()),
                ),
                _ => unreachable!(),
            };
            Some((Hash::arbitrary(g), outs))
        })
        .take(size)
        .collect();

        for (key, value) in arbitrary_utxos {
            ledger = ledger.add(&key, &[value]).unwrap();
        }
        ArbitaryLedgerUtxo(ledger)
    }
}

pub mod prop_impls {
    use std::collections::HashMap;

    use chain_addr::{Address, Discrimination};
    use proptest::{collection::hash_map, prelude::*};

    use crate::{
        key::Hash,
        testing::{average_value, data::AddressData},
        transaction::Output,
        utxo::Ledger,
    };

    pub fn utxo_strat() -> impl Strategy<Value = Ledger<Address>> {
        (1..=50usize, average_value())
            .prop_flat_map(|(size, value)| {
                let out = prop_oneof![
                    Just(AddressData::utxo(Discrimination::Test).make_output(value)),
                    Just(AddressData::delegation(Discrimination::Test).make_output(value)),
                ];
                hash_map(any::<Hash>(), (Just(0u8), out), size)
            })
            .prop_map(|map: HashMap<Hash, (u8, Output<Address>)>| {
                let mut ledger = Ledger::new();
                for (key, value) in map {
                    ledger = ledger.add(&key, &[value]).unwrap();
                }
                ledger
            })
    }
}
