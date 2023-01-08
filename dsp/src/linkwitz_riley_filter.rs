//! Linkwitz-Riley filter.
//!
//! Meant for audio crossover. Based on <https://github.com/jatinchowdhury18/AnalogTapeModel/blob/master/Plugin/Source/Processors/Input_Filters/LinkwitzRileyFilter.h>.

#[allow(unused_imports)]
use micromath::F32Ext as _;

use core::f32::consts::{PI, SQRT_2};

/// Yields filtered signal.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug)]
pub struct LinkwitzRileyFilter {
    sample_rate: f32,
    g: f32,
    h: f32,
    s0: f32,
    s1: f32,
    s2: f32,
    s3: f32,
}

impl LinkwitzRileyFilter {
    pub fn new(sample_rate: f32) -> Self {
        let mut filter = Self {
            sample_rate,
            g: 0.0,
            h: 0.0,
            s0: 0.0,
            s1: 0.0,
            s2: 0.0,
            s3: 0.0,
        };
        filter.set_frequency(0.0);
        filter
    }

    pub fn set_frequency(&mut self, frequency: f32) -> &mut Self {
        assert!(frequency.is_sign_positive() && frequency < self.sample_rate * 0.5);
        self.g = f32::tan(PI * frequency / self.sample_rate);
        self.h = 1.0 / (1.0 + SQRT_2 * self.g + self.g * self.g);
        self
    }

    pub fn tick(&mut self, x: f32) -> Signal {
        let y_h = (x - (SQRT_2 + self.g) * self.s0 - self.s1) * self.h;

        let t_b = self.g * y_h;
        let y_b = t_b + self.s0;
        self.s0 = t_b + y_b;

        let t_l = self.g * y_b;
        let y_l = t_l + self.s1;
        self.s1 = t_l + y_l;

        let y_h2 = (y_l - (SQRT_2 + self.g) * self.s2 - self.s3) * self.h;

        let t_b2 = self.g * y_h2;
        let y_b2 = t_b2 + self.s2;
        self.s2 = t_b2 + y_b2;

        let t_l2 = self.g * y_b2;
        let y_l2 = t_l2 + self.s3;
        self.s3 = t_l2 + y_l2;

        Signal {
            low_pass: y_l2,
            high_pass: y_l - SQRT_2 * y_b + y_h - y_l2,
        }
    }
}

/// Filtered signal.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Default)]
pub struct Signal {
    pub low_pass: f32,
    pub high_pass: f32,
}
