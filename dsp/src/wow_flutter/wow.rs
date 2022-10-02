use core::f32::consts::PI;

#[allow(unused_imports)]
use micromath::F32Ext as _;

use super::ornstein_uhlenbeck::OrnsteinUhlenbeck;
use super::wavefolder;
use crate::random::Random;
use sirena::state_variable_filter::{Bandform, StateVariableFilter};

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub frequency: f32,
    pub depth: f32,
    pub filter: f32,
    pub amplitude_noise: f32,
    pub amplitude_spring: f32,
    pub phase_noise: f32,
    pub phase_spring: f32,
    pub phase_drift: f32,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Wow {
    sample_rate: f32,
    frequency: f32,
    depth: f32,
    phase: f32,
    amplitude_ou: OrnsteinUhlenbeck,
    phase_ou: OrnsteinUhlenbeck,
    phase_drift: f32,
    filter: StateVariableFilter,
}

impl Wow {
    pub fn new(sample_rate: u32) -> Self {
        assert!(
            sample_rate > 500,
            "Wow may be unstable for low sample rates"
        );
        let mut filter = StateVariableFilter::new(sample_rate);
        filter.set_bandform(Bandform::LowPass);
        Self {
            sample_rate: sample_rate as f32,
            frequency: 0.0,
            depth: 0.0,
            phase: 0.5, // Start the offset sine wave on 0.0
            amplitude_ou: OrnsteinUhlenbeck::new(sample_rate as f32),
            phase_ou: OrnsteinUhlenbeck::new(sample_rate as f32),
            phase_drift: 0.0,
            filter,
        }
    }

    pub fn pop(&mut self, random: &mut impl Random) -> f32 {
        let target = {
            let x = (libm::cosf(self.phase * 2.0 * PI) + 1.0) * self.depth / 2.0;
            let drift = self.phase_ou.pop(0.0, random) * self.phase_drift;
            self.phase += (self.frequency / self.sample_rate) * (1.0 + drift);
            // TODO: Try if using (while > 1.0: -= 1.0) is faster
            self.phase %= 1.0;
            x
        };
        // TODO: Move wavefolder inside phase_ou itself, so its internal value
        // cannot wander too far off
        f32::clamp(
            self.filter.tick(wavefolder::fold(
                self.amplitude_ou.pop(target, random),
                0.0,
                self.depth,
            )),
            0.0,
            self.depth,
        )
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.frequency = attributes.frequency;
        self.depth = attributes.depth;
        self.amplitude_ou.noise = attributes.amplitude_noise;
        self.amplitude_ou.spring = attributes.amplitude_spring;
        self.filter
            .set_frequency(f32::min(attributes.filter, 0.2 * self.sample_rate));
        self.phase_ou.noise = attributes.phase_noise;
        self.phase_ou.spring = attributes.phase_spring;
        self.phase_drift = attributes.phase_drift;
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
        wow.set_attributes(Attributes {
            frequency: 1.0,
            depth: 1.0,
            filter: SAMPLE_RATE as f32 * 0.1,
            amplitude_noise: 0.0,
            amplitude_spring: 1000.0,
            ..Attributes::default()
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

        assert_relative_eq!(min, 0.0, epsilon = 0.001);
        assert_relative_eq!(max, 1.0, epsilon = 0.001);
    }

    #[test]
    fn it_starts_near_zero() {
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.set_attributes(Attributes {
            frequency: 1.0,
            depth: 1.0,
            filter: SAMPLE_RATE as f32 * 0.1,
            amplitude_noise: 0.0,
            amplitude_spring: 1000.0,
            ..Attributes::default()
        });

        let x = wow.pop(&mut TestRandom);
        assert!(x >= 0.0);
        assert_relative_eq!(x, 0.0);
    }

    #[test]
    fn it_cycles_in_expected_interval() {
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.set_attributes(Attributes {
            frequency: 1.0,
            depth: 1.0,
            filter: SAMPLE_RATE as f32 * 0.1,
            amplitude_noise: 0.0,
            amplitude_spring: 1000.0,
            ..Attributes::default()
        });

        for _ in 0..SAMPLE_RATE / 2 {
            assert!(wow.pop(&mut TestRandom) < 1.0);
        }
        assert_relative_eq!(wow.pop(&mut TestRandom), 1.0, epsilon = 0.001);
        for _ in 0..SAMPLE_RATE / 2 - 1 {
            assert!(wow.pop(&mut TestRandom) < 1.0);
        }
        assert_relative_eq!(wow.pop(&mut TestRandom), 0.0, epsilon = 0.001);
    }

    proptest! {
        #[test]
        fn it_never_falls_bellow_zero(
            frequency in 0.0f32..499.0,
            depth in 0.0f32..10.0,
            filter in 0.0..SAMPLE_RATE as f32 * 0.2,
            amplitude_noise in 0.0f32..5.0,
            amplitude_spring in 0.0f32..1000.0,
            phase_noise in 0.0f32..5.0,
            phase_spring in 0.0f32..1000.0,
            phase_drift in 0.0f32..10.0,
        ) {
            let mut wow = Wow::new(SAMPLE_RATE);
            wow.set_attributes(Attributes {
                frequency,
                depth,
                filter,
                amplitude_noise,
                amplitude_spring,
                phase_noise,
                phase_spring,
                phase_drift,
            });

            for _ in 0..SAMPLE_RATE {
                assert!(wow.pop(&mut TestRandom) >= 0.0);
            }
        }

        #[test]
        fn it_never_exceeds_given_depth(
            frequency in 0.0f32..499.0,
            depth in 0.0f32..10.0,
            filter in 0.0..SAMPLE_RATE as f32 * 0.2,
            amplitude_noise in 0.0f32..5.0,
            amplitude_spring in 0.0f32..1000.0,
            phase_noise in 0.0f32..5.0,
            phase_spring in 0.0f32..1000.0,
            phase_drift in 0.0f32..10.0,
        ) {
            let mut wow = Wow::new(SAMPLE_RATE);
            wow.set_attributes(Attributes {
                frequency,
                depth,
                filter,
                amplitude_noise,
                amplitude_spring,
                phase_noise,
                phase_spring,
                phase_drift,
            });

            for _ in 0..SAMPLE_RATE {
                assert!(wow.pop(&mut TestRandom) <= depth);
            }
        }
    }
}
