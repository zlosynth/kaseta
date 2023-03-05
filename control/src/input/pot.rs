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
    buffer: Buffer<32>,
    pub last_activation_movement: u32,
    pub last_value_above_noise: f32,
}

impl Pot {
    pub fn update(&mut self, value: f32) {
        self.buffer.write(value);

        self.last_activation_movement = if self.traveled_more_than(0.01) {
            0
        } else {
            self.last_activation_movement.saturating_add(1)
        };

        let value = self.buffer.read();
        let diff = (self.last_value_above_noise - value).abs();
        if diff > 0.002 {
            self.last_value_above_noise = value;
        } else if value < 0.0001 {
            self.last_value_above_noise = 0.0;
        } else if value > 0.9999 {
            self.last_value_above_noise = 1.0;
        }
    }

    pub fn value(&self) -> f32 {
        self.buffer.read()
    }

    pub fn activation_movement(&self) -> bool {
        self.last_activation_movement == 0
    }

    fn traveled_more_than(&self, toleration: f32) -> bool {
        self.buffer.traveled().abs() > toleration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_some_is_being_written_its_value_should_eventually_reach_it() {
        let mut pot = Pot::default();

        let mut value = pot.value();
        for _ in 0..32 {
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

    #[test]
    fn when_pot_moves_near_zero_it_snaps_to_it_despite_not_traveling_enough_distance() {
        let mut pot = Pot::default();

        for _ in 0..32 {
            pot.update(0.004);
        }
        for _ in 0..32 {
            pot.update(0.008);
        }
        for _ in 0..32 {
            pot.update(0.0);
        }

        assert_relative_eq!(pot.last_value_above_noise, 0.0);
    }

    #[test]
    fn when_pot_moves_near_full_it_snaps_to_it_despite_not_traveling_enough_distance() {
        let mut pot = Pot::default();

        for _ in 0..32 {
            pot.update(0.999);
        }
        for _ in 0..32 {
            pot.update(1.0);
        }

        assert_relative_eq!(pot.last_value_above_noise, 1.0);
    }
}
