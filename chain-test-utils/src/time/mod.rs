use chain_time::{Epoch, Slot, TimeEra};
use smoke::{Generator, R};

/// `TimeEra` configuration, encapsulates the building boundaries for the inner data
#[derive(Clone)]
pub struct TimeEraGenConfig {
    pub slot_range: (u64, u64),
    pub epoch_range: (u32, u32),
    pub slots_per_epoch_range: (u32, u32),
}

/// Generator wrapper for TimeEra generator methods
/// It can generate `TimeEra` values both randomized or configuration based.
/// The configuration can be change dynamically in runtime to change its behaviour with the method.
/// `TimeGenerator::set_config`.
/// This `TimeEraGenerator` implements `smoke::Generator`. It has publicly available a `gen` method
/// (`fn gen(&self, r: &mut R) -> TimeEra`) which is the core functionality of the type.
/// For example, we can generate a bunch of `TimeEra` like the following:
/// ```
/// use chain_test_utils::time::TimeEraGenerator;
/// use chain_test_utils::utils::Generator;
/// use chain_time::TimeEra;
/// let mut r = chain_test_utils::utils::new_random_generator();
/// let time_era_generator = TimeEraGenerator::default();
/// let time_eras : Vec<TimeEra> = (1..10).map(|_| time_era_generator.gen(&mut r)).collect();
/// ```
pub struct TimeEraGenerator {
    config: Option<TimeEraGenConfig>,
}

/// Generate an `Epoch` from a generator function
/// This function generates an Epoch value taking the value from the generator function it receives as parameter
/// The method assumes that the values generated from the generator function are valid for the scope
fn generate_epoch_with<GenF>(mut gen: GenF) -> Epoch
where
    GenF: FnMut() -> u32,
{
    Epoch(gen())
}

/// Generate an `Slot` from a generator function
/// This method generates an Slot value taking the value from the generator function it receives as parameter
/// The method assumes that the values generated from the generator function are valid for the scope
fn generate_slot_with<GenF>(mut gen: GenF) -> Slot
where
    GenF: FnMut() -> u64,
{
    gen().into()
}

/// Generate an `Epoch` given a `smoke::R` (random generator)
/// This method generates a completely random Epoch from a random generator.
pub fn generate_epoch(r: &mut R) -> Epoch {
    generate_epoch_with(|| r.num())
}

/// Generate an `Epoch` given a `smoke::R` (random generator) and a `(u32, u32)` range limit tuple
/// The range is assumed to be close start and close end `[start, end]`
pub fn generate_epoch_with_range(r: &mut R, range: (u32, u32)) -> Epoch {
    generate_epoch_with(|| r.num_range(range.0, range.1))
}

/// Generate an `Slot` given a `smoke::R` (random generator)
/// This method generates a completely random Slot from a random generator.
pub fn generate_slot(r: &mut R) -> Slot {
    generate_slot_with(|| r.num())
}

/// Generate an `Slot` given a `smoke::R` (random generator) and a `(u32, u32)` range limit tuple
/// The range is assumed to be close start and close end `[start, end]`
pub fn generate_slot_with_range(r: &mut R, range: (u64, u64)) -> Slot {
    generate_slot_with(|| r.num_range(range.0, range.1))
}

/// Generate an `TimeEra` given a `smoke::R` (random generator)
/// This method generates a completely random TimeEra. The generated `TimeEra` may not be a valid one.
pub fn generate_time_era(r: &mut R) -> TimeEra {
    TimeEra::new(generate_slot(r), generate_epoch(r), r.num())
}

/// Generate an `TimeEra` given a `smoke::R` (random generator) and a `TimeEraGenConfig` range limit tuple
pub fn generate_time_era_with_config(r: &mut R, config: &TimeEraGenConfig) -> TimeEra {
    TimeEra::new(
        generate_slot_with_range(r, config.slot_range),
        generate_epoch_with_range(r, config.epoch_range),
        r.num_range(
            config.slots_per_epoch_range.0,
            config.slots_per_epoch_range.1,
        ),
    )
}

impl TimeEraGenerator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: TimeEraGenConfig) -> Self {
        Self {
            config: Some(config),
        }
    }

    pub fn set_config(&mut self, config: TimeEraGenConfig) {
        self.config = Some(config)
    }

    pub fn clear_config(&mut self) {
        self.config = None;
    }
}

impl Default for TimeEraGenerator {
    fn default() -> Self {
        Self { config: None }
    }
}

impl Generator for TimeEraGenerator {
    type Item = TimeEra;

    fn gen(&self, r: &mut R) -> Self::Item {
        match &self.config {
            Some(config) => generate_time_era_with_config(r, config),
            None => generate_time_era(r),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::time::{TimeEraGenConfig, TimeEraGenerator};
    use crate::utils::new_random_generator;

    #[test]
    fn generate_epoch() {
        let epoch_value = 10;
        let epoch = generate_epoch_with(|| epoch_value);
        assert_eq!(epoch.0, epoch_value);
    }

    #[test]
    fn generate_slot() {
        let slot_value = 10;
        let slot = generate_slot_with(|| slot_value);
        assert_eq!(Into::<u64>::into(slot), slot_value);
    }

    #[test]
    fn generate_time_era() {
        let slot_range = (1, 10);
        let epoch_range = (1, 10);
        let slots_per_epoch_range = (1, 10);
        let config = TimeEraGenConfig {
            slot_range,
            epoch_range,
            slots_per_epoch_range,
        };
        let time_era_generator = TimeEraGenerator::with_config(config.clone());
        let mut r = new_random_generator();
        let new_time_era = time_era_generator.gen(&mut r);
        assert!((1..=10).contains(&new_time_era.slots_per_epoch()));
    }
}
