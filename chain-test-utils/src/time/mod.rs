use chain_time::{Epoch, Slot, TimeEra};
use smoke::{Generator, NumPrimitive, R};

struct TimeEraGen {
    r: R,
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

pub fn generate_slot(r: &mut R) -> Slot {
    _generate_slot(|| r.num())
}

pub fn generate_time_era(r: &mut R) -> TimeEra {
    TimeEra::new(generate_slot(r), generate_epoch(r), r.num())
}
