use crate::ledger::Error;
use crate::treasury::Treasury;
use crate::value::{Value, ValueError};
use std::cmp;
use std::fmt::Debug;

/// Special pots of money
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pots {
    pub(crate) fees: Value,
    pub(crate) treasury: Treasury,
    pub(crate) rewards: Value,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Entry {
    Fees(Value),
    Treasury(Value),
    Rewards(Value),
}

#[derive(Debug, Clone, Copy)]
pub enum EntryType {
    Fees,
    Treasury,
    Rewards,
}

impl Entry {
    pub fn value(&self) -> Value {
        match self {
            Entry::Fees(v) => *v,
            Entry::Treasury(v) => *v,
            Entry::Rewards(v) => *v,
        }
    }

    pub fn entry_type(&self) -> EntryType {
        match self {
            Entry::Fees(_) => EntryType::Fees,
            Entry::Treasury(_) => EntryType::Treasury,
            Entry::Rewards(_) => EntryType::Rewards,
        }
    }
}

pub enum IterState {
    Fees,
    Treasury,
    Rewards,
    Done,
}

pub struct Entries<'a> {
    pots: &'a Pots,
    it: IterState,
}

pub struct Values<'a>(Entries<'a>);

impl<'a> Iterator for Entries<'a> {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        match self.it {
            IterState::Fees => {
                self.it = IterState::Treasury;
                Some(Entry::Fees(self.pots.fees))
            }
            IterState::Treasury => {
                self.it = IterState::Rewards;
                Some(Entry::Treasury(self.pots.treasury.value()))
            }
            IterState::Rewards => {
                self.it = IterState::Done;
                Some(Entry::Rewards(self.pots.rewards))
            }
            IterState::Done => None,
        }
    }
}

impl<'a> Iterator for Values<'a> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|e| e.value())
    }
}

impl Pots {
    /// Create a new empty set of pots
    pub fn zero() -> Self {
        Pots {
            fees: Value::zero(),
            treasury: Treasury::initial(Value::zero()),
            rewards: Value::zero(),
        }
    }

    pub fn entries(&self) -> Entries<'_> {
        Entries {
            pots: self,
            it: IterState::Fees,
        }
    }

    pub fn values(&self) -> Values<'_> {
        Values(self.entries())
    }

    /// Sum the total values in the pots
    pub fn total_value(&self) -> Result<Value, ValueError> {
        Value::sum(self.values())
    }

    /// Append some fees in the pots
    pub fn append_fees(&mut self, fees: Value) -> Result<(), Error> {
        self.fees = (self.fees + fees).map_err(|error| Error::PotValueInvalid { error })?;
        Ok(())
    }

    /// Draw rewards from the pot
    #[must_use]
    pub fn draw_reward(&mut self, expected_reward: Value) -> Value {
        let to_draw = cmp::min(self.rewards, expected_reward);
        self.rewards = (self.rewards - to_draw).unwrap();
        to_draw
    }

    /// Draw rewards from the pot
    #[must_use]
    pub fn draw_treasury(&mut self, expected_treasury: Value) -> Value {
        self.treasury.draw(expected_treasury)
    }

    /// Siphon all the fees
    #[must_use]
    pub fn siphon_fees(&mut self) -> Value {
        let siphoned = self.fees;
        self.fees = Value::zero();
        siphoned
    }

    /// Add to treasury
    pub fn treasury_add(&mut self, value: Value) -> Result<(), Error> {
        self.treasury.add(value)
    }

    /// Add to treasury
    pub fn rewards_add(&mut self, value: Value) -> Result<(), Error> {
        self.rewards = self
            .rewards
            .checked_add(value)
            .map_err(|error| Error::PotValueInvalid { error })?;
        Ok(())
    }

    /// Get the value in the treasury
    pub fn fees_value(&self) -> Value {
        self.fees
    }

    /// Get the value in the treasury
    pub fn treasury_value(&self) -> Value {
        self.treasury.value()
    }

    pub fn set_from_entry(&mut self, e: &Entry) {
        match e {
            Entry::Fees(v) => self.fees = *v,
            Entry::Treasury(v) => self.treasury = Treasury::initial(*v),
            Entry::Rewards(v) => self.rewards = *v,
        }
    }
}

#[cfg(any(test, feature = "property-test-api"))]
mod pt {
    use super::*;
    use proptest::{arbitrary::StrategyFor, prelude::*, strategy::Map};

    type Args = (Value, Treasury, Value);

    impl Arbitrary for Pots {
        type Parameters = ();
        type Strategy = Map<StrategyFor<Args>, fn(Args) -> Self>;

        fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
            any::<Args>().prop_map(|(fees, treasury, rewards)| Pots {
                fees,
                treasury,
                rewards,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    use proptest::prop_assume;
    use quickcheck::{Arbitrary, Gen};
    use test_strategy::proptest;

    impl Arbitrary for Pots {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Pots {
                fees: Arbitrary::arbitrary(g),
                treasury: Arbitrary::arbitrary(g),
                rewards: Arbitrary::arbitrary(g),
            }
        }
    }

    #[test]
    pub fn zero_pots() {
        let pots = Pots::zero();
        assert_eq!(pots.fees, Value::zero());
        assert_eq!(pots.treasury, Treasury::initial(Value::zero()));
        assert_eq!(pots.rewards, Value::zero());
    }

    #[proptest]
    fn entries(pots: Pots) {
        for item in pots.entries() {
            match item {
                Entry::Fees(fees) => {
                    assert_eq!(pots.fees, fees);
                }
                Entry::Treasury(treasury) => {
                    assert_eq!(pots.treasury.value(), treasury);
                }
                Entry::Rewards(rewards) => {
                    assert_eq!(pots.rewards, rewards);
                }
            }
        }
    }

    #[proptest]
    fn append_fees(mut pots: Pots, value: Value) {
        prop_assume!((value + pots.fees).is_ok());

        let before = pots.fees;
        pots.append_fees(value).unwrap();
        assert_eq!((before + value).unwrap(), pots.fees);
    }

    #[proptest]
    fn siphon_fees(mut pots: Pots) {
        let before_siphon = pots.fees;
        let siphoned = pots.siphon_fees();
        if siphoned != before_siphon {
            panic!("{} is not equal to {}", siphoned, before_siphon);
        }
        assert_eq!(pots.fees, Value::zero());
    }

    #[proptest]
    fn draw_reward(mut pots: Pots, expected_reward: Value) {
        prop_assume!((expected_reward + pots.rewards).is_ok());

        let before_reward = pots.rewards;
        let to_draw = pots.draw_reward(expected_reward);
        let draw_reward = cmp::min(before_reward, expected_reward);
        if to_draw != draw_reward {
            panic!(
                "{} is not equal to smallest of pair({},{})",
                to_draw, before_reward, expected_reward
            );
        }
        assert_eq!(pots.rewards, (before_reward - to_draw).unwrap());
    }

    #[proptest]
    fn treasury_add(mut pots: Pots, value: Value) {
        prop_assume!((value + pots.treasury.value()).is_ok());
        let before_add = pots.treasury.value();
        pots.treasury_add(value).unwrap();
        assert_eq!(pots.treasury.value(), (before_add + value).unwrap());
    }
}
