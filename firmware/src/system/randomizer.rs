use daisy::hal::rng::{Rng, RngCore};

use kaseta_dsp::random::Random;

pub struct Randomizer {
    pub rng: Rng,
}

impl Randomizer {
    #[must_use]
    pub fn new(rng: Rng) -> Self {
        Self { rng }
    }
}

impl Random for Randomizer {
    #[allow(clippy::cast_lossless)]
    fn normal(&mut self) -> f32 {
        RngCore::<u16>::gen(&mut self.rng).unwrap() as f32 / (2 << 15) as f32
    }
}
