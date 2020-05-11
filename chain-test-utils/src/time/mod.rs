use chain_time::{Epoch, Slot, TimeEra};
use smoke::{Generator, R};

// Generate an `Epoch` from a generator function
fn _generate_epoch<GenF>(mut gen: GenF) -> Epoch
where
    GenF: FnMut() -> u32,
{
    Epoch(gen())
}

// Generate an `Slot` from a generator function
fn _generate_slot<GenF>(mut gen: GenF) -> Slot
where
    GenF: FnMut() -> u64,
{
    gen().into()
}

// Generate an `Epoch` given a `smoke::R` (random generator)
pub fn generate_epoch(r: &mut R) -> Epoch {
    _generate_epoch(|| r.num())
}

// Generate an `Epoch` given a `smoke::R` (random generator) and a `(u32, u32)` range limit tuple
pub fn generate_epoch_with_range(r: &mut R, range: (u32, u32)) -> Epoch {
    _generate_epoch(|| r.num_range(range.0, range.1))
}

// Generate an `Slot` given a `smoke::R` (random generator)
pub fn generate_slot(r: &mut R) -> Slot {
    _generate_slot(|| r.num())
}

// Generate an `Slot` given a `smoke::R` (random generator) and a `(u32, u32)` range limit tuple
pub fn generate_slot_with_range(r: &mut R, range: (u64, u64)) -> Slot {
    _generate_slot(|| r.num_range(range.0, range.1))
}

// Generate an `TimeEra` given a `smoke::R` (random generator)
pub fn generate_time_era(r: &mut R) -> TimeEra {
    TimeEra::new(generate_slot(r), generate_epoch(r), r.num())
}

// `TimeEra` configuration, encapsulates the building boundaries for the inner data
#[derive(Clone)]
pub struct TimeEraGenCfg {
    pub slot_range: (u64, u64),
    pub epoch_range: (u32, u32),
    pub slots_per_epoch_range: (u32, u32),
}

// Generate an `TimeEra` given a `smoke::R` (random generator) and a `TimeEraGenCfg` range limit tuple
pub fn generate_time_era_with_config(r: &mut R, config: &TimeEraGenCfg) -> TimeEra {
    TimeEra::new(
        generate_slot_with_range(r, config.slot_range),
        generate_epoch_with_range(r, config.epoch_range),
        r.num_range(
            config.slots_per_epoch_range.0,
            config.slots_per_epoch_range.1,
        ),
    )
}

// Generator wrapper for TimeEra generator methods
pub struct TimeEraGenerator {
    config: Option<TimeEraGenCfg>,
}

impl TimeEraGenerator {
    pub fn new(config: Option<TimeEraGenCfg>) -> Self {
        Self { config }
    }

    pub fn with_config(&mut self, config: TimeEraGenCfg) {
        self.config = Some(config);
    }

    pub fn clear_config(&mut self) {
        self.config = None;
    }
}

impl Default for TimeEraGenerator {
    fn default() -> Self {
        Self::new(None)
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
    use crate::time::{TimeEraGenCfg, TimeEraGenerator};
    use crate::utils::new_R_from_random_seed;

    #[test]
    fn generate_epoch() {
        let epoch_value = 10;
        let epoch = _generate_epoch(|| epoch_value);
        assert_eq!(epoch.0, epoch_value);
    }

    #[test]
    fn generate_slot() {
        let slot_value = 10;
        let slot = _generate_slot(|| slot_value);
        assert_eq!(Into::<u64>::into(slot), slot_value);
    }

    #[test]
    fn generate_time_era() {
        let slot_range = (1, 10);
        let epoch_range = (1, 10);
        let slots_per_epoch_range = (1, 10);
        let config = TimeEraGenCfg {
            slot_range,
            epoch_range,
            slots_per_epoch_range,
        };
        let time_era_generator = TimeEraGenerator::new(Some(config.clone()));
        let mut r = new_R_from_random_seed();
        let new_time_era = time_era_generator.gen(&mut r);
        assert!((1..10).contains(&new_time_era.slots_per_epoch()));
    }
}
