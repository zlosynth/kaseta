//! Trigger output abstraction keeping it up.

/// Abstraction of trigger output.
///
/// This is useful when a trigger is triggered by a control loop
/// and it should remain up for a moment, for other modules to get
/// a chance to detect it.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Trigger {
    since: u32,
}

impl Trigger {
    pub fn trigger(&mut self) {
        self.since = 0;
    }

    pub fn tick(&mut self) {
        self.since = self.since.saturating_add(1);
    }

    pub fn triggered(&self) -> bool {
        self.since < 10
    }
}
