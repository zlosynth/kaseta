#[allow(unused_imports)]
use micromath::F32Ext;

use crate::state_variable_filter::StateVariableFilter;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub tone: f32,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Tone {
    position: f32,
    filter: StateVariableFilter,
}

impl Tone {
    /// # Panics
    ///
    /// Sample rate must be higher than 500, otherwise the filter becomes
    /// unstable and the initialization will panic.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        assert!(
            sample_rate > 500,
            "Tone may be unstable in slow sample rates"
        );
        Self {
            position: 0.0,
            filter: StateVariableFilter::new(sample_rate),
        }
    }

    pub fn tick(&mut self, x: f32) -> f32 {
        if self.position < 0.4 {
            self.filter.tick(x).low_pass
        } else if self.position > 0.6 {
            self.filter.tick(x).high_pass
        } else {
            self.filter.tick(x);
            x
        }
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            *x = self.tick(*x);
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        let a = 13.73;
        self.position = attributes.tone;
        if self.position < 0.4 {
            let phase = self.position / 0.4;
            let voct = phase * 9.0;
            let cutoff = a * libm::powf(2.0, voct);
            self.filter.set_frequency(cutoff);
        } else if self.position > 0.6 {
            let phase = (self.position - 0.6) / 0.4;
            let voct = phase * 8.0 + 1.0;
            let cutoff = a * libm::powf(2.0, voct);
            self.filter.set_frequency(cutoff);
        }
    }
}
