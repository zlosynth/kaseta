use super::calculate;
use crate::taper;

// Coefficients for cubic function fitting drive breakout limit based on bias
const D4B_B: f32 = 2.185_433;
const D4B_A1: f32 = 3.910_791;
const D4B_A2: f32 = 7.821_096;
const D4B_A3: f32 = 5.846_348;
const MAX_DRIVE: f32 = D4B_B;

// Maximum limit of how much place on the slider is occupied by drive. This
// gets scaled down based on bias.
const DRIVE_PORTION: f32 = 3.0 / 4.0;
const DRIVE_RANGE: (f32, f32) = (0.1, MAX_DRIVE);
const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);
// Width (1.0 - bias) must never reach 1.0, otherwise it panics due to division by zero
const BIAS_RANGE: (f32, f32) = (0.0001, 1.0);

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub drive_pot: f32,
    pub drive_cv: f32,
    pub bias_pot: f32,
    pub bias_cv: f32,
}

#[allow(clippy::let_and_return)]
pub fn calculate_drive_and_saturation(cache: &Cache, bias: f32) -> (f32, f32) {
    let control = (cache.drive_pot + cache.drive_cv).clamp(0.0, 1.0);
    let max_drive = max_drive_for_bias(bias);

    let drive = calculate(
        Some((control / DRIVE_PORTION).min(1.0)),
        None,
        DRIVE_RANGE,
        Some(taper::log),
    )
    .min(max_drive);

    let drive_slider_limit = (max_drive / MAX_DRIVE) * DRIVE_PORTION;
    let saturation = calculate(
        Some(((control - drive_slider_limit) / (1.0 - drive_slider_limit)).clamp(0.0, 1.0)),
        None,
        SATURATION_RANGE,
        None,
    );

    (drive, saturation)
}

#[allow(clippy::let_and_return)]
pub fn calculate_bias(cache: &Cache) -> f32 {
    calculate(
        Some(cache.bias_pot),
        Some(cache.bias_cv),
        BIAS_RANGE,
        Some(taper::log),
    )
}

// Using <https://mycurvefit.com/> cubic interpolation over data gathered
// while testing the instability peak. The lower bias end was flattened not
// to overdrive too much. Input data (maximum stable drive per bias):
// 0.1 0.992 1.091 1.240 1.240 1.388 1.580 1.580 1.780 1.780
// 1.0 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.2 0.1
fn max_drive_for_bias(bias: f32) -> f32 {
    let bias2 = bias * bias;
    let bias3 = bias2 * bias;
    D4B_B - D4B_A1 * bias + D4B_A2 * bias2 - D4B_A3 * bias3
}
