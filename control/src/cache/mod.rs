pub mod calibration;
mod clock_detector;
pub mod configuration;
pub mod display;
mod interval_detector;
mod led;
pub mod mapping;
mod quantization;
mod reconcile;
mod tap_detector;
mod trigger;

use heapless::Vec;
use kaseta_dsp::processor::{Attributes as DSPAttributes, AttributesHead as DSPAttributesHead};

use self::calibration::Calibration;
use self::clock_detector::ClockDetector;
pub use self::configuration::Configuration;
use self::display::Display;
use self::led::Led;
use self::mapping::{AttributeIdentifier, Mapping};
use self::tap_detector::TapDetector;
use self::trigger::Trigger;
use crate::log;
use crate::output::DesiredOutput;
use crate::save::Save;

/// Cache keeping internal attributes.
///
/// This information should be sufficient to build attributes for DSP
/// and hold state for output peripherals.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Cache {
    pub mapping: Mapping,
    pub calibrations: Calibrations,
    pub options: Options,
    pub configuration: Configuration,
    pub clock_detectors: ClockDetectors,
    pub tap_detector: TapDetector,
    pub tapped_tempo: TappedTempo,
    pub attributes: Attributes,
    pub impulse_trigger: Trigger,
    pub impulse_led: Led,
    pub display: Display,
}

/// Storing calibration settings of all four inputs.
pub type Calibrations = [Calibration; 4];

/// Clock signal detectors of each control input.
pub type ClockDetectors = [ClockDetector; 4];

/// Easy to access modifications of the default module behavior.
///
/// Options are set by holding the button while turning module's pots.
/// They are meant to provide a quick access to some common extended features
/// such as quantization, rewind, etc.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Options {
    pub quantize_8: bool,
    pub quantize_6: bool,
    pub delay_range: DelayRange,
    pub rewind: bool,
    pub enable_oscillator: bool,
    pub random_impulse: bool,
    pub filter_feedback: bool,
    pub wow_flutter_placement: WowFlutterPlacement,
    pub unlimited: bool,
}

/// Range of the delay time.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DelayRange {
    Long,
    Short,
    Audio,
}

impl Default for DelayRange {
    fn default() -> Self {
        Self::Long
    }
}

/// Placement of the time effects of wow and flutter.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WowFlutterPlacement {
    Input,
    Read,
    Both,
}

impl Default for WowFlutterPlacement {
    fn default() -> Self {
        Self::Both
    }
}

impl WowFlutterPlacement {
    pub fn is_input(self) -> bool {
        matches!(self, Self::Input)
    }

    pub fn is_read(self) -> bool {
        matches!(self, Self::Read)
    }
}

/// Storing tempo if it was tapped in using the button.
pub type TappedTempo = Option<f32>;

/// Interpreted attributes for the DSP.
///
/// This structure can be directly translated to DSP configuration, used
/// to build the audio processor model.
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub pre_amp: f32,
    pub oscillator: f32,
    pub drive: f32,
    pub saturation: f32,
    pub bias: f32,
    pub dry_wet: f32,
    pub wow: f32,
    pub flutter_depth: f32,
    pub flutter_chance: f32,
    pub speed: f32,
    pub tone: f32,
    pub head: [AttributesHead; 4],
    pub reset_impulse: bool,
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AttributesHead {
    pub position: f32,
    pub volume: f32,
    pub feedback: f32,
    pub pan: f32,
}

impl Cache {
    pub fn build_dsp_attributes(&mut self) -> DSPAttributes {
        DSPAttributes {
            pre_amp: self.attributes.pre_amp,
            oscillator: self.attributes.oscillator,
            drive: self.attributes.drive,
            saturation: self.attributes.saturation,
            bias: self.attributes.bias,
            dry_wet: self.attributes.dry_wet,
            wow: self.attributes.wow,
            flutter_depth: self.attributes.flutter_depth,
            flutter_chance: self.attributes.flutter_chance,
            speed: self.attributes.speed,
            tone: self.attributes.tone,
            head: [
                DSPAttributesHead {
                    position: self.attributes.head[0].position,
                    volume: self.attributes.head[0].volume,
                    feedback: self.attributes.head[0].feedback,
                    pan: self.attributes.head[0].pan,
                },
                DSPAttributesHead {
                    position: self.attributes.head[1].position,
                    volume: self.attributes.head[1].volume,
                    feedback: self.attributes.head[1].feedback,
                    pan: self.attributes.head[1].pan,
                },
                DSPAttributesHead {
                    position: self.attributes.head[2].position,
                    volume: self.attributes.head[2].volume,
                    feedback: self.attributes.head[2].feedback,
                    pan: self.attributes.head[2].pan,
                },
                DSPAttributesHead {
                    position: self.attributes.head[3].position,
                    volume: self.attributes.head[3].volume,
                    feedback: self.attributes.head[3].feedback,
                    pan: self.attributes.head[3].pan,
                },
            ],
            rewind: self.options.rewind,
            enable_oscillator: self.options.enable_oscillator,
            rewind_speed: self.configuration.rewind_speed(),
            reset_impulse: self.attributes.reset_impulse,
            random_impulse: self.options.random_impulse,
            filter_feedback: self.options.filter_feedback,
            wow_flutter_placement: if self.options.wow_flutter_placement.is_input() {
                0
            } else if self.options.wow_flutter_placement.is_read() {
                1
            } else {
                2
            },
        }
    }

    pub fn save(&self) -> Save {
        Save {
            mapping: self.mapping,
            calibrations: self.calibrations,
            options: self.options,
            configuration: self.configuration,
            tapped_tempo: self.tapped_tempo,
        }
    }

    pub fn tick(&mut self) -> DesiredOutput {
        let output = DesiredOutput {
            display: self.display.active_screen().leds(),
            impulse_trigger: self.impulse_trigger.triggered(),
            impulse_led: self.impulse_led.triggered(),
        };

        self.impulse_trigger.tick();
        self.impulse_led.tick();
        self.display.tick();

        self.tap_detector.tick();
        self.clock_detectors.iter_mut().for_each(|d| d.tick());

        output
    }

    pub fn unmap_controls(&mut self, unplugged_controls: &Vec<usize, 4>, needs_save: &mut bool) {
        for i in unplugged_controls {
            let attribute = self.mapping[*i];
            if !attribute.is_none() {
                log::info!(
                    "Unmapping attribute={:?} from control={:?}",
                    attribute,
                    *i + 1
                );
                self.mapping[*i] = AttributeIdentifier::None;
                *needs_save |= true;
            }
        }
    }
}
