use core::f32::consts::PI;

use crate::random::Random;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Wow {
    sample_rate: u32,
    phase: f32,
    pub frequency: f32,
    pub depth: f32,
    pub amplitude_noise: f32,
}

impl Wow {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            phase: 0.5,
            frequency: 0.0,
            depth: 0.0,
            amplitude_noise: 0.0,
        }
    }

    pub fn pop(&mut self, random: &mut impl Random) -> f32 {
        let mut x = (libm::cosf(self.phase * 2.0 * PI) + 1.0) * self.depth / 2.0;
        x += random.normal() * self.amplitude_noise;
        self.phase += self.frequency / self.sample_rate as f32;
        self.phase %= 1.0;
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRandom;

    impl Random for TestRandom {
        fn normal(&mut self) -> f32 {
            0.0
        }
    }

    #[test]
    fn it_spans_in_expected_range() {
        const SAMPLE_RATE: u32 = 100;
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.frequency = 1.0;
        wow.depth = 1.0;

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
        const SAMPLE_RATE: u32 = 100;
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.frequency = 1.0;
        wow.depth = 1.0;

        let x = wow.pop(&mut TestRandom);
        assert!(x >= 0.0);
        assert_relative_eq!(x, 0.0);
    }

    #[test]
    fn it_cycles_in_expected_interval() {
        const SAMPLE_RATE: u32 = 100;
        let mut wow = Wow::new(SAMPLE_RATE);
        wow.frequency = 1.0;
        wow.depth = 1.0;

        for _ in 0..SAMPLE_RATE / 2 {
            assert!(wow.pop(&mut TestRandom) < 0.9999);
        }
        assert_relative_eq!(wow.pop(&mut TestRandom), 1.0);
        for _ in 0..SAMPLE_RATE / 2 - 1 {
            assert!(wow.pop(&mut TestRandom) < 0.9999);
        }
        assert_relative_eq!(wow.pop(&mut TestRandom), 0.0);
    }
}
