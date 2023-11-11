//! Manage control input's state.

#[allow(unused_imports)]
use micromath::F32Ext;

use super::buffer::Buffer;

/// Use this to hold control input's state over time.
///
/// This detects plugging and unplugging of the input, smoothens the value,
/// and detects clock signal.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Control {
    pub is_plugged: bool,
    pub was_plugged: bool,
    pub was_unplugged: bool,
    pub last_value_above_noise: f32,
    buffer: Buffer<4>,
}

impl Control {
    pub fn update(&mut self, value: Option<f32>) {
        let was_plugged = self.is_plugged;
        if let Some(value) = value {
            self.is_plugged = true;
            self.buffer.write(value);
        } else {
            self.is_plugged = false;
            self.buffer.reset();
        }
        self.was_plugged = !was_plugged && self.is_plugged;
        self.was_unplugged = was_plugged && !self.is_plugged;

        let value = self.buffer.read();
        let diff = (self.last_value_above_noise - value).abs();
        if diff > 0.01 {
            self.last_value_above_noise = value;
        } else if value < -4.97 {
            self.last_value_above_noise = -5.0;
        } else if value > 4.97 {
            self.last_value_above_noise = 5.0;
        }
    }

    pub fn value(&self) -> f32 {
        self.buffer.read()
    }

    pub fn value_raw(&self) -> f32 {
        self.buffer.read_raw()
    }

    pub fn triggered(&self) -> bool {
        self.previous_value_raw() <= 0.9 && self.value_raw() > 0.9 && self.traveled() > 0.3
    }

    fn previous_value_raw(&self) -> f32 {
        self.buffer.read_previous_raw()
    }

    fn traveled(&self) -> f32 {
        self.buffer.traveled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_none_is_written_its_value_should_be_zero() {
        let mut cv = Control::default();
        cv.update(Some(10.0));
        cv.update(None);
        assert_relative_eq!(cv.value(), 0.0);
    }

    #[test]
    fn when_some_is_being_written_its_value_should_eventually_reach_it() {
        let mut cv = Control::default();

        let mut value = cv.value();
        for _ in 0..20 {
            cv.update(Some(1.0));
            let new_value = cv.value();
            assert!(new_value > value);
            value = new_value;
            if relative_eq!(value, 1.0) {
                return;
            }
        }

        panic!("Control have not reached the target {}", value);
    }

    #[test]
    fn when_some_is_written_after_none_it_reports_as_plugged_for_one_cycle() {
        let mut control = Control::default();
        control.update(None);
        control.update(Some(10.0));
        assert!(control.was_plugged);
        control.update(None);
        assert!(!control.was_plugged);
    }
}
