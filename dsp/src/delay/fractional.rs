#[allow(unused_imports)]
use micromath::F32Ext as _;

use crate::ring_buffer::RingBuffer;

#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FractionalDelay {
    pointer: f32,
    state: State,
}

impl FractionalDelay {
    #[must_use]
    pub fn impulse_position(&self) -> f32 {
        self.pointer
    }
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum State {
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
struct StateRewinding {
    pub relative_speed: f32,
    pub target_position: f32,
    pub rewind_speed: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct StateBlending {
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
            self.pointer = attributes.position;
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
        *relative_speed += if rewind_speed < 0.9 { 0.00001 } else { 0.001 };
    } else if rewind_speed.is_sign_negative() && *relative_speed > rewind_speed {
        *relative_speed -= if rewind_speed > -0.9 { 0.00001 } else { 0.001 };
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
        // NOTE: In terms of a single sample distance, this is nothing.
        self.abs() < 0.001
    }

    fn relative_eq(self, other: f32, epsilon: f32) -> bool {
        (self - other).abs() < epsilon
    }
}
