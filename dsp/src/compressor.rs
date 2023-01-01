//! Dynamic range compressor.
//!
//! Based on the README of https://github.com/p-hlp/CTAGDRC.

// TODO: For stereo, analyzing max on both sides, applying equally

use libm::{expf, fabsf, log10f, powf};

const ATTACK_IN_SECONDS: f32 = 0.01;
const RELEASE_IN_SECONDS: f32 = 0.14;

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Compressor {
    n1: f32,
    alpha_attack: f32,
    alpha_release: f32,
}

// TODO:
// - Convert to maximum sizes of amplitudes from both channels
// - Convert to dbs
// - Apply knee
// - Apply smoothing (envelope)
// - Convert back to linear
// - Multiply the original signal by this
impl Compressor {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            n1: 0.0,
            alpha_attack: expf(-1.0 / (sample_rate * ATTACK_IN_SECONDS)),
            alpha_release: expf(-1.0 / (sample_rate * RELEASE_IN_SECONDS)),
        }
    }

    pub fn process(&mut self, buffer_left: &mut [f32], buffer_right: &mut [f32]) {
        // TODO: Try optimizing this by processing each stage in a buffer

        for (l, r) in buffer_left.iter_mut().zip(buffer_right) {
            let max = f32::max(fabsf(*l), fabsf(*r));
            let level = f32::max(max, 1.0e-6);
            let level_in_decibels = 20.0 * log10f(level);

            // TODO: Tweak this to keep good headroom
            // NOTE: This is 0.5 amplitude
            let treshold = -12.0;
            // TODO: Tweak ratio to make sure loud feedback can fit easily
            let ratio = 16.0;
            let slope = 1.0 / ratio - 1.0;
            let knee = 6.0;
            let knee_half = knee / 2.0;

            let overshoot = level_in_decibels - treshold;
            let compressed = if overshoot < -knee_half {
                0.0
            } else if overshoot < knee_half {
                0.5 * slope * ((overshoot + knee_half) * (overshoot + knee_half)) / knee
            } else {
                slope * overshoot
            };

            self.n1 = if compressed < self.n1 {
                self.alpha_attack * self.n1 + (1.0 - self.alpha_attack) * compressed
            } else {
                self.alpha_release * self.n1 + (1.0 - self.alpha_release) * compressed
            };
            let filtered = self.n1;

            // TODO: Consider applying makeup

            let level = powf(10.0, filtered / 20.0);

            *l *= level;
            *r *= level;
        }
    }
}
