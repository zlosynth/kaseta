//! Main interface for the DSP loop.

use sirena::memory_manager::MemoryManager;

use crate::dc_blocker::DCBlocker;
use crate::delay::{
    Attributes as DelayAttributes, Delay, HeadAttributes as DelayHeadAttributes,
    Reaction as DelayReaction,
};
use crate::hysteresis::{
    Attributes as HysteresisAttributes, Hysteresis, Reaction as HysteresisReaction,
};
use crate::oscillator::{Attributes as OscillatorAttributes, Oscillator};
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
    oscillator: Oscillator,
    hysteresis: Hysteresis,
    wow_flutter: WowFlutter,
    delay: Delay,
    tone: Tone,
    dc_blocker: DCBlocker,
    first_stage: FirstStage,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum FirstStage {
    PreAmp,
    Oscillator,
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(clippy::struct_excessive_bools)]
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
    pub enable_oscillator: bool,
    pub rewind: bool,
    pub reset_impulse: bool,
    pub random_impulse: bool,
    pub filter_feedback: bool,
    pub rewind_speed: [(f32, f32); 4],
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AttributesHead {
    pub position: f32,
    pub volume: f32,
    pub feedback: f32,
    pub pan: f32,
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
            oscillator: Oscillator::new(fs),
            hysteresis: Hysteresis::new(fs),
            wow_flutter: WowFlutter::new(fs as u32, memory_manager),
            delay: Delay::new(fs, memory_manager),
            tone: Tone::new(fs as u32),
            dc_blocker: DCBlocker::default(),
            first_stage: FirstStage::PreAmp,
        };

        uninitialized_processor.set_attributes(Attributes::default());
        let processor = uninitialized_processor;

        processor
    }

    pub fn process(&mut self, block: &mut [(f32, f32); 32], random: &mut impl Random) -> Reaction {
        let mut reaction = Reaction::default();

        self.dc_blocker.process(&mut block[..]);

        let mut buffer = [0.0; 32];

        match self.first_stage {
            FirstStage::PreAmp => {
                for (i, x) in block.iter().enumerate() {
                    buffer[i] = x.1;
                }
                self.pre_amp.process(&mut buffer);
            }
            FirstStage::Oscillator => {
                self.oscillator.populate(&mut buffer);
            }
        }

        self.wow_flutter.process(&mut buffer, random);

        let mut oversampled_block = [0.0; 32 * 4];
        self.upsampler.process(&buffer, &mut oversampled_block);
        self.hysteresis
            .process(&mut oversampled_block)
            .notify(&mut reaction);
        self.downsampler
            .process(&oversampled_block, &mut buffer[..]);

        self.delay
            .process(&mut buffer[..], &mut block[..], &mut self.tone, random)
            .notify(&mut reaction);

        // TODO: Enabling this causes instability on higher pre-amps
        // self.dc_blocker.process(&mut block[..]);

        // for (i, (l, r)) in block.iter_mut().enumerate() {
        //     *l = buffer[i];
        //     *r = buffer[i];
        // }

        reaction
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.first_stage = if attributes.enable_oscillator {
            FirstStage::Oscillator
        } else {
            FirstStage::PreAmp
        };

        self.pre_amp.set_attributes(attributes.into());
        self.oscillator.set_attributes(&attributes.into());
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

impl From<Attributes> for OscillatorAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            frequency: other.oscillator,
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
            width: 1.0 - other.bias,
        }
    }
}

impl From<Attributes> for WowFlutterAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            wow_depth: other.wow,
            flutter_depth: other.flutter_depth,
            flutter_chance: other.flutter_chance,
        }
    }
}

impl From<Attributes> for DelayAttributes {
    // TODO: Use rewind speed from other
    fn from(other: Attributes) -> Self {
        Self {
            length: other.speed,
            heads: [
                DelayHeadAttributes {
                    position: other.head[0].position,
                    volume: other.head[0].volume,
                    feedback: other.head[0].feedback,
                    pan: other.head[0].pan,
                    // rewind_forward: other.rewind.then_some(-0.25),
                    // rewind_backward: other.rewind.then_some(0.25),
                    rewind_forward: other.rewind.then_some(other.rewind_speed[0].1),
                    rewind_backward: other.rewind.then_some(other.rewind_speed[0].0),
                },
                DelayHeadAttributes {
                    position: other.head[1].position,
                    volume: other.head[1].volume,
                    feedback: other.head[1].feedback,
                    pan: other.head[1].pan,
                    // NOTE: Slightly detuned (not 0.125) to avoid ticking when crossing samples (?)
                    // rewind_forward: other.rewind.then_some(-0.124),
                    // rewind_backward: other.rewind.then_some(0.124),
                    rewind_forward: other.rewind.then_some(other.rewind_speed[1].1),
                    rewind_backward: other.rewind.then_some(other.rewind_speed[1].0),
                },
                DelayHeadAttributes {
                    position: other.head[2].position,
                    volume: other.head[2].volume,
                    feedback: other.head[2].feedback,
                    pan: other.head[2].pan,
                    // rewind_forward: other.rewind.then_some(-1.0),
                    // rewind_backward: other.rewind.then_some(2.0),
                    rewind_forward: other.rewind.then_some(other.rewind_speed[2].1),
                    rewind_backward: other.rewind.then_some(other.rewind_speed[2].0),
                },
                DelayHeadAttributes {
                    position: other.head[3].position,
                    volume: other.head[3].volume,
                    feedback: other.head[3].feedback,
                    pan: other.head[3].pan,
                    // rewind_forward: other.rewind.then_some(-0.124),
                    // rewind_backward: other.rewind.then_some(0.124),
                    rewind_forward: other.rewind.then_some(other.rewind_speed[3].1),
                    rewind_backward: other.rewind.then_some(other.rewind_speed[3].0),
                },
            ],
            reset_impulse: other.reset_impulse,
            random_impulse: other.random_impulse,
            filter_feedback: other.filter_feedback,
        }
    }
}

impl HysteresisReaction {
    fn notify(&mut self, reaction: &mut Reaction) {
        reaction.hysteresis_clipping = self.clipping;
    }
}

impl DelayReaction {
    fn notify(&mut self, reaction: &mut Reaction) {
        reaction.delay_impulse = self.impulse;
    }
}
