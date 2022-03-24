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
