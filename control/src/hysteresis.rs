use super::calculate;
use crate::taper;

// 0.1 is almost clean, and with high pre-amp it still passes through signal.
// 30 is well in the extreme, but still somehow stable.
const DRIVE_RANGE: (f32, f32) = (0.05, 30.0);

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
pub fn calculate_drive(cache: &Cache) -> f32 {
    calculate(
        Some(cache.drive_pot),
        Some(cache.drive_cv),
        DRIVE_RANGE,
        Some(taper::log),
    )
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
