pub mod calibration;
pub mod display;
mod led;
pub mod mapping;
mod quantization;
mod reconcile;
mod trigger;

use heapless::Vec;
use kaseta_dsp::processor::{
    Attributes as DSPAttributes, AttributesHead as DSPAttributesHead, Reaction as DSPReaction,
};

use self::calibration::Calibration;
use self::display::{Display, Screen};
use self::led::Led;
use self::mapping::{AttributeIdentifier, Mapping};
use self::trigger::Trigger;
use crate::output::DesiredOutput;
use crate::save::Save;

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
// TODO: Drop Copy
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Configuration {
    pub rewind_speed: [(usize, usize); 4],
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

impl Cache {
    pub fn build_dsp_attributes(&mut self) -> DSPAttributes {
        DSPAttributes {
            pre_amp: self.attributes.pre_amp,
            drive: self.attributes.drive,
            saturation: self.attributes.saturation,
            bias: self.attributes.bias,
            dry_wet: self.attributes.dry_wet,
            wow: self.attributes.wow,
            flutter: self.attributes.flutter,
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
            rewind_speed: rewind_indices_to_speeds(self.configuration.rewind_speed),
        }
    }

    pub fn apply_dsp_reaction(&mut self, dsp_reaction: DSPReaction) {
        if dsp_reaction.delay_impulse {
            self.impulse_trigger.trigger();
            self.impulse_led.trigger();
        }
    }

    pub fn screen_for_heads(&self) -> Screen {
        let ordered_heads = self.heads_ordered_by_position();

        Screen::Heads(
            [
                ordered_heads[0].volume > 0.05,
                ordered_heads[1].volume > 0.05,
                ordered_heads[2].volume > 0.05,
                ordered_heads[3].volume > 0.05,
            ],
            [
                ordered_heads[0].feedback > 0.05,
                ordered_heads[1].feedback > 0.05,
                ordered_heads[2].feedback > 0.05,
                ordered_heads[3].feedback > 0.05,
            ],
        )
    }

    fn heads_ordered_by_position(&self) -> [&AttributesHead; 4] {
        let mut ordered_heads = [
            &self.attributes.head[0],
            &self.attributes.head[1],
            &self.attributes.head[2],
            &self.attributes.head[3],
        ];
        for i in 0..ordered_heads.len() {
            for j in 0..ordered_heads.len() - 1 - i {
                if ordered_heads[j].position > ordered_heads[j + 1].position {
                    ordered_heads.swap(j, j + 1);
                }
            }
        }
        ordered_heads
    }

    pub fn save(&self) -> Save {
        Save {
            mapping: self.mapping,
            calibrations: self.calibrations,
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

        output
    }

    pub fn unmap_controls(&mut self, unplugged_controls: &Vec<usize, 4>) {
        for i in unplugged_controls {
            self.mapping[*i] = AttributeIdentifier::None;
        }
    }
}

fn rewind_indices_to_speeds(x: [(usize, usize); 4]) -> [(f32, f32); 4] {
    let mut speeds = [(0.0, 0.0); 4];
    for (i, indices) in x.iter().enumerate() {
        speeds[i].0 = rewind_index_to_speed(indices.0);
        speeds[i].1 = fast_forward_index_to_speed(indices.1);
    }
    speeds
}

fn fast_forward_index_to_speed(i: usize) -> f32 {
    [1.0, 2.0, 4.0, 8.0][i]
}

fn rewind_index_to_speed(i: usize) -> f32 {
    [0.75, 0.5, -1.0, -4.0][i]
}
