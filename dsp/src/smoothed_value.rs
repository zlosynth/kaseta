//! Glide between values set by a slower loop.
//!
//! This may be useful to smoothen input parameters set through the contorol
//! loop before passing them to a DSP component.

use sirena::signal::Signal;

/// Smoothen transition between values.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SmoothedValue {
    step: f32,
    state: State,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum State {
    Stable(f32),
    Converging(f32, f32, f32),
}

impl SmoothedValue {
    pub fn new(value: f32, steps: u32) -> Self {
        Self {
            step: 1.0 / steps as f32,
            state: State::Stable(value),
        }
    }

    pub fn set(&mut self, value: f32) {
        let last_value = self.next();
        self.state = State::Converging(last_value, value, 0.0);
    }

    pub fn value(&self) -> f32 {
        match self.state {
            State::Stable(value) => value,
            State::Converging(old, new, phase) => old + (new - old) * phase,
        }
    }
}

impl Signal for SmoothedValue {
    fn next(&mut self) -> f32 {
        if let State::Converging(old, new, mut phase) = self.state {
            phase += self.step;
            if phase > 1.0 {
                self.state = State::Stable(new);
            } else {
                self.state = State::Converging(old, new, phase);
            }
        }
        self.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_smooth_value_when_left_intact_it_returns_stable_value() {
        let value = SmoothedValue::new(1.0, 64);
        for x in value.take(10) {
            assert_relative_eq!(x, 1.0);
        }
    }

    #[test]
    fn given_smooth_value_when_sets_a_new_value_it_linearly_progresses_to_it_and_remains_stable() {
        let mut value = SmoothedValue::new(1.0, 64);
        value.set(0.0);
        for (i, x) in value.by_ref().take(64).enumerate() {
            assert_relative_eq!(x, 1.0 - (i as f32 + 1.0) / 64.0);
        }
        for x in value.take(100) {
            assert_relative_eq!(x, 0.0);
        }
    }
}
