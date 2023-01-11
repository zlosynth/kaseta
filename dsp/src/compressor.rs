//! Dynamic range compressor.
//!
//! Based on the README of <https://github.com/p-hlp/CTAGDRC>.

use libm::{expf, fabsf, log10f, powf};

const ATTACK_IN_SECONDS: f32 = 0.01;
const RELEASE_IN_SECONDS: f32 = 0.14;
const TRESHOLD: f32 = -6.0; // This is 0.5 amplitude
const RATIO: f32 = 16.0;
const SLOPE: f32 = 1.0 / RATIO - 1.0;
const KNEE: f32 = 6.0;
const KNEE_HALF: f32 = KNEE / 2.0;

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Compressor {
    n1: f32,
    alpha_attack: f32,
    alpha_release: f32,
}

impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            n1: 0.0,
            alpha_attack: expf(-1.0 / (sample_rate * ATTACK_IN_SECONDS)),
            alpha_release: expf(-1.0 / (sample_rate * RELEASE_IN_SECONDS)),
        }
    }

    pub fn process(&mut self, buffer_left: &mut [f32], buffer_right: &mut [f32]) {
        for (l, r) in buffer_left.iter_mut().zip(buffer_right) {
            let max = f32::max(fabsf(*l), fabsf(*r));
            let level = f32::max(max, 1.0e-6);
            let level_in_decibels = 20.0 * log10f(level);

            let overshoot = level_in_decibels - TRESHOLD;
            let compression = if overshoot < -KNEE_HALF {
                0.0
            } else if overshoot < KNEE_HALF {
                0.5 * SLOPE * ((overshoot + KNEE_HALF) * (overshoot + KNEE_HALF)) / KNEE
            } else {
                SLOPE * overshoot
            };

            let filtered_compression = if compression < self.n1 {
                self.alpha_attack * self.n1 + (1.0 - self.alpha_attack) * compression
            } else {
                self.alpha_release * self.n1 + (1.0 - self.alpha_release) * compression
            };
            self.n1 = filtered_compression;
            let filtered_compression_linear = powf(10.0, filtered_compression / 20.0);

            *l *= filtered_compression_linear;
            *r *= filtered_compression_linear;
        }
    }
}
