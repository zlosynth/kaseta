use core::f32::consts::PI;

use super::ornstein_uhlenbeck::OrnsteinUhlenbeck;
use crate::random::Random;

// TODO: Test that with any attributes, the output does not go beyond depth
// TODO: Test that with any attributes, the output does not go to negative numbers

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub frequency: f32,
    pub depth: f32,
    pub amplitude_noise: f32,
    pub amplitude_spring: f32,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Wow {
    sample_rate: f32,
    frequency: f32,
    depth: f32,
    phase: f32,
    amplitude_ou: OrnsteinUhlenbeck,
}

impl Wow {
    pub fn new(sample_rate: u32) -> Self {
        debug_assert!(
            sample_rate > 500,
            "Wow may be unstable for low sample rates"
        );
        Self {
            sample_rate: sample_rate as f32,
            frequency: 0.0,
            depth: 0.0,
            phase: 0.5, // Start the offset sine wave on 0.0
            amplitude_ou: OrnsteinUhlenbeck::new(sample_rate as f32),
        }
    }

    pub fn pop(&mut self, random: &mut impl Random) -> f32 {
        let target = (libm::cosf(self.phase * 2.0 * PI) + 1.0) * self.depth / 2.0;
        let x = self.amplitude_ou.pop(target, random);
        self.phase += self.frequency / self.sample_rate;
        // TODO: Try if using (while > 1.0: -= 1.0) is faster
        self.phase %= 1.0;
        x
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.frequency = attributes.frequency;
        self.depth = attributes.depth;
        self.amplitude_ou.noise = attributes.amplitude_noise;
        self.amplitude_ou.spring = attributes.amplitude_spring;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct TestRandom;

    impl Random for TestRandom {
        fn normal(&mut self) -> f32 {
            0.0
        }
    }

    #[test]
    fn it_spans_in_expected_range() {
        const SAMPLE_RATE: u32 = 1000;
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.set_attributes(Attributes {
            frequency: 1.0,
            depth: 1.0,
            amplitude_noise: 0.0,
            amplitude_spring: 1000.0,
        });

        let x = wow.pop(&mut TestRandom);
        let (mut min, mut max) = (x, x);

        for _ in 0..SAMPLE_RATE {
            let x = wow.pop(&mut TestRandom);
            if x < min {
                min = x;
            }
            if x > max {
                max = x;
            }
        }

        assert_relative_eq!(min, 0.0);
        assert_relative_eq!(max, 1.0);
    }

    #[test]
    fn it_starts_near_zero() {
        const SAMPLE_RATE: u32 = 1000;
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.set_attributes(Attributes {
            frequency: 1.0,
            depth: 1.0,
            amplitude_noise: 0.0,
            amplitude_spring: 1000.0,
        });

        let x = wow.pop(&mut TestRandom);
        assert!(x >= 0.0);
        assert_relative_eq!(x, 0.0);
    }

    #[test]
    fn it_cycles_in_expected_interval() {
        const SAMPLE_RATE: u32 = 1000;
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.set_attributes(Attributes {
            frequency: 1.0,
            depth: 1.0,
            amplitude_noise: 0.0,
            amplitude_spring: 1000.0,
        });

        for _ in 0..SAMPLE_RATE / 2 {
            assert!(wow.pop(&mut TestRandom) < 1.0,);
        }
        assert_relative_eq!(wow.pop(&mut TestRandom), 1.0);
        for _ in 0..SAMPLE_RATE / 2 - 1 {
            assert!(wow.pop(&mut TestRandom) < 1.0);
        }
        assert_relative_eq!(wow.pop(&mut TestRandom), 0.0);
    }

    proptest! {
        #[test]
        fn it_never_falls_bellow_zero(
            frequency in 0.0f32..499.0,
            depth in 0.0f32..10.0,
            amplitude_noise in 0.0f32..5.0,
            amplitude_spring in 0.0f32..1000.0,
        ) {
            const SAMPLE_RATE: u32 = 1000;
            let mut wow = Wow::new(SAMPLE_RATE);
            wow.set_attributes(Attributes {
                frequency,
                depth,
                amplitude_noise,
                amplitude_spring,
            });

            for _ in 0..SAMPLE_RATE {
                assert!(wow.pop(&mut TestRandom) >= 0.0);
            }
        }

        #[test]
        fn it_never_exceeds_given_depth(
            frequency in 0.0f32..499.0,
            depth in 0.0f32..10.0,
            amplitude_noise in 0.0f32..5.0,
            amplitude_spring in 0.0f32..1000.0,
        ) {
            const SAMPLE_RATE: u32 = 1000;
            let mut wow = Wow::new(SAMPLE_RATE);
            wow.set_attributes(Attributes {
                frequency,
                depth,
                amplitude_noise,
                amplitude_spring,
            });

            for _ in 0..SAMPLE_RATE {
                assert!(wow.pop(&mut TestRandom) <= depth);
            }
        }
    }
}
