#[allow(unused_imports)]
use micromath::F32Ext as _;

use crate::math;
use crate::random::Random;
use crate::ring_buffer::RingBuffer;
use sirena::memory_manager::MemoryManager;

const MAX_LENGTH: f32 = 2.0 * 60.0;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Delay {
    sample_rate: f32,
    buffer: RingBuffer,
    heads: [Head; 4],
    length: f32,
    impulse_cursor: f32,
    random_impulse: bool,
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
        input_buffer: &[f32],
        output_buffer: &mut [(f32, f32)],
        random: &mut impl Random,
    ) -> bool {
        for x in input_buffer.iter() {
            self.buffer.write(*x);
        }

        let buffer_len = output_buffer.len();
        for (i, x) in output_buffer.iter_mut().enumerate() {
            // NOTE: Must read from back, so heads can move from old to new
            let age = buffer_len - i;

            let feedback: f32 = self
                .heads
                .iter_mut()
                .map(|head| head.reader.read(&self.buffer, age) * head.feedback)
                .sum();
            *self.buffer.peek_mut(age) += feedback;

            // NOTE: Must read again now when feedback was written back
            let mut left = 0.0;
            let mut right = 0.0;
            for head in &mut self.heads {
                let value = head.reader.read(&self.buffer, age);
                let amplified = value * head.volume;
                left += amplified * (1.0 - head.pan);
                right += amplified * head.pan;
            }

            *x = (left, right);
        }

        let initial_impulse_cursor = self.impulse_cursor;
        self.impulse_cursor += input_buffer.len() as f32 / self.sample_rate as f32;
        while self.impulse_cursor > self.length {
            self.impulse_cursor -= self.length;
        }

        let mut impulse = false;
        for head in &self.heads {
            if head.volume < 0.01 {
                continue;
            }
            let impulse_position = head.reader.impulse_position() / self.sample_rate;
            let head_impulse = if initial_impulse_cursor > self.impulse_cursor {
                impulse_position >= initial_impulse_cursor || impulse_position < self.impulse_cursor
            } else {
                initial_impulse_cursor <= impulse_position && impulse_position < self.impulse_cursor
            };
            let chance = if self.random_impulse {
                dice_to_bool(random.normal(), head.volume)
            } else {
                true
            };
            impulse |= head_impulse && chance;
        }
        impulse
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        if attributes.reset_impulse {
            self.impulse_cursor = 0.0;
        }
        self.random_impulse = attributes.random_impulse;

        self.length = attributes.length;
        for (i, head) in self.heads.iter_mut().enumerate() {
            head.feedback = attributes.heads[i].feedback;
            head.volume = attributes.heads[i].volume;
            head.pan = attributes.heads[i].pan;
            head.reader.set_attributes(&FractionalDelayAttributes {
                position: self.length * attributes.heads[i].position * self.sample_rate,
                rewind_forward: attributes.heads[i].rewind_forward,
                rewind_backward: attributes.heads[i].rewind_backward,
                blend_steps: 3200, // TODO: Make sure it is never higher than buffer size passed to process, it must be also dividable by buffer size
            });
        }
    }
}

fn dice_to_bool(random: f32, chance: f32) -> bool {
    random + chance > 1.0
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FractionalDelay {
    pointer: f32,
    state: State,
}

impl FractionalDelay {
    #[must_use]
    pub fn impulse_position(&self) -> f32 {
        // TODO: Use the target immediatelly with blend
        self.pointer
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    Rewinding(StateRewinding),
    Blending(StateBlending),
    Stable,
}

impl Default for State {
    fn default() -> Self {
        Self::Stable
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StateRewinding {
    pub relative_speed: f32,
    pub target_position: f32,
    pub rewind_speed: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StateBlending {
    pub target: f32,
    pub current_volume: f32,
    pub target_volume: f32,
    pub step: f32,
    pub done: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FractionalDelayAttributes {
    pub position: f32,
    pub rewind_forward: Option<f32>,
    pub rewind_backward: Option<f32>,
    pub blend_steps: usize,
}

// NOTE: Rewind is moving to the target in a steady pace. Fading is going there
// instantly, fading between the current and the destination.
impl FractionalDelay {
    pub fn read(&mut self, buffer: &RingBuffer, offset: usize) -> f32 {
        match &mut self.state {
            State::Stable => buffer.peek(self.pointer as usize + offset),
            State::Rewinding(StateRewinding {
                ref mut relative_speed,
                target_position,
                rewind_speed,
            }) => {
                let a = buffer.peek(self.pointer as usize + offset);
                let b = buffer.peek(self.pointer as usize + 1 + offset);
                let x = a + (b - a) * self.pointer.fract();

                self.pointer += *relative_speed;

                if has_crossed_target(self.pointer, *target_position, *rewind_speed) {
                    self.pointer = *target_position;
                } else {
                    reflect_inertia_on_relative_speed(
                        relative_speed,
                        self.pointer,
                        *target_position,
                        *rewind_speed,
                    );
                }

                x
            }
            State::Blending(StateBlending {
                target,
                current_volume,
                target_volume,
                step,
                done,
            }) => {
                let x = buffer.peek(self.pointer as usize + offset);
                let y = buffer.peek(*target as usize + offset);
                let out = x * *current_volume + y * *target_volume;

                if target_volume.relative_eq(1.0, 0.0001) {
                    self.pointer = *target;
                    *done = true;
                } else {
                    debug_assert!(
                        *target_volume < 1.0,
                        "Make sure that number of steps is divisible by buffer length",
                    );
                    *current_volume -= *step;
                    *target_volume += *step;
                }

                out
            }
        }
    }

    // NOTE: This must be called every 32 or so reads, to assure that the right
    // state is entered. This is to keep state re-calculation outside reads.
    // XXX: For this to work, `set_attributes` must be called every buffer.
    pub fn set_attributes(&mut self, attributes: &FractionalDelayAttributes) {
        let distance_to_target = (attributes.position - self.pointer).abs();
        if distance_to_target.is_zero() {
            self.state = State::Stable;
            return;
        }

        let travelling_forward = attributes.position < self.pointer;
        let rewind_config = if travelling_forward {
            attributes.rewind_forward
        } else {
            attributes.rewind_backward
        };
        if let Some(rewind_speed) = rewind_config {
            self.state = if let State::Rewinding(state) = self.state {
                State::Rewinding(StateRewinding {
                    target_position: attributes.position,
                    rewind_speed,
                    ..state
                })
            } else {
                State::Rewinding(StateRewinding {
                    relative_speed: 0.0,
                    target_position: attributes.position,
                    rewind_speed,
                })
            };
        } else {
            self.state = if let State::Blending(state) = self.state {
                if state.done {
                    State::Blending(StateBlending {
                        target: attributes.position,
                        current_volume: 1.0,
                        target_volume: 0.0,
                        step: 1.0 / attributes.blend_steps as f32,
                        done: false,
                    })
                } else {
                    State::Blending(state)
                }
            } else {
                State::Blending(StateBlending {
                    target: attributes.position,
                    current_volume: 1.0,
                    target_volume: 0.0,
                    step: 1.0 / attributes.blend_steps as f32,
                    done: false,
                })
            };
        }
    }
}

fn has_crossed_target(current_position: f32, target_position: f32, rewind_speed: f32) -> bool {
    rewind_speed.is_sign_positive() && current_position > target_position
        || rewind_speed.is_sign_negative() && current_position < target_position
}

// TODO: The acceleration speed should depend on total size of the delay
fn reflect_inertia_on_relative_speed(
    relative_speed: &mut f32,
    current_position: f32,
    target_position: f32,
    rewind_speed: f32,
) {
    let distance_to_target = (target_position - current_position).abs();
    if distance_to_target < 0.1 * 48_000.0 {
        let acceleration =
            relative_speed.signum() * relative_speed.pow2() / (2.0 * distance_to_target + 1.0);
        *relative_speed -= acceleration;
    } else if rewind_speed.is_sign_positive() && *relative_speed < rewind_speed {
        *relative_speed += 0.00001;
    } else if rewind_speed.is_sign_negative() && *relative_speed > rewind_speed {
        *relative_speed -= 0.00001;
    }
}

trait F32Ext {
    fn pow2(self) -> Self;
    fn signum(self) -> Self;
    fn is_zero(&self) -> bool;
    fn relative_eq(self, other: f32, epsilon: f32) -> bool;
}

impl F32Ext for f32 {
    fn signum(self) -> f32 {
        if self.is_sign_positive() {
            1.0
        } else {
            -1.0
        }
    }

    fn pow2(self) -> f32 {
        self * self
    }

    fn is_zero(&self) -> bool {
        self.abs() < f32::EPSILON
    }

    fn relative_eq(self, other: f32, epsilon: f32) -> bool {
        (self - other).abs() < epsilon
    }
}
