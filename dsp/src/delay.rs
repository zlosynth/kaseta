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

// TODO: Moving slowly from one to another
// TODO: Or fading between with variable speed
// TODO: Implement rewind, can be enabled in either direction.
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

                // TODO: Refactor this, hide the logic as inertia.
                // Check whether the target was just crossed.
                if rewind_speed.is_sign_positive() && self.pointer > *target_position
                    || rewind_speed.is_sign_negative() && self.pointer < *target_position
                {
                    self.pointer = *target_position;
                } else {
                    // Check whether it is time to decelerate.
                    let distance_to_target = (*target_position - self.pointer).abs();
                    if distance_to_target < 0.1 * 48_000.0 {
                        let step =
                            (*relative_speed * *relative_speed) / (2.0 * distance_to_target + 1.0);
                        if relative_speed.is_sign_positive() {
                            *relative_speed -= step;
                        } else {
                            *relative_speed += step;
                        }
                        if relative_speed.is_sign_positive() {
                            *relative_speed = relative_speed.max(f32::EPSILON);
                        } else {
                            *relative_speed = relative_speed.min(-f32::EPSILON);
                        }
                    } else {
                        // Check whether acceleration is needed.
                        if rewind_speed.is_sign_positive() && relative_speed < rewind_speed {
                            *relative_speed += 0.00001;
                        } else if rewind_speed.is_sign_negative() && relative_speed > rewind_speed {
                            *relative_speed -= 0.00001;
                        }
                    }
                }
            }
        }

        x
    }

    // NOTE: This must be called every 32 or so reads, to assure that the right
    // state is entered. This is to keep state re-calculation outside reads.
    pub fn set_attributes(&mut self, attributes: &FractionalDelayAttributes) {
        // TODO: Test that this is really used with rewinding
        let distance_to_target = (attributes.position - self.pointer).abs();
        if is_zero(distance_to_target) {
            self.state = State::Stable;
            return;
        }

        let travelling_forward = attributes.position < self.pointer;

        // TODO: Merge the two
        #[allow(clippy::collapsible_else_if)]
        if travelling_forward {
            if let Some(rewind_speed) = attributes.rewind_forward {
                self.state = if let State::Rewinding(mut state) = self.state {
                    state.target_position = attributes.position;
                    state.rewind_speed = -rewind_speed;
                    State::Rewinding(state)
                } else {
                    State::Rewinding(StateRewinding {
                        relative_speed: 0.0,
                        target_position: attributes.position,
                        rewind_speed: -rewind_speed,
                    })
                };
            } else {
                // TODO: Blend
                self.pointer = attributes.position;
                self.state = State::Stable;
            }
        } else {
            if let Some(rewind_speed) = attributes.rewind_backward {
                self.state = if let State::Rewinding(mut state) = self.state {
                    state.target_position = attributes.position;
                    state.rewind_speed = rewind_speed;
                    State::Rewinding(state)
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
}

fn is_zero(value: f32) -> bool {
    value.abs() < f32::EPSILON
}
