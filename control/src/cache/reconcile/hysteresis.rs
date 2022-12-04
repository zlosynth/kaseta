use super::calculate;
use super::taper;
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

const DRY_WET_RANGE: (f32, f32) = (0.0, 1.0);
const DRIVE_RANGE: (f32, f32) = (0.1, 1.1);
const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);
const BIAS_RANGE: (f32, f32) = (0.01, 1.0);

impl Store {
    pub fn reconcile_hysteresis(&mut self) {
        // Maximum limit of how much place on the slider is occupied by drive. This
        // gets scaled down based on bias.
        const DRIVE_PORTION: f32 = 1.0 / 2.0;

        self.cache.attributes.dry_wet = calculate(
            self.input.dry_wet.value(),
            self.control_value_for_attribute(AttributeIdentifier::DryWet),
            DRY_WET_RANGE,
            None,
        );

        let drive_input = calculate(
            self.input.drive.value(),
            self.control_value_for_attribute(AttributeIdentifier::Drive),
            (0.0, 1.0),
            None,
        );

        let drive = calculate(
            (drive_input / DRIVE_PORTION).min(1.0),
            None,
            DRIVE_RANGE,
            None,
        );
        self.cache.attributes.drive = drive;

        self.cache.attributes.saturation = calculate(
            ((drive_input - DRIVE_PORTION) / (1.0 - DRIVE_PORTION)).clamp(0.0, 1.0),
            None,
            SATURATION_RANGE,
            None,
        );

        let max_bias = max_bias_for_drive(drive).clamp(BIAS_RANGE.0, BIAS_RANGE.1);
        self.cache.attributes.bias = calculate(
            self.input.bias.value(),
            self.control_value_for_attribute(AttributeIdentifier::Bias),
            (0.01, max_bias),
            Some(taper::log),
        );
    }
}

// Using <https://mycurvefit.com/> quadratic interpolation over data gathered
// while testing the instability peak. The lower bias end was flattened not
// to overdrive too much. Input data (maximum stable bias per drive):
// 1.0 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.2 0.1
// 0.1 0.992 1.091 1.240 1.240 1.388 1.580 1.580 1.780 1.780
fn max_bias_for_drive(drive: f32) -> f32 {
    const D4B_B: f32 = 1.0;
    const D4B_A1: f32 = 0.0;
    const D4B_A2: f32 = -0.2;

    let drive2 = drive * drive;
    D4B_B + D4B_A1 * drive + D4B_A2 * drive2
}
