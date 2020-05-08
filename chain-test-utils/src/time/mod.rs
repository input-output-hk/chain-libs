use chain_time::{Epoch, Slot, TimeEra};
use smoke::{Generator, NumPrimitive, R};

// struct TimeEraGen {
//     r: R,
// }

pub struct TimeEraGenCfg {
    pub slot_rng: (u64, u64),
    pub epoch_rng: (u32, u32),
    pub slots_per_epoch_rng: (u32, u32),
}

fn _generate_epoch<GenF>(mut gen: GenF) -> Epoch
where
    GenF: FnMut() -> u32,
{
    Epoch(gen())
}

fn _generate_slot<GenF>(mut gen: GenF) -> Slot
where
    GenF: FnMut() -> u64,
{
    gen().into()
}

pub fn generate_epoch(r: &mut R) -> Epoch {
    _generate_epoch(|| r.num())
}

pub fn generate_epoch_with_range(r: &mut R, range: (u32, u32)) -> Epoch {
    _generate_epoch(|| r.num_range(range.0, range.1))
}

pub fn generate_slot(r: &mut R) -> Slot {
    _generate_slot(|| r.num())
}

pub fn generate_slot_with_range(r: &mut R, range: (u64, u64)) -> Slot {
    _generate_slot(|| r.num_range(range.0, range.1))
}

pub fn generate_time_era(r: &mut R) -> TimeEra {
    TimeEra::new(generate_slot(r), generate_epoch(r), r.num())
}

pub fn generate_time_era_with_config(r: &mut R, config: TimeEraGenCfg) -> TimeEra {
    TimeEra::new(
        generate_slot_with_range(r, config.slot_rng),
        generate_epoch_with_range(r, config.epoch_rng),
        r.num_range(config.slots_per_epoch_rng.0, config.slots_per_epoch_rng.1),
    )
}

#[cfg(test)]
mod test {}
