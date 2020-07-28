pub mod address;
pub mod config_builder;
pub mod kind_type;
pub mod ledger_builder;
pub mod output;
pub mod transaction;
pub mod update_proposal;
pub mod utils;
pub mod utxo;
pub mod wallet;

use crate::{key::BftLeaderId, transaction::Output, value::Value};
use chain_addr::Address;
use chain_crypto::{Ed25519, SecretKey};
use quickcheck::{Arbitrary, Gen};

pub use address::*;
pub use kind_type::*;
pub use output::*;
use std::cmp;
pub use transaction::*;
pub use update_proposal::*;
pub use wallet::WalletCollection;

impl Arbitrary for Value {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        Value(u64::arbitrary(gen))
    }
}

// Average value used in test where value is larger than zero
#[derive(Debug, Copy, Clone)]
pub struct NonZeroValue(pub Value);

impl Arbitrary for NonZeroValue {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        NonZeroValue(Value(cmp::max(u64::arbitrary(gen), 1)))
    }
}

impl From<NonZeroValue> for Value {
    fn from(value: NonZeroValue) -> Self {
        value.0
    }
}

// Average value used in test where value is larger than zero and not too big
// in case we would like to sum up values and not suffer with buffer overflow
#[derive(Debug, Copy, Clone)]
pub struct AverageValue(pub Value);

impl Arbitrary for AverageValue {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        AverageValue(Value(u64::arbitrary(gen) % 10000 + 253))
    }
}

impl From<AverageValue> for Value {
    fn from(value: AverageValue) -> Self {
        value.0
    }
}

impl Arbitrary for Output<Address> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Output {
            address: Arbitrary::arbitrary(g),
            value: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for BftLeaderId {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mut seed = [0; 32];
        for byte in seed.iter_mut() {
            *byte = Arbitrary::arbitrary(g);
        }
        let sk: SecretKey<Ed25519> = Arbitrary::arbitrary(g);
        BftLeaderId(sk.to_public())
    }
}
