use sirena::signal::Signal;

use super::makeup;
use super::simulation::Simulation;
use crate::smoothed_value::SmoothedValue;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct State {
    simulation: Simulation,
    drive: SmoothedValue,
    saturation: SmoothedValue,
    width: SmoothedValue,
    makeup: SmoothedValue,
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
        let smoothing_steps = (sample_rate * 0.001) as u32;
        let simulation = Simulation::new(sample_rate);
        let drive = SmoothedValue::new(0.0, smoothing_steps);
        let saturation = SmoothedValue::new(0.0, smoothing_steps);
        let width = SmoothedValue::new(0.0, smoothing_steps);
        let makeup = SmoothedValue::new(0.0, smoothing_steps);

        let state = {
            let mut state = Self {
                simulation,
                drive,
                saturation,
                width,
                makeup,
            };
            state.set_attributes(Attributes::default());
            state
        };

        state
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.drive.set(attributes.drive);
        self.saturation.set(attributes.saturation);
        self.width.set(attributes.width);
        self.makeup.set(makeup::calculate(
            attributes.drive,
            attributes.saturation,
            attributes.width,
        ));
    }
}

pub trait SignalApplyHysteresis: Signal {
    fn apply_hysteresis(self, state: &mut State) -> ApplyHysteresis<Self>
    where
        Self: Sized,
    {
        ApplyHysteresis {
            source: self,
            state,
        }
    }
}

impl<T> SignalApplyHysteresis for T where T: Signal {}

pub struct ApplyHysteresis<'a, S> {
    source: S,
    state: &'a mut State,
}

impl<'a, S> Signal for ApplyHysteresis<'a, S>
where
    S: Signal,
{
    fn next(&mut self) -> f32 {
        let drive = self.state.drive.next();
        let saturation = self.state.saturation.next();
        let width = self.state.width.next();

        let makeup = makeup::calculate(drive, saturation, width);

        self.state.simulation.set_drive(drive);
        self.state.simulation.set_saturation(saturation);
        self.state.simulation.set_width(width);
        self.state.simulation.process(self.source.next()) * makeup
    }
}
