use super::calculate;
use crate::taper;

// 0.1 is almost clean, and with high pre-amp it still passes through signal.
// 2.0 is absolute maximum drive that can be stable with 4pp amplitude. The
// actual used maximum is always calculated based on the current bias.
const DRIVE_RANGE: (f32, f32) = (0.1, 2.0);

const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);

// Width (1.0 - bias) must never reach 1.0, otherwise it panics due to division by zero
const BIAS_RANGE: (f32, f32) = (0.0001, 1.0);

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub drive_pot: f32,
    pub drive_cv: f32,
    pub saturation_pot: f32,
    pub saturation_cv: f32,
    pub bias_pot: f32,
    pub bias_cv: f32,
}

#[allow(clippy::let_and_return)]
pub fn calculate_drive(cache: &Cache, bias: f32) -> f32 {
    let bias2 = bias * bias;
    let bias3 = bias2 * bias;
    // Using <https://mycurvefit.com/> cubic interpolation over data gathered
    // while testing the instability peak. The lower bias end was flattened not
    // to overdrive too much. Input data (maximum stable drive per bias):
    // 0.1 0.992 1.091 1.240 1.240 1.388 1.580 1.580 1.780 1.780
    // 1.0 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.2 0.1
    let max_drive = 2.185_433 - 3.910_791 * bias + 7.821_096 * bias2 - 5.846_348 * bias3;
    calculate(
        Some(cache.drive_pot),
        Some(cache.drive_cv),
        DRIVE_RANGE,
        Some(taper::log),
    )
    .min(max_drive)
}

#[allow(clippy::let_and_return)]
pub fn calculate_saturation(cache: &Cache) -> f32 {
    calculate(
        Some(cache.saturation_pot),
        Some(cache.saturation_cv),
        SATURATION_RANGE,
        Some(taper::reverse_log),
    )
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
