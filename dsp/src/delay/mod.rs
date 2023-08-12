mod compressor;
mod fractional;

#[allow(unused_imports)]
use micromath::F32Ext as _;

use sirena::memory_manager::MemoryManager;

use crate::dc_blocker::DCBlocker;
use crate::math;
use crate::random::Random;
use crate::ring_buffer::RingBuffer;
use crate::tone::Tone2;
use crate::wow_flutter::WowFlutter;

use self::compressor::Compressor;
use self::fractional::{FractionalDelay, FractionalDelayAttributes};

// Assuming sample rate of 48 kHz, 64 MB memory and f32 samples of 4 bytes,
// the module should hold up to 349 seconds of audio. Rounding down to whole
// minutes and adding some overhead for wow and flutter.
const MAX_LENGTH: f32 = 5.0 * 60.0 + 5.0;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Delay {
    sample_rate: f32,
    buffer: RingBuffer,
    heads: [Head; 4],
    length: f32,
    impulse_cursor: f32,
    random_impulse: bool,
    filter_placement: FilterPlacement,
    wow_flutter_placement: WowFlutterPlacement,
    compressor: [Compressor; 4],
    dc_blocker: [DCBlocker; 4],
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Head {
    reader: FractionalDelay,
    feedback: f32,
    volume: f32,
    pan: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub length: f32,
    pub heads: [HeadAttributes; 4],
    pub reset_impulse: bool,
    pub random_impulse: bool,
    pub filter_placement: FilterPlacement,
    pub wow_flutter_placement: WowFlutterPlacement,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HeadAttributes {
    pub position: f32,
    pub feedback: f32,
    pub volume: f32,
    pub pan: f32,
    pub rewind_forward: Option<f32>,
    pub rewind_backward: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum FilterPlacement {
    Input,
    Feedback,
    Both,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum WowFlutterPlacement {
    Input,
    Read,
    Both,
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Reaction {
    pub impulse: bool,
}

impl Delay {
    /// # Panics
    ///
    /// Panics if there is not enough space in the memory manager to allocate a
    /// buffer of `MAX_LENGTH`.
    pub fn new(sample_rate: f32, memory_manager: &mut MemoryManager) -> Self {
        Self {
            sample_rate,
            buffer: RingBuffer::from(
                memory_manager
                    .allocate(math::upper_power_of_two(
                        (sample_rate * MAX_LENGTH) as usize,
                    ))
                    .unwrap(),
            ),
            heads: [
                Head::default(),
                Head::default(),
                Head::default(),
                Head::default(),
            ],
            length: 0.0,
            impulse_cursor: 0.0,
            random_impulse: false,
            filter_placement: FilterPlacement::default(),
            wow_flutter_placement: WowFlutterPlacement::default(),
            compressor: [
                Compressor::new(sample_rate),
                Compressor::new(sample_rate),
                Compressor::new(sample_rate),
                Compressor::new(sample_rate),
            ],
            dc_blocker: [
                DCBlocker::default(),
                DCBlocker::default(),
                DCBlocker::default(),
                DCBlocker::default(),
            ],
        }
    }

    // IN                     (1) write samples from the input
    // |
    // +--------------------+ (3) feed read samples back to the write
    // W                    |
    // ===================  |
    // R   R     R      R   | (2) read samples from the tape
    // +---+-----+------+---+
    // |
    // OUT                    (4) mix all read samples together and play them back
    pub fn process(
        &mut self,
        input_buffer: &mut [f32],
        output_buffer_left: &mut [f32],
        output_buffer_right: &mut [f32],
        tone: &mut Tone2,
        wow_flutter: &mut WowFlutter,
        random: &mut impl Random,
    ) -> Reaction {
        if self.filter_placement.is_input() {
            tone.tone_1.process(input_buffer);
        }

        let mut wow_flutter_delays = [0.0; 32];
        wow_flutter.populate_delays(&mut wow_flutter_delays[..], random);
        if self.wow_flutter_placement.is_both() {
            for x in &mut wow_flutter_delays {
                *x /= 2.0;
            }
        }

        if self.wow_flutter_placement.is_input() {
            wow_flutter.process(input_buffer, &wow_flutter_delays);
        } else {
            wow_flutter.dry_process(input_buffer);
        }

        for x in input_buffer.iter() {
            self.buffer.write(*x);
        }

        let buffer_len = output_buffer_left.len();
        for (i, (l, r)) in output_buffer_left
            .iter_mut()
            .zip(output_buffer_right)
            .enumerate()
        {
            // NOTE: Must read from back, so heads can move from old to new.
            let age = buffer_len - i;

            let mut offset = age as f32;
            if self.wow_flutter_placement.is_read() {
                offset += wow_flutter_delays[i];
            }

            let mut feedback: f32 = self
                .heads
                .iter_mut()
                .map(|head| head.reader.read(&self.buffer, offset) * head.feedback)
                .enumerate()
                .map(|(i, x)| self.compressor[i].process(self.dc_blocker[i].tick(x)))
                .sum();
            if self.filter_placement.is_feedback() {
                feedback = tone.tone_2.tick(feedback);
            }
            *self.buffer.peek_mut(age) += feedback;

            // NOTE: Must read again now when feedback was written back.
            let mut left = 0.0;
            let mut right = 0.0;
            for head in &mut self.heads {
                let value = head.reader.read(&self.buffer, offset);
                let amplified = value * head.volume;
                left += amplified * (1.0 - head.pan);
                right += amplified * head.pan;
            }

            *l = left;
            *r = right;
        }

        let impulse = self.consider_impulse(input_buffer.len(), random);

        Reaction { impulse }
    }

    fn consider_impulse(&mut self, traversed_samples: usize, random: &mut impl Random) -> bool {
        // NOTE: In case the length gets set to 0, don't send any impulse.
        if self.length < f32::EPSILON {
            return false;
        }

        let initial_impulse_cursor = self.impulse_cursor;
        self.impulse_cursor += traversed_samples as f32 / self.sample_rate;
        while self.impulse_cursor > self.length {
            self.impulse_cursor -= self.length;
        }

        let mut impulse = false;
        for head in &self.heads {
            if head.volume < 0.01 {
                continue;
            }
            let head_position = head.reader.impulse_position() / self.sample_rate;
            let crossed_head = if initial_impulse_cursor > self.impulse_cursor {
                head_position >= initial_impulse_cursor || head_position < self.impulse_cursor
            } else {
                initial_impulse_cursor <= head_position && head_position < self.impulse_cursor
            };
            let chance = if self.random_impulse {
                dice_to_bool(random.normal(), head.volume)
            } else {
                true
            };
            impulse |= crossed_head && chance;
        }

        impulse
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        if attributes.reset_impulse {
            self.impulse_cursor = 0.0;
        }
        self.random_impulse = attributes.random_impulse;
        self.filter_placement = attributes.filter_placement;
        self.wow_flutter_placement = attributes.wow_flutter_placement;

        self.length = attributes.length;
        for (i, head) in self.heads.iter_mut().enumerate() {
            head.feedback = attributes.heads[i].feedback;
            head.volume = attributes.heads[i].volume;
            head.pan = attributes.heads[i].pan;
            head.reader.set_attributes(&FractionalDelayAttributes {
                position: self.length * attributes.heads[i].position * self.sample_rate,
                rewind_forward: attributes.heads[i].rewind_forward,
                rewind_backward: attributes.heads[i].rewind_backward,
                blend_steps: 3200, // XXX: It must be also dividable by buffer size
            });
        }
    }
}

fn dice_to_bool(random: f32, chance: f32) -> bool {
    random + chance > 0.99
}

impl Default for FilterPlacement {
    fn default() -> Self {
        Self::Both
    }
}

impl FilterPlacement {
    fn is_input(self) -> bool {
        matches!(self, Self::Input) || matches!(self, Self::Both)
    }

    fn is_feedback(self) -> bool {
        matches!(self, Self::Feedback) || matches!(self, Self::Both)
    }
}

impl Default for WowFlutterPlacement {
    fn default() -> Self {
        Self::Both
    }
}

impl WowFlutterPlacement {
    fn is_input(self) -> bool {
        matches!(self, Self::Input) || matches!(self, Self::Both)
    }

    fn is_read(self) -> bool {
        matches!(self, Self::Read) || matches!(self, Self::Both)
    }

    fn is_both(self) -> bool {
        matches!(self, Self::Both)
    }
}
