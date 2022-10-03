use super::calculate;
use crate::taper;

const WOW_FREQUENCY_RANGE: (f32, f32) = (0.01, 4.0);
// The max depth is limited by frequency, in a way that the playing signal never
// goes backwards. For 0.02 minimal frequency, the maximum depth is defined by:
// (1.0 / 0.01) * 0.31
const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 16.0);
const WOW_FILTER_RANGE: (f32, f32) = (0.0, 10.0);
const WOW_AMPLITUDE_NOISE_RANGE: (f32, f32) = (0.0, 5.0);
const WOW_AMPLITUDE_SPRING_RANGE: (f32, f32) = (0.0, 300.0);
const WOW_PHASE_NOISE_RANGE: (f32, f32) = (0.0, 5.0);
const WOW_PHASE_SPRING_RANGE: (f32, f32) = (0.0, 10.0);
const WOW_PHASE_DRIFT_RANGE: (f32, f32) = (0.0, 1.0);

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub frequency_pot: f32,
    pub frequency_cv: f32,
    pub depth_pot: f32,
    pub depth_cv: f32,
    pub filter_pot: f32,
    pub amplitude_noise_pot: f32,
    pub amplitude_spring_pot: f32,
    pub phase_noise_pot: f32,
    pub phase_spring_pot: f32,
    pub phase_drift_pot: f32,
}

#[allow(clippy::let_and_return)]
pub fn calculate_frequency(cache: &Cache) -> f32 {
    calculate(
        Some(cache.frequency_pot),
        Some(cache.frequency_cv),
        WOW_FREQUENCY_RANGE,
        None,
    )
}

#[allow(clippy::let_and_return)]
pub fn calculate_depth(cache: &Cache, wow_frequency: f32) -> f32 {
    let wow_depth_sum = (cache.depth_pot + cache.depth_cv).clamp(0.0, 1.0);
    let wow_depth_curved = taper::log(wow_depth_sum);
    let max_depth = f32::min(WOW_DEPTH_RANGE.1, (1.0 / wow_frequency) * 0.31);
    let wow_depth_scaled = wow_depth_curved * (max_depth - WOW_DEPTH_RANGE.0) + WOW_DEPTH_RANGE.0;
    wow_depth_scaled
}

#[allow(clippy::let_and_return)]
pub fn calculate_amplitude_noise(cache: &Cache) -> f32 {
    calculate(
        Some(cache.amplitude_noise_pot),
        None,
        WOW_AMPLITUDE_NOISE_RANGE,
        None,
    )
}

#[allow(clippy::let_and_return)]
pub fn calculate_amplitude_spring(cache: &Cache) -> f32 {
    calculate(
        Some(cache.amplitude_spring_pot),
        None,
        WOW_AMPLITUDE_SPRING_RANGE,
        None,
    )
}

#[allow(clippy::let_and_return)]
pub fn calculate_phase_noise(cache: &Cache) -> f32 {
    calculate(
        Some(cache.phase_noise_pot),
        None,
        WOW_PHASE_NOISE_RANGE,
        None,
    )
}

#[allow(clippy::let_and_return)]
pub fn calculate_phase_spring(cache: &Cache) -> f32 {
    calculate(
        Some(cache.phase_spring_pot),
        None,
        WOW_PHASE_SPRING_RANGE,
        None,
    )
}

#[allow(clippy::let_and_return)]
pub fn calculate_phase_drift(cache: &Cache) -> f32 {
    calculate(
        Some(cache.phase_drift_pot),
        None,
        WOW_PHASE_DRIFT_RANGE,
        None,
    )
}

#[allow(clippy::let_and_return)]
pub fn calculate_filter(cache: &Cache) -> f32 {
    calculate(Some(cache.filter_pot), None, WOW_FILTER_RANGE, None)
}
