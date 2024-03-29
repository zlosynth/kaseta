mod heads;
mod hysteresis;
mod pre_amp;
mod speed;
mod taper;
mod tone;
mod wow_flutter;

#[allow(clippy::let_and_return)]
pub fn calculate(
    pot: f32,
    cv: Option<f32>,
    range: (f32, f32),
    taper_function: Option<fn(f32) -> f32>,
) -> f32 {
    let sum = sum(pot, cv);
    calculate_from_sum(sum, range, taper_function)
}

pub fn sum(pot: f32, cv: Option<f32>) -> f32 {
    (pot + cv.unwrap_or(0.0)).clamp(0.0, 1.0)
}

#[allow(clippy::let_and_return)]
pub fn calculate_from_sum(
    sum: f32,
    range: (f32, f32),
    taper_function: Option<fn(f32) -> f32>,
) -> f32 {
    let curved = if let Some(taper_function) = taper_function {
        taper_function(sum)
    } else {
        sum
    };
    let scaled = curved * (range.1 - range.0) + range.0;
    scaled
}
