//! Pot abstraction tracking its state over time.

#[allow(unused_imports)]
use micromath::F32Ext;

use super::buffer::Buffer;

/// Abstraction of a potentiometer.
///
/// Use it to smoothen the value received from pots and detect their
/// movement.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Pot {
    buffer: Buffer<4>,
}

impl Pot {
    pub fn update(&mut self, value: f32) {
        self.buffer.write(value);
    }

    // TODO: Implement window support
    pub fn value(&self) -> f32 {
        self.buffer.read()
    }

    pub fn active(&self) -> bool {
        self.buffer.traveled().abs() > 0.001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_some_is_being_written_its_value_should_eventually_reach_it() {
        let mut pot = Pot::default();

        let mut value = pot.value();
        for _ in 0..20 {
            pot.update(1.0);
            let new_value = pot.value();
            assert!(new_value > value);
            value = new_value;
            if relative_eq!(value, 1.0) {
                return;
            }
        }

        panic!("Control have not reached the target {}", value);
    }
}