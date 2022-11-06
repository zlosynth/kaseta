//! Main interface for the DSP loop.

use sirena::memory_manager::MemoryManager;

use crate::delay::{Attributes as DelayAttributes, Delay, HeadAttributes as DelayHeadAttributes};
use crate::hysteresis::{
    Attributes as HysteresisAttributes, Hysteresis, Reaction as HysteresisReaction,
};
use crate::oversampling::{Downsampler4, Upsampler4};
use crate::pre_amp::{Attributes as PreAmpAttributes, PreAmp};
use crate::random::Random;
use crate::tone::{Attributes as ToneAttributes, Tone};
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
    tone: Tone,
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
    pub wow_depth: f32,
    pub flutter_depth: f32,
    pub delay_length: f32,
    pub delay_rewind_forward: bool,
    pub delay_rewind_backward: bool,
    pub delay_head_1_position: f32,
    pub delay_head_2_position: f32,
    pub delay_head_3_position: f32,
    pub delay_head_4_position: f32,
    pub delay_head_1_feedback: f32,
    pub delay_head_2_feedback: f32,
    pub delay_head_3_feedback: f32,
    pub delay_head_4_feedback: f32,
    pub delay_head_1_volume: f32,
    pub delay_head_2_volume: f32,
    pub delay_head_3_volume: f32,
    pub delay_head_4_volume: f32,
    pub tone: f32,
}

// TODO: Just re-use and re-export component's attributes
#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Reaction {
    pub hysteresis_clipping: bool,
    pub delay_impulse: bool,
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
            tone: Tone::new(fs as u32),
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
        reaction.delay_impulse = self.delay.process(&mut block[..]);
        self.tone.process(&mut block[..]);

        reaction
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.pre_amp.set_attributes(attributes.into());
        self.hysteresis.set_attributes(attributes.into());
        self.wow_flutter.set_attributes(attributes.into());
        self.delay.set_attributes(attributes.into());
        self.tone.set_attributes(attributes.into());
    }
}

impl From<Attributes> for PreAmpAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            gain: other.pre_amp,
        }
    }
}

impl From<Attributes> for ToneAttributes {
    fn from(other: Attributes) -> Self {
        Self { tone: other.tone }
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
            wow_depth: other.wow_depth,
            flutter_depth: other.flutter_depth,
        }
    }
}

impl From<Attributes> for DelayAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            length: other.delay_length,
            heads: [
                DelayHeadAttributes {
                    position: other.delay_head_1_position,
                    volume: other.delay_head_1_volume,
                    feedback: other.delay_head_1_feedback,
                    rewind_forward: other.delay_rewind_forward.then_some(-0.25),
                    rewind_backward: other.delay_rewind_backward.then_some(0.25),
                },
                DelayHeadAttributes {
                    position: other.delay_head_2_position,
                    volume: other.delay_head_2_volume,
                    feedback: other.delay_head_2_feedback,
                    // NOTE: Slightly detuned (not 0.125) to avoid ticking when crossing samples (?)
                    rewind_forward: other.delay_rewind_forward.then_some(-0.124),
                    rewind_backward: other.delay_rewind_backward.then_some(0.124),
                },
                DelayHeadAttributes {
                    position: other.delay_head_3_position,
                    volume: other.delay_head_3_volume,
                    feedback: other.delay_head_3_feedback,
                    rewind_forward: other.delay_rewind_forward.then_some(-1.0),
                    rewind_backward: other.delay_rewind_backward.then_some(2.0),
                },
                DelayHeadAttributes {
                    position: other.delay_head_4_position,
                    volume: other.delay_head_4_volume,
                    feedback: other.delay_head_4_feedback,
                    rewind_forward: other.delay_rewind_forward.then_some(-0.124),
                    rewind_backward: other.delay_rewind_backward.then_some(0.124),
                },
            ],
        }
    }
}

impl HysteresisReaction {
    fn notify(&mut self, reaction: &mut Reaction) {
        reaction.hysteresis_clipping = self.clipping;
    }
}
