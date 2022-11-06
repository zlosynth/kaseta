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
//!           (DSPAction) |   | (DSPReaction)
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

use kaseta_dsp::processor::{Attributes, Reaction as DSPReaction};

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
    SetWowDepthPot(f32),
    SetWowDepthCV(f32),
    SetDelayRangeSwitch(bool),
    SetDelayRewindForwardSwitch(bool),
    SetDelayRewindBackwardSwitch(bool),
    SetDelayLengthPot(f32),
    SetDelayLengthCV(f32),
    SetDelayHeadPositionPot(usize, f32),
    SetDelayHeadPositionCV(usize, f32),
    SetDelayQuantizationSix(bool),
    SetDelayQuantizationEight(bool),
    SetDelayHeadFeedbackAmount(usize, f32),
    SetDelayHeadVolume(usize, f32),
    SetTone(f32),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DSPAction {
    pub pre_amp: f32,
    pub dry_wet: f32,
    pub drive: f32,
    pub saturation: f32,
    pub bias: f32,
    pub wow_depth: f32,
    pub delay_length: f32,
    pub delay_rewind_forward: bool,
    pub delay_rewind_backward: bool,
    pub delay_head_position: [f32; 4],
    pub delay_head_feedback: [f32; 4],
    pub delay_head_volume: [f32; 4],
    pub tone: f32,
}

impl From<DSPAction> for Attributes {
    fn from(other: DSPAction) -> Self {
        Attributes {
            pre_amp: other.pre_amp,
            dry_wet: other.dry_wet,
            drive: other.drive,
            saturation: other.saturation,
            width: 1.0 - other.bias,
            wow_depth: other.wow_depth,
            delay_length: other.delay_length,
            delay_rewind_forward: other.delay_rewind_forward,
            delay_rewind_backward: other.delay_rewind_backward,
            delay_head_1_position: other.delay_head_position[0],
            delay_head_2_position: other.delay_head_position[1],
            delay_head_3_position: other.delay_head_position[2],
            delay_head_4_position: other.delay_head_position[3],
            delay_head_1_feedback: other.delay_head_feedback[0],
            delay_head_2_feedback: other.delay_head_feedback[1],
            delay_head_3_feedback: other.delay_head_feedback[2],
            delay_head_4_feedback: other.delay_head_feedback[3],
            delay_head_1_volume: other.delay_head_volume[0],
            delay_head_2_volume: other.delay_head_volume[1],
            delay_head_3_volume: other.delay_head_volume[2],
            delay_head_4_volume: other.delay_head_volume[3],
            tone: other.tone,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub pre_amp_pot: f32,
    pub hysteresis: HysteresisCache,
    pub wow: WowCache,
    pub delay: DelayCache,
    pub tone: f32,
    pub leds: [ImpulseCache; 8],
    pub impulse: ImpulseCache,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            pre_amp_pot: 0.0,
            hysteresis: HysteresisCache::default(),
            wow: WowCache::default(),
            delay: DelayCache::default(),
            tone: 0.0,
            leds: [ImpulseCache::new(100); 8],
            impulse: ImpulseCache::new(100),
        }
    }
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ImpulseCache {
    timeout: u32,
    pub on: Option<u32>,
}

impl ImpulseCache {
    #[must_use]
    pub fn new(timeout: u32) -> Self {
        Self { timeout, on: None }
    }

    #[must_use]
    pub fn is_on(&self) -> bool {
        self.on.is_some()
    }

    pub fn trigger(&mut self) {
        self.on = Some(self.timeout);
    }

    pub fn tick(&mut self) {
        if let Some(countdown) = self.on {
            if countdown == 1 {
                self.on = None;
            } else {
                self.on = Some(countdown - 1);
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Reaction {
    pub leds: [bool; 8],
    pub impulse: bool,
}

// TODO: Reaction is not needed when DSP passed. DSP should just update cache. Another function
// will be used to gather the current state.
// TODO: Impulse should be kept 20 ms. It could be part of cache, and reset after X ticks.
#[must_use]
pub fn reduce_dsp_reaction(reaction: DSPReaction, cache: &mut Cache) -> Reaction {
    cache.impulse.tick();
    cache.leds[0].tick();
    if reaction.hysteresis_clipping {
        cache.leds[0].trigger();
    }
    if reaction.delay_impulse {
        cache.impulse.trigger();
    }
    let mut leds = [false; 8];
    leds[0] = cache.leds[0].is_on();
    Reaction {
        leds,
        impulse: cache.impulse.is_on(),
    }
}

#[must_use]
pub fn reduce_control_action(action: ControlAction, cache: &mut Cache) -> DSPAction {
    apply_control_action_in_cache(action, cache);
    cook_dsp_reaction_from_cache(cache)
}

#[must_use]
pub fn cook_dsp_reaction_from_cache(cache: &Cache) -> DSPAction {
    let pre_amp = calculate_pre_amp(cache);
    let dry_wet = hysteresis::calculate_dry_wet(&cache.hysteresis);
    let (drive, saturation) = hysteresis::calculate_drive_and_saturation(&cache.hysteresis);
    let bias = hysteresis::calculate_bias(&cache.hysteresis, drive);
    let wow_depth = wow::calculate_depth(&cache.wow);
    let delay_length = delay::calculate_length(&cache.delay);
    let delay_head_1_position = delay::calculate_head_position(&cache.delay, 0);
    let delay_head_2_position = delay::calculate_head_position(&cache.delay, 1);
    let delay_head_3_position = delay::calculate_head_position(&cache.delay, 2);
    let delay_head_4_position = delay::calculate_head_position(&cache.delay, 3);
    DSPAction {
        pre_amp,
        dry_wet,
        drive,
        saturation,
        bias,
        wow_depth,
        delay_length,
        delay_rewind_forward: cache.delay.rewind_forward_switch,
        delay_rewind_backward: cache.delay.rewind_backward_switch,
        delay_head_position: [
            delay_head_1_position,
            delay_head_2_position,
            delay_head_3_position,
            delay_head_4_position,
        ],
        delay_head_feedback: cache.delay.head_feedback,
        delay_head_volume: cache.delay.head_volume,
        tone: cache.tone,
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
        SetWowDepthPot(x) => {
            cache.wow.depth_pot = x;
        }
        SetWowDepthCV(x) => {
            cache.wow.depth_cv = x;
        }
        SetDelayRangeSwitch(b) => {
            cache.delay.range_switch = b;
        }
        SetDelayRewindForwardSwitch(b) => {
            cache.delay.rewind_forward_switch = b;
        }
        SetDelayRewindBackwardSwitch(b) => {
            cache.delay.rewind_backward_switch = b;
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
            cache.delay.head_feedback[i] = x;
        }
        SetDelayHeadVolume(i, x) => {
            cache.delay.head_volume[i] = x;
        }
        SetTone(x) => {
            cache.tone = x;
        }
    }
}
