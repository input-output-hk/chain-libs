use rand::random;
use smoke::Seed;

pub fn random_seed() -> Seed {
    let f: u128 = random();
    Seed::from(f)
}
