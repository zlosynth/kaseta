#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PreAmp {
    gain: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub gain: f32,
}

impl PreAmp {
    pub fn new() -> Self {
        Self { gain: 0.0 }
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            *x *= self.gain;
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.gain = attributes.gain;
    }
}
