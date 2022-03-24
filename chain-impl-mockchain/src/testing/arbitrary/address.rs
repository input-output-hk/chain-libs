use crate::{
    testing::{
        arbitrary::kind_type::KindTypeWithoutMultisig,
        arbitrary::AverageValue,
        data::{AddressData, AddressDataValue},
        ledger::TestLedger,
    },
    tokens::name::TokenName,
    transaction::{Input, Output},
    value::Value,
};
use chain_addr::{Address, Discrimination, Kind};
use quickcheck::{Arbitrary, Gen};
use std::{collections::HashMap, iter};

#[derive(Clone, Debug)]
pub struct ArbitraryAddressDataVec(pub Vec<AddressData>);

impl Arbitrary for ArbitraryAddressDataVec {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let size_limit = 253;
        let n = usize::arbitrary(gen) % size_limit + 1;
        let addresses = iter::from_fn(|| Some(AddressData::arbitrary(gen))).take(n);
        ArbitraryAddressDataVec(addresses.collect())
    }
}

#[derive(Clone, Debug)]
pub struct ArbitraryAddressDataValueVec(pub Vec<AddressDataValue>);

impl Arbitrary for ArbitraryAddressDataValueVec {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let size_limit = 10;
        let n = usize::arbitrary(gen) % size_limit + 1;
        let addresses = iter::from_fn(|| Some(AddressDataValue::arbitrary(gen))).take(n);
        ArbitraryAddressDataValueVec(addresses.collect())
    }
}

pub mod proptest_impls {
    use chain_addr::Kind;
    use proptest::collection::{hash_map, vec};
    use proptest::prelude::*;

    use crate::testing::data::{AddressData, AddressDataValue};
    use crate::testing::pt::kind_type_without_multisig;
    use crate::tokens::name::TokenName;
    use crate::value::Value;

    pub fn arb_address_data_value_vec() -> impl Strategy<Value = Vec<AddressDataValue>> {
        vec(any::<AddressDataValue>(), 1..=10)
    }

    fn arb_adv() -> impl Strategy<Value = AddressDataValue> {
        any::<(AddressData, Value)>().prop_flat_map(|(ad, v)| match ad.address.kind() {
            Kind::Account(_) => hash_map(any::<TokenName>(), any::<Value>(), 0..usize::MAX)
                .prop_map(move |map| AddressDataValue::new_with_tokens(ad.clone(), v, map))
                .boxed(),
            _ => Just(AddressDataValue::new(ad, v)).boxed(),
        })
    }

    impl Arbitrary for AddressDataValue {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            arb_adv().boxed()
        }
    }

    impl Arbitrary for AddressData {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            kind_type_without_multisig()
                .prop_map(|kind| {
                    AddressData::from_discrimination_and_kind_type(
                        chain_addr::Discrimination::Test,
                        kind,
                    )
                })
                .boxed()
        }
    }
}

impl ArbitraryAddressDataValueVec {
    pub fn iter(&self) -> std::slice::Iter<AddressDataValue> {
        self.0.iter()
    }

    pub fn values(&self) -> Vec<AddressDataValue> {
        self.0.clone()
    }

    pub fn as_addresses(&self) -> Vec<AddressData> {
        self.iter().cloned().map(|x| x.address_data).collect()
    }

    pub fn make_outputs(&self) -> Vec<Output<Address>> {
        self.iter().map(|x| x.make_output()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.values().len() == 0
    }

    pub fn total_value(&self) -> Value {
        Value::sum(self.iter().map(|input| input.value)).unwrap()
    }

    pub fn make_inputs(&self, ledger: &TestLedger) -> Vec<Input> {
        self.iter()
            .map(|x| {
                let utxo = ledger.find_utxo_for_address(&x.clone().into());
                x.make_input(utxo)
            })
            .collect()
    }
}

impl Arbitrary for AddressData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let kind_without_multisig = KindTypeWithoutMultisig::arbitrary(gen);
        AddressData::from_discrimination_and_kind_type(
            Discrimination::Test,
            kind_without_multisig.kind_type(),
        )
    }
}

impl Arbitrary for AddressDataValue {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let address_data = AddressData::arbitrary(gen);
        let value = AverageValue::arbitrary(gen).into();

        match address_data.address.kind() {
            Kind::Account(_) => {
                let voting_token_len = usize::arbitrary(gen);
                let arbitrary_voting_tokens =
                    iter::from_fn(|| Some((TokenName::arbitrary(gen), Value::arbitrary(gen))))
                        .take(voting_token_len)
                        .collect::<HashMap<_, _>>();

                AddressDataValue::new_with_tokens(address_data, value, arbitrary_voting_tokens)
            }
            _ => AddressDataValue::new(address_data, value),
        }
    }
}

impl ArbitraryAddressDataValueVec {
    pub fn utxos(&self) -> Vec<AddressDataValue> {
        self.0
            .iter()
            .cloned()
            .filter(|x| matches!(x.address_data.kind(), Kind::Single { .. }))
            .collect()
    }
    pub fn accounts(&self) -> Vec<AddressDataValue> {
        self.0
            .iter()
            .cloned()
            .filter(|x| matches!(x.address_data.kind(), Kind::Account { .. }))
            .collect()
    }

    pub fn delegations(&self) -> Vec<AddressDataValue> {
        self.0
            .iter()
            .cloned()
            .filter(|x| matches!(x.address_data.kind(), Kind::Group { .. }))
            .collect()
    }
}
