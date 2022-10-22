#[allow(unused_imports)]
use micromath::F32Ext as _;

use crate::math;
use crate::ring_buffer::RingBuffer;
use sirena::memory_manager::MemoryManager;

const MAX_LENGTH: f32 = 50.0;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Delay {
    sample_rate: f32,
    buffer: RingBuffer,
    heads: [Head; 4],
    length: f32,
}

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Head {
    reader: FractionalDelay,
    feedback: f32,
    volume: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub length: f32,
    pub heads: [HeadAttributes; 4],
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct HeadAttributes {
    pub position: f32,
    pub feedback: f32,
    pub volume: f32,
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
    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter() {
            self.buffer.write(*x);
        }

        let buffer_len = buffer.len();
        for (i, x) in buffer.iter_mut().enumerate() {
            // NOTE: Must read from back, so heads can move from old to new
            let age = buffer_len - i;

            let feedback: f32 = self
                .heads
                .iter_mut()
                .map(|head| head.reader.read(&self.buffer, age) * head.feedback)
                .sum();
            *self.buffer.peek_mut(age) += feedback;

            // NOTE: Must read again now when feedback was written back
            let output: f32 = self
                .heads
                .iter_mut()
                .map(|head| head.reader.read(&self.buffer, age) * head.volume)
                .sum();
            *x = output;
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.length = attributes.length;
        for (i, head) in self.heads.iter_mut().enumerate() {
            head.feedback = attributes.heads[i].feedback;
            head.volume = attributes.heads[i].volume;
            head.reader.set_attributes(&FractionalDelayAttributes {
                position: self.length * attributes.heads[i].position * self.sample_rate,
                rewind_forward: attributes.heads[i].rewind_forward,
                rewind_backward: attributes.heads[i].rewind_backward,
            });
        }
    }
}

// TODO: Implement wrapper over Buffer that will interpolate samples and fade between them when jumps get too far
// <https://www.kvraudio.com/forum/viewtopic.php?t=251962>
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FractionalDelay {
    pointer: f32,
    state: State,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum State {
    Rewinding(StateRewinding),
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

// #[derive(Debug)]
// #[cfg_attr(feature = "defmt", derive(defmt::Format))]
// pub struct StateBlending {
//     pub target: f32,
// }

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FractionalDelayAttributes {
    pub position: f32,
    pub rewind_forward: Option<f32>,
    pub rewind_backward: Option<f32>,
}

// NOTE: Rewind is moving to the target in a steady pace. Fading is going there
// instantly, fading between the current and the destination.
impl FractionalDelay {
    pub fn read(&mut self, buffer: &RingBuffer, offset: usize) -> f32 {
        let a = buffer.peek(self.pointer as usize + offset);
        let b = buffer.peek(self.pointer as usize + 1 + offset);
        let x = a + (b - a) * self.pointer.fract();

        match &mut self.state {
            State::Stable => (),
            State::Rewinding(StateRewinding {
                ref mut relative_speed,
                target_position,
                rewind_speed,
            }) => {
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
            }
        }

        x
    }

    // NOTE: This must be called every 32 or so reads, to assure that the right
    // state is entered. This is to keep state re-calculation outside reads.
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
            // TODO: Blend
            self.pointer = attributes.position;
            self.state = State::Stable;
        }
    }
}

fn has_crossed_target(current_position: f32, target_position: f32, rewind_speed: f32) -> bool {
    rewind_speed.is_sign_positive() && current_position > target_position
        || rewind_speed.is_sign_negative() && current_position < target_position
}

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
}
