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
        let max_cutoff = 6000.0;
        self.position = attributes.tone;
        if self.position < 0.4 {
            let position = log(self.position / 0.4);
            self.filter.set_frequency(position * max_cutoff);
        } else if self.position > 0.6 {
            let position = log((self.position - 0.6) * (1.0 / 0.4));
            self.filter.set_frequency(position * max_cutoff);
        }
    }
}

const LOG: [f32; 22] = [
    0.0,
    0.005,
    0.019_996_643,
    0.040_958_643,
    0.062_983_93,
    0.086_186_11,
    0.110_698_28,
    0.136_677_15,
    0.164_309_44,
    0.19382,
    0.225_483,
    0.259_637_3,
    0.296_708_64,
    0.337_242_2,
    0.381_951_87,
    0.431_798_22,
    0.488_116_62,
    0.552_841_96,
    0.628_932_1,
    0.721_246_36,
    0.838_632,
    1.0,
];

fn log(position: f32) -> f32 {
    if position < 0.0 {
        return 0.0;
    } else if position > 1.0 {
        return 1.0;
    }

    let array_position = position * (LOG.len() - 1) as f32;
    let index_a = array_position as usize;
    let index_b = (array_position as usize + 1).min(LOG.len() - 1);
    let remainder = array_position.fract();

    let value = LOG[index_a];
    let delta_to_next = LOG[index_b] - LOG[index_a];

    value + delta_to_next * remainder
}
