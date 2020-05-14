use crate::generators::utils::{Generator, R};

#[derive(Clone)]
pub struct ConstantGenerator<T>(T);

impl<T: Clone> ConstantGenerator<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T: Clone> Generator for ConstantGenerator<T> {
    type Item = T;

    fn gen(&self, _: &mut R) -> Self::Item {
        self.0.clone()
    }
}
