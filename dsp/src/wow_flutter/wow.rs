#[allow(unused_imports)]
use micromath::F32Ext as _;

use super::ornstein_uhlenbeck::OrnsteinUhlenbeck;
use super::wavefolder;
use crate::random::Random;
use crate::state_variable_filter::StateVariableFilter;
use crate::trigonometry;

// These constants were obtained through design in hack/wow.py and
// experimentation with sound.
const BASE_FREQUENCY: f32 = 0.07;
const MODULATION_CUTOFF: f32 = 1.0;
const ORNSTEIN_UHLENBECK_NOISE: f32 = 5.0;
const ORNSTEIN_UHLENBECK_SPRING: f32 = 8.0;
const PHASE_DRIFT: f32 = 0.9;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub depth: f32,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Wow {
    sample_rate: f32,
    depth: f32,
    phase: f32,
    ornstein_uhlenbeck: OrnsteinUhlenbeck,
    filter: StateVariableFilter,
}

impl Wow {
    pub fn new(sample_rate: u32) -> Self {
        assert!(
            sample_rate > 500,
            "Wow may be unstable for low sample rates"
        );
        let filter = {
            let mut filter = StateVariableFilter::new(sample_rate);
            filter.set_frequency(MODULATION_CUTOFF);
            filter
        };
        let ornstein_uhlenbeck = {
            let mut ornstein_uhlenbeck = OrnsteinUhlenbeck::new(sample_rate as f32);
            ornstein_uhlenbeck.noise = ORNSTEIN_UHLENBECK_NOISE;
            ornstein_uhlenbeck.spring = ORNSTEIN_UHLENBECK_SPRING;
            ornstein_uhlenbeck
        };
        Self {
            sample_rate: sample_rate as f32,
            depth: 0.0,
            phase: 0.5, // Start the offset sine wave on 0.0
            ornstein_uhlenbeck,
            filter,
        }
    }

    // TODO: This needs a high quality interpolation
    pub fn pop(&mut self, random: &mut impl Random) -> f32 {
        // TODO: If depth is 0.0, then target should be always 0.0. Create a bench and test it
        let target = {
            let x = (trigonometry::cos(self.phase) + 1.0) * self.depth / 2.0;

            let drift = self.ornstein_uhlenbeck.pop(random) * PHASE_DRIFT;
            self.phase += (BASE_FREQUENCY / self.sample_rate) * (1.0 + drift);
            // TODO: Try if using (while > 1.0: -= 1.0) is faster
            self.phase %= 1.0;

            x
        };
        wavefolder::fold(self.filter.tick(target).low_pass, 0.0, 1000.0)
    }

    pub fn set_attributes(&mut self, attributes: &Attributes) {
        self.depth = attributes.depth;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const SAMPLE_RATE: u32 = 1000;

    struct TestRandom;

    impl Random for TestRandom {
        fn normal(&mut self) -> f32 {
            use rand::prelude::*;
            let mut rng = rand::thread_rng();
            rng.gen()
        }
    }

    #[test]
    fn it_spans_in_expected_range() {
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.set_attributes(&Attributes { depth: 1.0 });

        let x = wow.pop(&mut TestRandom);
        let (mut min, mut max) = (x, x);

        for _ in 0..SAMPLE_RATE * (1.0 / BASE_FREQUENCY) as u32 {
            let x = wow.pop(&mut TestRandom);
            if x < min {
                min = x;
            }
            if x > max {
                max = x;
            }
        }

        assert_relative_eq!(min, 0.0, epsilon = 0.01);
        assert_relative_eq!(max, 1.0, epsilon = 0.01);
    }

    #[test]
    fn it_starts_near_zero() {
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.set_attributes(&Attributes { depth: 1.0 });

        let x = wow.pop(&mut TestRandom);
        assert!(x >= 0.0);
        assert_relative_eq!(x, 0.0);
    }

    proptest! {
        #[test]
        fn it_never_falls_bellow_zero(
            depth in 0.0f32..10.0,
        ) {
            let mut wow = Wow::new(SAMPLE_RATE);
            wow.set_attributes(&Attributes { depth });

            for _ in 0..SAMPLE_RATE * (1.0 / BASE_FREQUENCY) as u32 {
                assert!(wow.pop(&mut TestRandom) >= 0.0);
            }
        }

        #[test]
        fn it_never_exceeds_given_depth(
            depth in 0.0f32..10.0,
        ) {
            let mut wow = Wow::new(SAMPLE_RATE);
            wow.set_attributes(&Attributes { depth });

            for _ in 0..SAMPLE_RATE * (1.0 / BASE_FREQUENCY) as u32 {
                assert!(wow.pop(&mut TestRandom) <= depth);
            }
        }
    }
}
