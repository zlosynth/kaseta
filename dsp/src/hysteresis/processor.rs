use super::makeup;
use super::simulation::Simulation;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct State {
    simulation: Simulation,
    makeup: f32,
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub drive: f32,
    pub saturation: f32,
    pub width: f32,
}

impl State {
    #[allow(clippy::let_and_return)]
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let simulation = Simulation::new(sample_rate);

        let state = {
            let mut state = Self {
                simulation,
                makeup: 0.0,
            };
            state.set_attributes(Attributes::default());
            state
        };

        state
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.simulation.set_drive(attributes.drive);
        self.simulation.set_saturation(attributes.saturation);
        self.simulation.set_width(attributes.width);
        self.makeup = makeup::calculate(attributes.drive, attributes.saturation, attributes.width);
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            *x = self.simulation.process(*x) * self.makeup;
        }
    }
}
