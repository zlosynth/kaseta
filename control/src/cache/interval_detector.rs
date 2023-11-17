//! Evaluate whether triggers are forming a clock.

use core::ops::Range;

/// Detect clock tempo in signal.
///
/// Call `tick` for every sample and `trigger` for those that have a raised
/// edge. Read `detected_tempo` to see if clock was detected. The detected
/// value is held until the next time `trigger` is executed.
///
/// If for any reason the detector should be invalidated, call `reset`.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IntervalDetector {
    trigger_age: [u32; 3],
    pub tempo: Option<u32>,
    pub just_detected: bool,
    pub first_beat_after_detection: bool,
}

impl IntervalDetector {
    pub fn trigger(&mut self) {
        let minus_1 = self.trigger_age[2];
        let minus_2 = self.trigger_age[1];
        let minus_3 = self.trigger_age[0];

        let distance = minus_1;
        let allowed_range = toleration(distance);
        if distance > 100
            && allowed_range.contains(&(minus_2 - minus_1))
            && allowed_range.contains(&(minus_3 - minus_2))
        {
            self.tempo = Some(distance);
            self.just_detected = true;
        } else {
            self.tempo = None;
        }

        self.trigger_age[0] = self.trigger_age[1];
        self.trigger_age[1] = self.trigger_age[2];
        self.trigger_age[2] = 0;
    }

    pub fn reset(&mut self) {
        self.trigger_age = [0, 0, 0];
        self.tempo = None;
        self.just_detected = false;
    }

    pub fn tick(&mut self) {
        for x in self.trigger_age.iter_mut() {
            *x = x.saturating_add(1);
        }
        self.just_detected = false;

        if let Some(tempo) = self.tempo {
            self.first_beat_after_detection = self.trigger_age[2] == tempo;
        }
    }
}

fn toleration(distance: u32) -> Range<u32> {
    let tolerance = distance / 10;
    distance - tolerance..distance + tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_triggered_in_exact_interval_it_detects_tempo() {
        let mut detector = IntervalDetector::default();
        for _ in 0..4 {
            for _ in 0..2000 {
                detector.tick();
            }
            detector.trigger();
        }
        assert_eq!(detector.tempo, Some(2000));
    }

    #[test]
    fn when_triggered_in_rough_interval_within_toleration_it_detects_tempo() {
        let mut detector = IntervalDetector::default();
        detector.trigger();
        for _ in 0..1990 {
            detector.tick();
        }
        detector.trigger();
        for _ in 0..2059 {
            detector.tick();
        }
        detector.trigger();
        for _ in 0..2000 {
            detector.tick();
        }
        detector.trigger();
        assert_eq!(detector.tempo, Some(2000));
    }

    #[test]
    fn when_triggered_too_fast_it_does_not_detect_tempo() {
        let mut detector = IntervalDetector::default();
        for _ in 0..4 {
            for _ in 0..20 {
                detector.tick();
            }
            detector.tick();
        }
        assert_eq!(detector.tempo, None);
    }

    #[test]
    fn when_triggered_in_unequal_interval_it_does_not_detect_tempo() {
        let mut detector = IntervalDetector::default();
        detector.trigger();
        for _ in 0..2000 {
            detector.tick();
        }
        detector.trigger();
        for _ in 0..2000 {
            detector.tick();
        }
        detector.trigger();
        for _ in 0..1083 {
            detector.tick();
        }
        detector.trigger();
        assert_eq!(detector.tempo, None);
    }
}
