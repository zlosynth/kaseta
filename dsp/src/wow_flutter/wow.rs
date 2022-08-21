use core::f32::consts::PI;

const FREQ: f32 = 1.0;
const DEPTH: f32 = 1.0;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Wow {
    sample_rate: u32,
    phase: f32,
}

impl Wow {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
        }
    }

    pub fn pop(&mut self) -> f32 {
        let x = (libm::cosf(self.phase * 2.0 * PI) - 1.0) * DEPTH / 2.0;
        self.phase += FREQ / self.sample_rate as f32;
        self.phase %= 1.0;
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_spans_in_expected_range() {
        const SAMPLE_RATE: u32 = 100;
        let mut wow = Wow::new(SAMPLE_RATE);

        let x = wow.pop();
        let (mut min, mut max) = (x, x);

        for _ in 0..SAMPLE_RATE {
            let x = wow.pop();
            if x < min {
                min = x;
            }
            if x > max {
                max = x;
            }
        }

        assert_relative_eq!(min, -DEPTH);
        assert_relative_eq!(max, 0.0);
    }

    #[test]
    fn it_starts_near_zero() {
        const SAMPLE_RATE: u32 = 100;
        let mut wow = Wow::new(SAMPLE_RATE);

        let x = wow.pop();
        assert!(x <= 0.0);
        assert_relative_eq!(x, 0.0);
    }

    #[test]
    fn it_cycles_in_expected_interval() {
        const SAMPLE_RATE: u32 = 100;
        let mut wow = Wow::new(SAMPLE_RATE);

        for _ in 0..SAMPLE_RATE / 2 {
            assert!(wow.pop() > -0.9999);
        }
        assert_relative_eq!(wow.pop(), -DEPTH);
        for _ in 0..SAMPLE_RATE / 2 - 1 {
            assert!(wow.pop() > -0.9999);
        }
        assert_relative_eq!(wow.pop(), 0.0);
    }
}
