//! Main interface for the DSP loop.

use sirena::memory_manager::MemoryManager;

use crate::delay::{Attributes as DelayAttributes, Delay};
use crate::hysteresis::{
    Attributes as HysteresisAttributes, Hysteresis, Reaction as HysteresisReaction,
};
use crate::oversampling::{Downsampler4, Upsampler4};
use crate::pre_amp::{Attributes as PreAmpAttributes, PreAmp};
use crate::random::Random;
use crate::wow_flutter::{Attributes as WowFlutterAttributes, WowFlutter};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Processor {
    upsampler: Upsampler4,
    downsampler: Downsampler4,
    pre_amp: PreAmp,
    hysteresis: Hysteresis,
    wow_flutter: WowFlutter,
    delay: Delay,
}

// TODO: Just re-use and re-export component's attributes
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(clippy::struct_excessive_bools)]
pub struct Attributes {
    pub pre_amp: f32,
    pub dry_wet: f32,
    pub drive: f32,
    pub saturation: f32,
    pub width: f32,
    pub wow_frequency: f32,
    pub wow_depth: f32,
    pub wow_filter: f32,
    pub wow_amplitude_noise: f32,
    pub wow_amplitude_spring: f32,
    pub wow_phase_noise: f32,
    pub wow_phase_spring: f32,
    pub wow_phase_drift: f32,
    pub delay_length: f32,
    pub delay_head_1_position: f32,
    pub delay_head_2_position: f32,
    pub delay_head_3_position: f32,
    pub delay_head_4_position: f32,
    pub delay_head_1_play: bool,
    pub delay_head_2_play: bool,
    pub delay_head_3_play: bool,
    pub delay_head_4_play: bool,
    pub delay_head_1_feedback: bool,
    pub delay_head_2_feedback: bool,
    pub delay_head_3_feedback: bool,
    pub delay_head_4_feedback: bool,
    pub delay_head_1_feedback_amount: f32,
    pub delay_head_2_feedback_amount: f32,
    pub delay_head_3_feedback_amount: f32,
    pub delay_head_4_feedback_amount: f32,
    pub delay_head_1_volume: f32,
    pub delay_head_2_volume: f32,
    pub delay_head_3_volume: f32,
    pub delay_head_4_volume: f32,
}

// TODO: Just re-use and re-export component's attributes
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Reaction {
    pub hysteresis_clipping: bool,
}

impl Processor {
    #[allow(clippy::let_and_return)]
    #[must_use]
    pub fn new(fs: f32, memory_manager: &mut MemoryManager) -> Self {
        let mut uninitialized_processor = Self {
            upsampler: Upsampler4::new_4(memory_manager),
            downsampler: Downsampler4::new_4(memory_manager),
            pre_amp: PreAmp::new(),
            hysteresis: Hysteresis::new(fs),
            wow_flutter: WowFlutter::new(fs as u32, memory_manager),
            delay: Delay::new(fs, memory_manager),
        };

        uninitialized_processor.set_attributes(Attributes::default());
        let processor = uninitialized_processor;

        processor
    }

    pub fn process(&mut self, block: &mut [f32; 32], random: &mut impl Random) -> Reaction {
        let mut reaction = Reaction::default();

        self.wow_flutter.process(block, random);
        self.pre_amp.process(block);
        let mut oversampled_block = [0.0; 32 * 4];
        self.upsampler.process(block, &mut oversampled_block);
        self.hysteresis
            .process(&mut oversampled_block)
            .notify(&mut reaction);
        self.downsampler.process(&oversampled_block, &mut block[..]);
        self.delay.process(&mut block[..]);

        reaction
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.pre_amp.set_attributes(attributes.into());
        self.hysteresis.set_attributes(attributes.into());
        self.wow_flutter.set_attributes(attributes.into());
        self.delay.set_attributes(attributes.into());
    }
}

impl From<Attributes> for PreAmpAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            gain: other.pre_amp,
        }
    }
}

impl From<Attributes> for HysteresisAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            dry_wet: other.dry_wet,
            drive: other.drive,
            saturation: other.saturation,
            width: other.width,
        }
    }
}

impl From<Attributes> for WowFlutterAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            wow_frequency: other.wow_frequency,
            wow_depth: other.wow_depth,
            wow_filter: other.wow_filter,
            wow_amplitude_noise: other.wow_amplitude_noise,
            wow_amplitude_spring: other.wow_amplitude_spring,
            wow_phase_noise: other.wow_phase_noise,
            wow_phase_spring: other.wow_phase_spring,
            wow_phase_drift: other.wow_phase_drift,
        }
    }
}

impl From<Attributes> for DelayAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            length: other.delay_length,
            head_1_position: other.delay_head_1_position,
            head_2_position: other.delay_head_2_position,
            head_3_position: other.delay_head_3_position,
            head_4_position: other.delay_head_4_position,
            head_1_play: other.delay_head_1_play,
            head_2_play: other.delay_head_2_play,
            head_3_play: other.delay_head_3_play,
            head_4_play: other.delay_head_4_play,
            head_1_feedback: other.delay_head_1_feedback,
            head_2_feedback: other.delay_head_2_feedback,
            head_3_feedback: other.delay_head_3_feedback,
            head_4_feedback: other.delay_head_4_feedback,
            head_1_feedback_amount: other.delay_head_1_feedback_amount,
            head_2_feedback_amount: other.delay_head_2_feedback_amount,
            head_3_feedback_amount: other.delay_head_3_feedback_amount,
            head_4_feedback_amount: other.delay_head_4_feedback_amount,
            head_1_volume: other.delay_head_1_volume,
            head_2_volume: other.delay_head_2_volume,
            head_3_volume: other.delay_head_3_volume,
            head_4_volume: other.delay_head_4_volume,
        }
    }
}

impl HysteresisReaction {
    fn notify(&mut self, reaction: &mut Reaction) {
        reaction.hysteresis_clipping = self.clipping;
    }
}
