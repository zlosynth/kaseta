use super::calculate;
use crate::taper;

const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 0.2);

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub depth_pot: f32,
    pub depth_cv: f32,
}

#[allow(clippy::let_and_return)]
pub fn calculate_depth(cache: &Cache) -> f32 {
    calculate(
        Some(cache.depth_pot),
        Some(cache.depth_cv),
        WOW_DEPTH_RANGE,
        Some(taper::log),
    )
}
