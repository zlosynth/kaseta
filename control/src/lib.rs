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

use kaseta_dsp::processor::Attributes;

mod taper;

// 0.1 is almost clean, and with high pre-amp it still passes through signal.
// 30 is well in the extreme, but still somehow stable.
const DRIVE_RANGE: (f32, f32) = (0.1, 30.0);

const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);

// Width (1.0 - bias) must never reach 1.0, otherwise it panics due to division by zero
const BIAS_RANGE: (f32, f32) = (0.0001, 1.0);

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ControlAction {
    SetDrivePot(f32),
    SetDriveCV(f32),
    SetSaturationPot(f32),
    SetSaturationCV(f32),
    SetBiasPot(f32),
    SetBiasCV(f32),
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DSPReaction {
    pub drive: f32,
    pub saturation: f32,
    pub bias: f32,
}

impl From<DSPReaction> for Attributes {
    fn from(other: DSPReaction) -> Self {
        Attributes {
            drive: other.drive,
            saturation: other.saturation,
            width: 1.0 - other.bias,
        }
    }
}

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

#[must_use]
pub fn reduce_control_action(action: ControlAction, cache: &mut Cache) -> DSPReaction {
    apply_control_action_in_cache(action, cache);
    cook_dsp_reaction_from_cache(cache)
}

#[must_use]
pub fn cook_dsp_reaction_from_cache(cache: &Cache) -> DSPReaction {
    let drive = calculate_drive(cache);
    let saturation = calculate_saturation(cache);
    let bias = calculate_bias(cache);
    DSPReaction {
        drive,
        saturation,
        bias,
    }
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

fn apply_control_action_in_cache(action: ControlAction, cache: &mut Cache) {
    #[allow(clippy::enum_glob_use)]
    use ControlAction::*;
    match action {
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
    }
}
