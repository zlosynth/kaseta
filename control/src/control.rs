//! Manage control input's state.

use crate::buffer::Buffer;
use crate::clock::ClockDetector;

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
    buffer: Buffer<4>,
    clock_detector: ClockDetector,
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

        self.clock_detector.tick();

        if self.buffer.read_raw() < 0.45 {
            self.clock_detector.reset();
        }

        let triggered = self.buffer.read_raw() > 0.9 && self.buffer.traveled() > 0.3;
        if triggered {
            self.clock_detector.trigger();
        }
    }

    pub fn value(&self) -> f32 {
        self.buffer.read()
    }

    // TODO: Use this to control speed
    #[allow(dead_code)]
    pub fn detected_tempo(&self) -> Option<u32> {
        self.clock_detector.tempo
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

    #[test]
    fn when_steady_clock_passes_in_it_detects_tempo() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        assert_eq!(control.detected_tempo().unwrap(), 2000);
    }

    #[test]
    fn when_clock_within_toleration_passes_it_detects_tempo() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1990 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..2030 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        assert_eq!(control.detected_tempo().unwrap(), 2000);
    }

    #[test]
    fn when_unevenly_spaced_triggers_are_given_it_is_not_recognized_as_tempo() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..930 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.5));
        }
        control.update(Some(1.0));
        assert!(control.detected_tempo().is_none());
    }

    #[test]
    fn when_signal_goes_below_zero_it_is_not_recognized_as_clock() {
        let mut control = Control::default();
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.1));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.1));
        }
        control.update(Some(1.0));
        for _ in 0..1999 {
            control.update(Some(0.1));
        }
        control.update(Some(1.0));
        assert!(control.detected_tempo().is_none());
    }

    #[test]
    fn when_signal_has_fast_enough_attacks_it_is_recognized_as_clock() {
        let mut control = Control::default();

        fn attack(control: &mut Control, should_detect: bool) {
            let mut detected = false;
            let attack = 3;
            for i in 0..=attack {
                control.update(Some(0.5 + 0.5 * (i as f32 / attack as f32)));
                detected |= control.detected_tempo().is_some();
            }
            assert_eq!(should_detect, detected, "{:?}", control);
        }

        fn silence(control: &mut Control) {
            for _ in 0..1999 {
                control.update(Some(0.5));
                assert!(control.detected_tempo().is_none());
            }
        }

        attack(&mut control, false);
        silence(&mut control);
        attack(&mut control, false);
        silence(&mut control);
        attack(&mut control, false);
        silence(&mut control);
        attack(&mut control, true);
    }

    #[test]
    fn when_signal_does_not_have_fast_attacks_it_is_not_recognized_as_clock() {
        let mut control = Control::default();

        fn attack(control: &mut Control) {
            for i in 0..200 {
                control.update(Some(0.5 + 0.5 * (i as f32 / 200.0)));
                assert!(control.detected_tempo().is_none());
            }
        }

        fn silence(control: &mut Control) {
            for _ in 0..1999 {
                control.update(Some(0.1));
                assert!(control.detected_tempo().is_none());
            }
        }

        attack(&mut control);
        silence(&mut control);
        attack(&mut control);
        silence(&mut control);
        attack(&mut control);
        silence(&mut control);
        attack(&mut control);
        silence(&mut control);
    }
}
