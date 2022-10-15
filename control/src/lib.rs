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

mod delay;
mod hysteresis;
mod quantization;
mod taper;
mod wow;

use kaseta_dsp::processor::Attributes;

use crate::delay::Cache as DelayCache;
use crate::hysteresis::Cache as HysteresisCache;
use crate::wow::Cache as WowCache;

// Pre-amp scales between -20 to +28 dB.
const PRE_AMP_RANGE: (f32, f32) = (0.0, 25.0);

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ControlAction {
    SetPreAmpPot(f32),
    SetDryWetPot(f32),
    SetDrivePot(f32),
    SetDriveCV(f32),
    SetBiasPot(f32),
    SetBiasCV(f32),
    SetWowFrequencyPot(f32),
    SetWowFrequencyCV(f32),
    SetWowDepthPot(f32),
    SetWowDepthCV(f32),
    SetWowFilterPot(f32),
    SetWowAmplitudeNoisePot(f32),
    SetWowAmplitudeSpringPot(f32),
    SetWowPhaseNoisePot(f32),
    SetWowPhaseSpringPot(f32),
    SetWowPhaseDriftPot(f32),
    SetDelayLengthPot(f32),
    SetDelayLengthCV(f32),
    SetDelayHeadPositionPot(usize, f32),
    SetDelayHeadPositionCV(usize, f32),
    SetDelayQuantizationSix(bool),
    SetDelayQuantizationEight(bool),
    SetDelayHeadFeedbackAmount(usize, f32),
    SetDelayHeadVolume(usize, f32),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DSPReaction {
    pub pre_amp: f32,
    pub dry_wet: f32,
    pub drive: f32,
    pub saturation: f32,
    pub bias: f32,
    pub wow_frequency: f32,
    pub wow_depth: f32,
    pub wow_filter: f32,
    pub wow_amplitude_noise: f32,
    pub wow_amplitude_spring: f32,
    pub wow_phase_noise: f32,
    pub wow_phase_spring: f32,
    pub wow_phase_drift: f32,
    pub delay_length: f32,
    pub delay_head_position: [f32; 4],
    pub delay_head_feedback_amount: [f32; 4],
    pub delay_head_volume: [f32; 4],
}

impl From<DSPReaction> for Attributes {
    fn from(other: DSPReaction) -> Self {
        Attributes {
            pre_amp: other.pre_amp,
            dry_wet: other.dry_wet,
            drive: other.drive,
            saturation: other.saturation,
            width: 1.0 - other.bias,
            wow_frequency: other.wow_frequency,
            wow_depth: other.wow_depth,
            wow_filter: other.wow_filter,
            wow_amplitude_noise: other.wow_amplitude_noise,
            wow_amplitude_spring: other.wow_amplitude_spring,
            wow_phase_noise: other.wow_phase_noise,
            wow_phase_spring: other.wow_phase_spring,
            wow_phase_drift: other.wow_phase_drift,
            delay_length: other.delay_length,
            delay_head_1_position: other.delay_head_position[0],
            delay_head_2_position: other.delay_head_position[1],
            delay_head_3_position: other.delay_head_position[2],
            delay_head_4_position: other.delay_head_position[3],
            delay_head_1_play: other.delay_head_volume[0] > 0.01,
            delay_head_2_play: other.delay_head_volume[1] > 0.01,
            delay_head_3_play: other.delay_head_volume[2] > 0.01,
            delay_head_4_play: other.delay_head_volume[3] > 0.01,
            delay_head_1_feedback: other.delay_head_feedback_amount[0] > 0.01,
            delay_head_2_feedback: other.delay_head_feedback_amount[1] > 0.01,
            delay_head_3_feedback: other.delay_head_feedback_amount[2] > 0.01,
            delay_head_4_feedback: other.delay_head_feedback_amount[3] > 0.01,
            delay_head_1_feedback_amount: other.delay_head_feedback_amount[0],
            delay_head_2_feedback_amount: other.delay_head_feedback_amount[1],
            delay_head_3_feedback_amount: other.delay_head_feedback_amount[2],
            delay_head_4_feedback_amount: other.delay_head_feedback_amount[3],
            delay_head_1_volume: other.delay_head_volume[0],
            delay_head_2_volume: other.delay_head_volume[1],
            delay_head_3_volume: other.delay_head_volume[2],
            delay_head_4_volume: other.delay_head_volume[3],
        }
    }
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub pre_amp_pot: f32,
    pub hysteresis: HysteresisCache,
    pub wow: WowCache,
    pub delay: DelayCache,
}

#[must_use]
pub fn reduce_control_action(action: ControlAction, cache: &mut Cache) -> DSPReaction {
    apply_control_action_in_cache(action, cache);
    cook_dsp_reaction_from_cache(cache)
}

#[must_use]
pub fn cook_dsp_reaction_from_cache(cache: &Cache) -> DSPReaction {
    let pre_amp = calculate_pre_amp(cache);
    let dry_wet = hysteresis::calculate_dry_wet(&cache.hysteresis);
    let (drive, saturation) = hysteresis::calculate_drive_and_saturation(&cache.hysteresis);
    let bias = hysteresis::calculate_bias(&cache.hysteresis, drive);
    let wow_frequency = wow::calculate_frequency(&cache.wow);
    let wow_depth = wow::calculate_depth(&cache.wow, wow_frequency);
    let wow_amplitude_noise = wow::calculate_amplitude_noise(&cache.wow);
    let wow_amplitude_spring = wow::calculate_amplitude_spring(&cache.wow);
    let wow_phase_noise = wow::calculate_phase_noise(&cache.wow);
    let wow_phase_spring = wow::calculate_phase_spring(&cache.wow);
    let wow_phase_drift = wow::calculate_phase_drift(&cache.wow);
    let wow_filter = wow::calculate_filter(&cache.wow);
    let delay_length = delay::calculate_length(&cache.delay);
    let delay_head_1_position = delay::calculate_head_position(&cache.delay, 0);
    let delay_head_2_position = delay::calculate_head_position(&cache.delay, 1);
    let delay_head_3_position = delay::calculate_head_position(&cache.delay, 2);
    let delay_head_4_position = delay::calculate_head_position(&cache.delay, 3);
    DSPReaction {
        pre_amp,
        dry_wet,
        drive,
        saturation,
        bias,
        wow_frequency,
        wow_depth,
        wow_amplitude_noise,
        wow_amplitude_spring,
        wow_phase_noise,
        wow_phase_spring,
        wow_phase_drift,
        wow_filter,
        delay_length,
        delay_head_position: [
            delay_head_1_position,
            delay_head_2_position,
            delay_head_3_position,
            delay_head_4_position,
        ],
        delay_head_feedback_amount: cache.delay.head_feedback_amount,
        delay_head_volume: cache.delay.head_volume,
    }
}

#[allow(clippy::let_and_return)]
pub(crate) fn calculate(
    pot: Option<f32>,
    cv: Option<f32>,
    range: (f32, f32),
    taper_function: Option<fn(f32) -> f32>,
) -> f32 {
    let sum = (pot.unwrap_or(0.0) + cv.unwrap_or(0.0)).clamp(0.0, 1.0);
    let curved = if let Some(taper_function) = taper_function {
        taper_function(sum)
    } else {
        sum
    };
    let scaled = curved * (range.1 - range.0) + range.0;
    scaled
}

#[allow(clippy::let_and_return)]
fn calculate_pre_amp(cache: &Cache) -> f32 {
    calculate(
        Some(cache.pre_amp_pot),
        None,
        PRE_AMP_RANGE,
        Some(taper::log),
    )
}

fn apply_control_action_in_cache(action: ControlAction, cache: &mut Cache) {
    #[allow(clippy::enum_glob_use)]
    use ControlAction::*;
    match action {
        SetPreAmpPot(x) => {
            cache.pre_amp_pot = x;
        }
        SetDryWetPot(x) => {
            cache.hysteresis.dry_wet_pot = x;
        }
        SetDrivePot(x) => {
            cache.hysteresis.drive_pot = x;
        }
        SetDriveCV(x) => {
            cache.hysteresis.drive_cv = x;
        }
        SetBiasPot(x) => {
            cache.hysteresis.bias_pot = x;
        }
        SetBiasCV(x) => {
            cache.hysteresis.bias_cv = x;
        }
        SetWowFrequencyPot(x) => {
            cache.wow.frequency_pot = x;
        }
        SetWowFrequencyCV(x) => {
            cache.wow.frequency_cv = x;
        }
        SetWowDepthPot(x) => {
            cache.wow.depth_pot = x;
        }
        SetWowDepthCV(x) => {
            cache.wow.depth_cv = x;
        }
        SetWowAmplitudeNoisePot(x) => {
            cache.wow.amplitude_noise_pot = x;
        }
        SetWowAmplitudeSpringPot(x) => {
            cache.wow.amplitude_spring_pot = x;
        }
        SetWowPhaseNoisePot(x) => {
            cache.wow.phase_noise_pot = x;
        }
        SetWowPhaseSpringPot(x) => {
            cache.wow.phase_spring_pot = x;
        }
        SetWowPhaseDriftPot(x) => {
            cache.wow.phase_drift_pot = x;
        }
        SetWowFilterPot(x) => {
            cache.wow.filter_pot = x;
        }
        SetDelayLengthPot(x) => {
            cache.delay.length_pot = x;
        }
        SetDelayLengthCV(x) => {
            cache.delay.length_cv = x;
        }
        SetDelayHeadPositionPot(i, x) => {
            cache.delay.head_position_pot[i] = x;
        }
        SetDelayHeadPositionCV(i, x) => {
            cache.delay.head_position_cv[i] = x;
        }
        SetDelayQuantizationEight(b) => {
            cache.delay.quantization_8 = b;
        }
        SetDelayQuantizationSix(b) => {
            cache.delay.quantization_6 = b;
        }
        SetDelayHeadFeedbackAmount(i, x) => {
            cache.delay.head_feedback_amount[i] = x;
        }
        SetDelayHeadVolume(i, x) => {
            cache.delay.head_volume[i] = x;
        }
    }
}
