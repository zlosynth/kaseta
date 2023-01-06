use super::interval_detector::IntervalDetector;

/// Identify tempo being tapped in by the user.
///
/// This is only a momentary detector. To persist once detected
/// tempo, the result needs to be snapshotted and stored elsewhere.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TapDetector {
    detector: IntervalDetector,
}

impl TapDetector {
    pub fn trigger(&mut self) {
        self.detector.trigger();
    }

    pub fn just_detected(&self) -> bool {
        self.detector.just_detected
    }

    pub fn tick(&mut self) {
        self.detector.tick();
    }

    pub fn reset(&mut self) {
        self.detector.reset();
    }

    pub fn detected_tempo(&self) -> Option<u32> {
        self.detector.tempo
    }
}
