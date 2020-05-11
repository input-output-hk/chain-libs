pub mod seed;
pub use smoke::R;

#[allow(non_snake_case)] // Allow because R is a type name, the function is snake case anyway
pub fn new_R_from_random_seed() -> R {
    R::from_seed(seed::random_seed())
}
