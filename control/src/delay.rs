use super::calculate;
use crate::quantization::{quantize, Quantization};
use crate::taper;

const DELAY_LENGTH_RANGE: (f32, f32) = (0.02, 8.0);
const DELAY_HEAD_POSITION_RANGE: (f32, f32) = (0.0, 1.0);

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub length_pot: f32,
    pub length_cv: f32,
    pub head_position_pot: [f32; 4],
    pub head_position_cv: [f32; 4],
    pub quantization_6: bool,
    pub quantization_8: bool,
    pub head_play: [bool; 4],
    pub head_feedback: [bool; 4],
    pub head_feedback_amount: [f32; 4],
    pub head_volume: [f32; 4],
}

#[allow(clippy::let_and_return)]
pub fn calculate_length(cache: &Cache) -> f32 {
    calculate(
        Some(cache.length_pot),
        Some(cache.length_cv),
        DELAY_LENGTH_RANGE,
        Some(taper::reverse_log),
    )
}

#[allow(clippy::let_and_return)]
pub fn calculate_head_position(cache: &Cache, head: usize) -> f32 {
    let delay_head_position_sum =
        (cache.head_position_pot[head] + cache.head_position_cv[head]).clamp(0.0, 1.0);
    let delay_head_position_scaled = delay_head_position_sum
        * (DELAY_HEAD_POSITION_RANGE.1 - DELAY_HEAD_POSITION_RANGE.0)
        + DELAY_HEAD_POSITION_RANGE.0;
    let delay_head_position_quantized = if cache.quantization_6 || cache.quantization_8 {
        let quantization = Quantization::from((cache.quantization_6, cache.quantization_8));
        quantize(delay_head_position_scaled, quantization)
    } else {
        delay_head_position_scaled
    };

    delay_head_position_quantized
}
