pub mod seed;
pub use smoke::R;

pub fn new_random_generator() -> R {
    R::from_seed(seed::random_seed())
}
