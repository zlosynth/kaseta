//! Components of user inteface, passing user input to DSP and reactions back.
//!
//! It is mainly targetted to run in a firmware with multiple loops running in
//! different frequencies, passing messages from one to another. However, parts
//! of it may be useful in software as well.
//!
//! Following is an example of communication that may be used in an eurorack
//! module:
//!
//! ```text
//!                    [ DSPLoop ]
//!                       A   |
//!           (DSPAction) |   | (DSPAction)
//!                       |   V
//!                 [ Reducer {Cache} ] <--------> {Store}
//!                     A        |
//!     (ControlAction) |        | (DisplayReaction)
//!             +-------+        +--+
//!             |                   |
//!             |                   V
//!    [ ControlLoop ]         [ DisplayLoop ]
//!     |     |     |                 |
//!   [CV] [Pots] [Buttons]         [LEDs]
//! ```

#![no_std]
#![allow(clippy::items_after_statements)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]

#[cfg(test)]
#[macro_use]
extern crate approx;

mod quantization;
mod taper;

use kaseta_dsp::processor::Attributes;

use crate::quantization::{quantize, Quantization};

// Pre-amp scales between -20 to +28 dB.
const PRE_AMP_RANGE: (f32, f32) = (0.1, 25.0);

// 0.1 is almost clean, and with high pre-amp it still passes through signal.
// 30 is well in the extreme, but still somehow stable.
const DRIVE_RANGE: (f32, f32) = (0.05, 30.0);

const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);

// Width (1.0 - bias) must never reach 1.0, otherwise it panics due to division by zero
const BIAS_RANGE: (f32, f32) = (0.0001, 1.0);

const WOW_FREQUENCY_RANGE: (f32, f32) = (0.02, 4.0);
// The max depth is limited by frequency, in a way that the playing signal never
// goes backwards. For 0.02 minimal frequency, the maximum depth is defined by:
// (1.0 / 0.01) * 0.31
const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 16.0);

const DELAY_LENGTH_RANGE: (f32, f32) = (0.02, 8.0);
const DELAY_HEAD_POSITION_RANGE: (f32, f32) = (0.0, 1.0);

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ControlAction {
    SetPreAmpPot(f32),
    SetDrivePot(f32),
    SetDriveCV(f32),
    SetSaturationPot(f32),
    SetSaturationCV(f32),
    SetBiasPot(f32),
    SetBiasCV(f32),
    SetWowFrequencyPot(f32),
    SetWowFrequencyCV(f32),
    SetWowDepthPot(f32),
    SetWowDepthCV(f32),
    SetDelayLengthPot(f32),
    SetDelayLengthCV(f32),
    SetDelayHeadPositionPot(usize, f32),
    SetDelayHeadPositionCV(usize, f32),
    SetDelayQuantizationSix(bool),
    SetDelayQuantizationEight(bool),
    SetDelayHeadPlay(usize, bool),
    SetDelayHeadFeedback(usize, bool),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DSPReaction {
    pub pre_amp: f32,
    pub drive: f32,
    pub saturation: f32,
    pub bias: f32,
    pub wow_frequency: f32,
    pub wow_depth: f32,
    pub delay_length: f32,
    pub delay_head_position: [f32; 4],
    pub delay_head_play: [bool; 4],
    pub delay_head_feedback: [bool; 4],
}

impl From<DSPReaction> for Attributes {
    fn from(other: DSPReaction) -> Self {
        Attributes {
            pre_amp: other.pre_amp,
            drive: other.drive,
            saturation: other.saturation,
            width: 1.0 - other.bias,
            wow_frequency: other.wow_frequency,
            wow_depth: other.wow_depth,
            delay_length: other.delay_length,
            delay_head_1_position: other.delay_head_position[0],
            delay_head_2_position: other.delay_head_position[1],
            delay_head_3_position: other.delay_head_position[2],
            delay_head_4_position: other.delay_head_position[3],
            delay_head_1_play: other.delay_head_play[0],
            delay_head_2_play: other.delay_head_play[1],
            delay_head_3_play: other.delay_head_play[2],
            delay_head_4_play: other.delay_head_play[3],
            delay_head_1_feedback: other.delay_head_feedback[0],
            delay_head_2_feedback: other.delay_head_feedback[1],
            delay_head_3_feedback: other.delay_head_feedback[2],
            delay_head_4_feedback: other.delay_head_feedback[3],
        }
    }
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub pre_amp_pot: f32,
    pub drive_pot: f32,
    pub drive_cv: f32,
    pub saturation_pot: f32,
    pub saturation_cv: f32,
    pub bias_pot: f32,
    pub bias_cv: f32,
    pub wow_frequency_pot: f32,
    pub wow_frequency_cv: f32,
    pub wow_depth_pot: f32,
    pub wow_depth_cv: f32,
    pub delay_length_pot: f32,
    pub delay_length_cv: f32,
    pub delay_head_position_pot: [f32; 4],
    pub delay_head_position_cv: [f32; 4],
    pub delay_quantization_6: bool,
    pub delay_quantization_8: bool,
    pub delay_head_play: [bool; 4],
    pub delay_head_feedback: [bool; 4],
}

#[must_use]
pub fn reduce_control_action(action: ControlAction, cache: &mut Cache) -> DSPReaction {
    apply_control_action_in_cache(action, cache);
    cook_dsp_reaction_from_cache(cache)
}

#[must_use]
pub fn cook_dsp_reaction_from_cache(cache: &Cache) -> DSPReaction {
    let pre_amp = calculate_pre_amp(cache);
    let drive = calculate_drive(cache);
    let saturation = calculate_saturation(cache);
    let bias = calculate_bias(cache);
    let wow_frequency = calculate_wow_frequency(cache);
    let wow_depth = calculate_wow_depth(cache, wow_frequency);
    let delay_length = calculate_delay_length(cache);
    let delay_head_1_position = calculate_delay_head_position(cache, 0);
    let delay_head_2_position = calculate_delay_head_position(cache, 1);
    let delay_head_3_position = calculate_delay_head_position(cache, 2);
    let delay_head_4_position = calculate_delay_head_position(cache, 3);
    DSPReaction {
        pre_amp,
        drive,
        saturation,
        bias,
        wow_frequency,
        wow_depth,
        delay_length,
        delay_head_position: [
            delay_head_1_position,
            delay_head_2_position,
            delay_head_3_position,
            delay_head_4_position,
        ],
        delay_head_play: cache.delay_head_play,
        delay_head_feedback: cache.delay_head_feedback,
    }
}

#[allow(clippy::let_and_return)]
fn calculate_pre_amp(cache: &Cache) -> f32 {
    let pre_amp_curved = taper::log(cache.pre_amp_pot);
    let pre_amp_scaled = pre_amp_curved * (PRE_AMP_RANGE.1 - PRE_AMP_RANGE.0) + PRE_AMP_RANGE.0;
    pre_amp_scaled
}

#[allow(clippy::let_and_return)]
fn calculate_drive(cache: &Cache) -> f32 {
    let drive_sum = (cache.drive_pot + cache.drive_cv).clamp(0.0, 1.0);
    let drive_curved = taper::log(drive_sum);
    let drive_scaled = drive_curved * (DRIVE_RANGE.1 - DRIVE_RANGE.0) + DRIVE_RANGE.0;
    drive_scaled
}

#[allow(clippy::let_and_return)]
fn calculate_saturation(cache: &Cache) -> f32 {
    let saturation_sum = (cache.saturation_pot + cache.saturation_cv).clamp(0.0, 1.0);
    let saturation_curved = taper::reverse_log(saturation_sum);
    let saturation_scaled =
        saturation_curved * (SATURATION_RANGE.1 - SATURATION_RANGE.0) + SATURATION_RANGE.0;
    saturation_scaled
}

#[allow(clippy::let_and_return)]
fn calculate_bias(cache: &Cache) -> f32 {
    let bias_sum = (cache.bias_pot + cache.bias_cv).clamp(0.0, 1.0);
    let bias_curved = taper::log(bias_sum);
    let bias_scaled = bias_curved * (BIAS_RANGE.1 - BIAS_RANGE.0) + BIAS_RANGE.0;
    bias_scaled
}

#[allow(clippy::let_and_return)]
fn calculate_wow_frequency(cache: &Cache) -> f32 {
    let wow_frequency_sum = (cache.wow_frequency_pot + cache.wow_frequency_cv).clamp(0.0, 1.0);
    let wow_frequency_curved = taper::reverse_log(wow_frequency_sum);
    let wow_frequency_scaled = wow_frequency_curved
        * (WOW_FREQUENCY_RANGE.1 - WOW_FREQUENCY_RANGE.0)
        + WOW_FREQUENCY_RANGE.0;
    wow_frequency_scaled
}

#[allow(clippy::let_and_return)]
fn calculate_wow_depth(cache: &Cache, wow_frequency: f32) -> f32 {
    let wow_depth_sum = (cache.wow_depth_pot + cache.wow_depth_cv).clamp(0.0, 1.0);
    let wow_depth_curved = taper::log(wow_depth_sum);
    let max_depth = f32::min(WOW_DEPTH_RANGE.1, (1.0 / wow_frequency) * 0.31);
    let wow_depth_scaled = wow_depth_curved * (max_depth - WOW_DEPTH_RANGE.0) + WOW_DEPTH_RANGE.0;
    wow_depth_scaled
}

#[allow(clippy::let_and_return)]
fn calculate_delay_length(cache: &Cache) -> f32 {
    let delay_length_sum = (cache.delay_length_pot + cache.delay_length_cv).clamp(0.0, 1.0);
    let delay_length_curved = taper::reverse_log(delay_length_sum);
    let delay_length_scaled =
        delay_length_curved * (DELAY_LENGTH_RANGE.1 - DELAY_LENGTH_RANGE.0) + DELAY_LENGTH_RANGE.0;
    delay_length_scaled
}

#[allow(clippy::let_and_return)]
fn calculate_delay_head_position(cache: &Cache, head: usize) -> f32 {
    let delay_head_position_sum =
        (cache.delay_head_position_pot[head] + cache.delay_head_position_cv[head]).clamp(0.0, 1.0);
    let delay_head_position_scaled = delay_head_position_sum
        * (DELAY_HEAD_POSITION_RANGE.1 - DELAY_HEAD_POSITION_RANGE.0)
        + DELAY_HEAD_POSITION_RANGE.0;
    let delay_head_position_quantized = if cache.delay_quantization_6 || cache.delay_quantization_8
    {
        let quantization =
            Quantization::from((cache.delay_quantization_6, cache.delay_quantization_8));
        quantize(delay_head_position_scaled, quantization)
    } else {
        delay_head_position_scaled
    };

    delay_head_position_quantized
}

fn apply_control_action_in_cache(action: ControlAction, cache: &mut Cache) {
    #[allow(clippy::enum_glob_use)]
    use ControlAction::*;
    match action {
        SetPreAmpPot(x) => {
            cache.pre_amp_pot = x;
        }
        SetDrivePot(x) => {
            cache.drive_pot = x;
        }
        SetDriveCV(x) => {
            cache.drive_cv = x;
        }
        SetSaturationPot(x) => {
            cache.saturation_pot = x;
        }
        SetSaturationCV(x) => {
            cache.saturation_cv = x;
        }
        SetBiasPot(x) => {
            cache.bias_pot = x;
        }
        SetBiasCV(x) => {
            cache.bias_cv = x;
        }
        SetWowFrequencyPot(x) => {
            cache.wow_frequency_pot = x;
        }
        SetWowFrequencyCV(x) => {
            cache.wow_frequency_cv = x;
        }
        SetWowDepthPot(x) => {
            cache.wow_depth_pot = x;
        }
        SetWowDepthCV(x) => {
            cache.wow_depth_cv = x;
        }
        SetDelayLengthPot(x) => {
            cache.delay_length_pot = x;
        }
        SetDelayLengthCV(x) => {
            cache.delay_length_cv = x;
        }
        SetDelayHeadPositionPot(i, x) => {
            cache.delay_head_position_pot[i] = x;
        }
        SetDelayHeadPositionCV(i, x) => {
            cache.delay_head_position_cv[i] = x;
        }
        SetDelayQuantizationEight(b) => {
            cache.delay_quantization_8 = b;
        }
        SetDelayQuantizationSix(b) => {
            cache.delay_quantization_6 = b;
        }
        SetDelayHeadPlay(i, b) => {
            cache.delay_head_play[i] = b;
        }
        SetDelayHeadFeedback(i, b) => {
            cache.delay_head_feedback[i] = b;
        }
    }
}
