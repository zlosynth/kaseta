//! Calculate and store calibration settings.

#[allow(unused_imports)]
use micromath::F32Ext;

/// Use to manage calibration of a control input.
///
/// This structure is calculates needed offset and scaling to adjust given
/// octave range to match 1V/oct precisely.
///
/// Note that the given input must be already scaled to volt range of the
/// hardware peripheral input.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Calibration {
    pub offset: f32,
    pub scaling: f32,
}

impl Calibration {
    pub fn try_new(octave_1: f32, octave_2: f32) -> Option<Self> {
        let (bottom, top) = if octave_1 < octave_2 {
            (octave_1, octave_2)
        } else {
            (octave_2, octave_1)
        };

        let distance = top - bottom;
        if !(0.5..=1.9).contains(&distance) {
            return None;
        }

        let scaling = 1.0 / (top - bottom);

        let scaled_bottom_fract = (bottom * scaling).fract();
        let offset = if scaled_bottom_fract > 0.5 {
            1.0 - scaled_bottom_fract
        } else {
            -1.0 * scaled_bottom_fract
        };

        Some(Self { offset, scaling })
    }

    pub fn apply(self, value: f32) -> f32 {
        value * self.scaling + self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    mod with_octave_2_above_octave_1 {
        use super::*;

        #[test]
        fn when_sets_proper_octaves_it_calibrates_properly() {
            let calibration = Calibration::try_new(1.1, 2.3).expect("Calibration failed");
            assert_relative_eq!(calibration.apply(1.1), 1.0);
            assert_relative_eq!(calibration.apply(2.3), 2.0);
        }

        #[test]
        fn when_sets_second_octave_too_close_it_fails() {
            assert!(Calibration::try_new(1.1, 1.3).is_none());
        }

        #[test]
        fn when_sets_second_octave_too_far_it_fails() {
            assert!(Calibration::try_new(1.3, 3.3).is_none());
        }
    }

    #[cfg(test)]
    mod with_octave_2_below_octave_1 {
        use super::*;

        #[test]
        fn when_sets_proper_octaves_it_sets_offset_and_scale_accordingly() {
            let calibration = Calibration::try_new(2.3, 1.1).expect("Calibration failed");
            assert_relative_eq!(calibration.apply(1.1), 1.0);
            assert_relative_eq!(calibration.apply(2.3), 2.0);
        }

        #[test]
        fn when_sets_second_octave_too_close_it_fails() {
            assert!(Calibration::try_new(1.3, 1.1).is_none());
        }

        #[test]
        fn when_sets_second_octave_too_far_it_fails() {
            assert!(Calibration::try_new(3.3, 1.3).is_none());
        }
    }
}
