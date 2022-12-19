use core::f32::consts::PI;

const SUB_COEFFICIENT: f32 = 0.499;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Oscillator {
    sample_rate: f32,
    frequency: f32,
    phase_base: f32,
    phase_sub: f32,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub frequency: f32,
}

impl Oscillator {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            frequency: 0.0,
            phase_base: 0.0,
            phase_sub: 0.0,
        }
    }

    pub fn populate(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            // TODO: Compare performance with micromath
            let x_base = libm::sinf(self.phase_base * 2.0 * PI);
            let x_sub = libm::sinf(self.phase_sub * 2.0 * PI);
            *x = (x_base + x_sub) * 1.0;

            let step = self.frequency / self.sample_rate;
            self.phase_base += step;
            self.phase_sub += step * SUB_COEFFICIENT;
        }

        while self.phase_base > 1.0 {
            self.phase_base -= 1.0;
        }
        while self.phase_sub > 1.0 {
            self.phase_sub -= 1.0;
        }
    }

    pub fn set_attributes(&mut self, attributes: &Attributes) {
        self.frequency = attributes.frequency;
    }
}
