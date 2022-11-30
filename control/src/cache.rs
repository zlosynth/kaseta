use crate::calibration::Calibration;
use crate::display::Display;
use crate::led::Led;
use crate::mapping::Mapping;
use crate::trigger::Trigger;

/// TODO docs
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub mapping: Mapping,
    pub calibrations: Calibrations,
    pub options: Options,
    pub configuration: Configuration,
    pub tapped_tempo: TappedTempo,
    pub attributes: Attributes,
    pub impulse_trigger: Trigger,
    pub impulse_led: Led,
    pub display: Display,
}

/// TODO Docs
// TODO: One per head, offset and amplification
pub type Calibrations = [Calibration; 4];

/// Easy to access modifications of the default module behavior.
///
/// Options are configured using the DIP switch on the front of the module.
/// They are meant to provide a quick access to some common extended features
/// such as quantization, rewind, etc.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options {
    pub quantize_8: bool,
    pub quantize_6: bool,
    pub short_delay_range: bool,
    pub rewind: bool,
}

/// Tweaking of the default module configuration.
///
/// This is mean to allow tweaking of some more niche configuration of the
/// module. Unlike with `Options`, the parameters here may be continuous
/// (float) or offer enumeration of variants. An examle of a configuration
/// may be tweaking of head's rewind speed.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Configuration {
    pub rewind_speed: [(f32, f32); 4],
}

/// TODO: doc
pub type TappedTempo = Option<f32>;

/// Interpreted attributes for the DSP.
///
/// This structure can be directly translated to DSP configuration, used
/// to build the audio processor model.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub pre_amp: f32,
    pub drive: f32,
    pub saturation: f32,
    pub bias: f32,
    pub dry_wet: f32,
    pub wow: f32,
    pub flutter: f32,
    pub speed: f32,
    pub tone: f32,
    pub head: [AttributesHead; 4],
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AttributesHead {
    pub position: f32,
    pub volume: f32,
    pub feedback: f32,
    pub pan: f32,
}
