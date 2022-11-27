//! Led abstraction keeping it lit.

/// Abstraction of leds
///
/// This is useful when a led blink is triggered by a control loop
/// and it should remain lit for a moment.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Led {
    since: u32,
}

impl Led {
    pub fn trigger(&mut self) {
        self.since = 0;
    }

    pub fn tick(&mut self) {
        self.since = self.since.saturating_add(1);
    }

    pub fn triggered(&self) -> bool {
        self.since < 100
    }
}
