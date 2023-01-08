#[allow(unused_imports)]
use micromath::F32Ext;

use crate::linkwitz_riley_filter::LinkwitzRileyFilter;

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub tone: f32,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Tone {
    sample_rate: f32,
    lpf: LinkwitzRileyFilter,
    hpf: LinkwitzRileyFilter,
}

impl Tone {
    /// # Panics
    ///
    /// Sample rate must be higher than 500, otherwise the filter becomes
    /// unstable and the initialization will panic.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: sample_rate as f32,
            lpf: LinkwitzRileyFilter::new(sample_rate as f32),
            hpf: LinkwitzRileyFilter::new(sample_rate as f32),
        }
    }

    pub fn tick(&mut self, x: f32) -> f32 {
        self.lpf.tick(self.hpf.tick(x).high_pass).low_pass
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            *x = self.tick(*x);
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        let a = 13.73;
        if attributes.tone < 0.4 {
            self.hpf.set_frequency(0.0);
            let phase = attributes.tone / 0.4;
            let voct = phase * 10.645;
            let cutoff = a * libm::powf(2.0, voct);
            self.lpf.set_frequency(cutoff);
        } else if attributes.tone < 0.6 {
            self.lpf.set_frequency(self.sample_rate * 0.48);
            self.hpf.set_frequency(0.0);
        } else {
            self.lpf.set_frequency(self.sample_rate * 0.48);
            let phase = (attributes.tone - 0.6) / 0.4;
            let voct = phase * 10.0;
            let cutoff = a * libm::powf(2.0, voct);
            self.hpf.set_frequency(cutoff);
        }
    }
}
