use super::calculate;
use crate::taper;

// Coefficients for quadratic function fitting drive breakout limit based on bias
const D4B_B: f32 = 1.0;
const D4B_A1: f32 = 0.0;
const D4B_A2: f32 = -0.2;

// Maximum limit of how much place on the slider is occupied by drive. This
// gets scaled down based on bias.
const DRIVE_PORTION: f32 = 1.0 / 2.0;
const DRIVE_RANGE: (f32, f32) = (0.1, 1.1);
const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);
const BIAS_RANGE: (f32, f32) = (0.01, 1.0);

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub drive_pot: f32,
    pub drive_cv: f32,
    pub bias_pot: f32,
    pub bias_cv: f32,
}

#[allow(clippy::let_and_return)]
pub fn calculate_drive_and_saturation(cache: &Cache) -> (f32, f32) {
    let control = (cache.drive_pot + cache.drive_cv).clamp(0.0, 1.0);

    let drive = calculate(
        Some((control / DRIVE_PORTION).min(1.0)),
        None,
        DRIVE_RANGE,
        None,
    );

    let saturation = calculate(
        Some(((control - DRIVE_PORTION) / (1.0 - DRIVE_PORTION)).clamp(0.0, 1.0)),
        None,
        SATURATION_RANGE,
        None,
    );

    (drive, saturation)
}

#[allow(clippy::let_and_return)]
pub fn calculate_bias(cache: &Cache, drive: f32) -> f32 {
    let max_bias = max_bias_for_drive(drive).clamp(BIAS_RANGE.0, BIAS_RANGE.1);
    calculate(
        Some(cache.bias_pot),
        Some(cache.bias_cv),
        (0.01, max_bias),
        Some(taper::log),
    )
}

// Using <https://mycurvefit.com/> quadratic interpolation over data gathered
// while testing the instability peak. The lower bias end was flattened not
// to overdrive too much. Input data (maximum stable bias per drive):
// 1.0 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.2 0.1
// 0.1 0.992 1.091 1.240 1.240 1.388 1.580 1.580 1.780 1.780
fn max_bias_for_drive(drive: f32) -> f32 {
    let drive2 = drive * drive;
    D4B_B + D4B_A1 * drive + D4B_A2 * drive2
}
