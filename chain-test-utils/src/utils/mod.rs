pub mod seed;
pub use smoke::{Generator, R};

pub fn new_random_generator() -> R {
    R::from_seed(seed::random_seed())
}
