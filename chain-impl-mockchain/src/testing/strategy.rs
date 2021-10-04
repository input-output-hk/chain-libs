use std::{
    convert::TryFrom,
    fmt::Debug,
    num::{NonZeroU32, NonZeroU64},
};

use proptest::{collection::vec, prelude::*, sample::select};

// TODO proptest this should be implemented in proptest itself
pub fn non_zero_u32() -> impl Strategy<Value = NonZeroU32> {
    (1..u32::MAX).prop_map(|value| NonZeroU32::try_from(value).unwrap())
}

pub fn non_zero_u64() -> impl Strategy<Value = NonZeroU64> {
    (1..u64::MAX).prop_map(|value| NonZeroU64::try_from(value).unwrap())
}

pub fn optional_non_zero_u64() -> impl Strategy<Value = Option<NonZeroU64>> {
    proptest::option::of(non_zero_u64())
}

pub fn kind_type_without_multisig() -> impl Strategy<Value = chain_addr::KindType> {
    any::<chain_addr::KindType>().prop_filter("only non-multisig variants are accepted", |kt| {
        kt != &chain_addr::KindType::Multisig
    })
}

pub fn address_without_multisig() -> impl Strategy<Value = chain_addr::Address> {
    any::<chain_addr::Kind>()
        .prop_filter("only non-multisig variants are accepted", |k| {
            !matches!(k, chain_addr::Kind::Multisig(_))
        })
        .prop_map(|k| chain_addr::Address(chain_addr::Discrimination::Test, k))
}

pub fn choose_random_vec_subset<T: Debug + Clone + 'static>(
    v: Vec<T>,
    max_len: Option<usize>,
) -> impl Strategy<Value = Vec<T>> {
    let max_len = max_len.unwrap_or_else(|| v.len());
    vec(select(v), 0..max_len)
}
