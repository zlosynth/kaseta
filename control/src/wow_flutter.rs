use super::calculate;

const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 0.2);
const FLUTTER_DEPTH_RANGE: (f32, f32) = (0.0, 0.0015);

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub depth_pot: f32,
    pub depth_cv: f32,
}

#[allow(clippy::let_and_return)]
pub fn calculate_wow_flutter_depth(cache: &Cache) -> (f32, f32) {
    let depth = calculate(
        Some(cache.depth_pot),
        Some(cache.depth_cv),
        (-1.0, 1.0),
        None,
    );
    if depth.is_sign_negative() {
        (calculate(Some(-depth), None, WOW_DEPTH_RANGE, None), 0.0)
    } else {
        (0.0, calculate(Some(depth), None, FLUTTER_DEPTH_RANGE, None))
    }
}
