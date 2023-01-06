use super::interval_detector::IntervalDetector;

/// Identify incoming clock signal on control input.
///
/// This is only a momentary detector. To persist once detected
/// tempo, the result needs to be snapshotted and stored elsewhere.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ClockDetector {
    detector: IntervalDetector,
}

impl ClockDetector {
    pub fn trigger(&mut self) {
        self.detector.trigger();
    }

    pub fn just_detected(&self) -> bool {
        self.detector.just_detected
    }

    pub fn tick(&mut self) {
        self.detector.tick();
        self.detector.reset_if_inactive();
    }

    pub fn reset(&mut self) {
        self.detector.reset();
    }

    pub fn detected_tempo(&self) -> Option<u32> {
        self.detector.tempo
    }
}
