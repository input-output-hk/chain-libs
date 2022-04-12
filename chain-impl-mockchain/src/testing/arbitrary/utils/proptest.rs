use std::fmt::Debug;

use proptest::prelude::*;

/// Take a strategy for generating any iterator, and convert it into a strategy with a random
/// subset
pub fn random_subset<I, T>(s: impl Strategy<Value = I>) -> impl Strategy<Value = I>
where
    I: IntoIterator<Item = T> + FromIterator<T> + Debug,
    T: Clone,
{
    s.prop_flat_map(|iter| {
        let items: Vec<_> = iter.into_iter().collect();
        let bitflags = proptest::collection::vec(any::<bool>(), items.len());

        bitflags.prop_map(move |bitflags| {
            items
                .iter()
                .cloned()
                .enumerate()
                .filter(|(i, _)| bitflags[*i])
                .map(|(_, t)| t)
                .collect()
        })
    })
}

/// Take a strategy for generating any iterator, and convert it into a strategy with a random
/// subset
///
/// The given iterator must contain at least one element, and the returned iterator will also
/// contain at least one element
pub fn random_non_zero_subset<I, T>(s: impl Strategy<Value = I>) -> impl Strategy<Value = I>
where
    I: IntoIterator<Item = T> + FromIterator<T> + Debug,
    T: Clone,
{
    s.prop_flat_map(|iter| {
        let items: Vec<_> = iter.into_iter().collect();
        assert!(!items.is_empty(), "iterator must be non-empty");
        let bitflags =
            proptest::collection::vec(any::<bool>(), items.len()).prop_map(|mut flags| {
                flags[0] = true;
                flags
            });

        bitflags.prop_map(move |bitflags| {
            items
                .iter()
                .cloned()
                .enumerate()
                .filter(|(i, _)| bitflags[*i])
                .map(|(_, t)| t)
                .collect()
        })
    })
}

