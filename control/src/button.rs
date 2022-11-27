//! Manage button's state.

use crate::clock::ClockDetector;

/// Use this to hold buttons state over time.
///
/// Detects clicking, holding, or tapped-in tempo.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Button {
    pub pressed: bool,
    pub clicked: bool,
    pub held: u32,
    clock_detector: ClockDetector,
}

impl Button {
    pub fn update(&mut self, down: bool) {
        let was_pressed = self.pressed;
        self.pressed = down;
        self.clicked = !was_pressed && self.pressed;
        self.held = if self.pressed {
            self.held.saturating_add(1)
        } else {
            0
        };

        self.clock_detector.tick();
        if self.clicked {
            self.clock_detector.trigger();
        }
    }

    pub fn detected_tempo(&self) -> Option<u32> {
        self.clock_detector.tempo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_was_up_and_now_is_down_it_is_marked_as_clicked() {
        let mut button = Button::default();
        assert!(!button.clicked);
        button.update(true);
        assert!(button.clicked);
        button.update(true);
        assert!(!button.clicked);
        button.update(false);
        assert!(!button.clicked);
    }

    #[test]
    fn when_is_down_it_reports_how_many_cycles() {
        let mut button = Button::default();
        assert_eq!(button.held, 0);
        button.update(false);
        assert_eq!(button.held, 0);
        button.update(true);
        assert_eq!(button.held, 1);
        button.update(true);
        assert_eq!(button.held, 2);
        button.update(true);
        assert_eq!(button.held, 3);
        button.update(false);
        assert_eq!(button.held, 0);
    }

    #[test]
    fn when_clicked_in_exact_interval_it_detects_tempo() {
        let mut button = Button::default();
        for _ in 0..4 {
            for _ in 0..1999 {
                button.update(false);
            }
            button.update(true);
        }
        assert_eq!(button.clock_detector.tempo, Some(2000));
    }

    #[test]
    fn when_clicked_in_rough_interval_within_toleration_it_detects_tempo() {
        let mut button = Button::default();
        button.update(true);
        for _ in 0..1990 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..2059 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..1999 {
            button.update(false);
        }
        button.update(true);
        assert_eq!(button.clock_detector.tempo, Some(2000));
    }

    #[test]
    fn when_clicked_too_fast_it_does_not_detect_tempo() {
        let mut button = Button::default();
        for _ in 0..4 {
            for _ in 0..19 {
                button.update(false);
            }
            button.update(true);
        }
        assert_eq!(button.clock_detector.tempo, None);
    }

    #[test]
    fn when_clicked_in_unequal_interval_it_does_not_detect_tempo() {
        let mut button = Button::default();
        button.update(true);
        for _ in 0..1999 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..1999 {
            button.update(false);
        }
        button.update(true);
        for _ in 0..1083 {
            button.update(false);
        }
        button.update(true);
        assert_eq!(button.clock_detector.tempo, None);
    }
}
