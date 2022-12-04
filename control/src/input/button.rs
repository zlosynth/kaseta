//! Manage button's state.

/// Use this to hold buttons state over time.
///
/// Detects clicking, holding, or tapped-in tempo.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Button {
    pub pressed: bool,
    pub clicked: bool,
    pub held: u32,
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
}
