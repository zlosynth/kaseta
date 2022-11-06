use core::f32::consts::PI;

#[allow(unused_imports)]
use micromath::F32Ext as _;

const BASE_FREQUENCY: f32 = 8.0;
const SECOND_FREQUENCY: f32 = BASE_FREQUENCY * 1.718_712_4;
const THIRD_FREQUENCY: f32 = BASE_FREQUENCY * 2.834_324_1;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub depth: f32,
}

// TODO: Based on intensity, start with pulsing, then enable fully.
// TODO: On the very end it would start getting deeper
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Flutter {
    sample_rate: f32,
    depth: f32,
    phase_base: f32,
    phase_second: f32,
    phase_third: f32,
}

impl Flutter {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            depth: 0.0,
            phase_base: 0.5, // Start the offset sine wave on 0.0
            phase_second: 0.5,
            phase_third: 0.5,
        }
    }

    pub fn pop(&mut self) -> f32 {
        let x1 = libm::cosf(self.phase_base * 2.0 * PI) + 1.0;
        let x2 = libm::cosf(self.phase_second * 2.0 * PI) + 1.0;
        let x3 = libm::cosf(self.phase_third * 2.0 * PI) + 1.0;
        let x = ((x1 + x2 + x3) / 3.0) * self.depth / 2.0;
        self.phase_base += BASE_FREQUENCY / self.sample_rate;
        self.phase_second += SECOND_FREQUENCY / self.sample_rate;
        self.phase_third += THIRD_FREQUENCY / self.sample_rate;
        // TODO: Try if using (while > 1.0: -= 1.0) is faster
        self.phase_base %= 1.0;
        self.phase_second %= 1.0;
        self.phase_third %= 1.0;
        x
    }

    pub fn set_attributes(&mut self, attributes: &Attributes) {
        self.depth = attributes.depth;
    }
}
