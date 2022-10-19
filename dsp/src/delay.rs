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
            head.reader
                .set_position(self.length * attributes.heads[i].position * self.sample_rate);
            head.feedback = attributes.heads[i].feedback;
            head.volume = attributes.heads[i].volume;
            head.reader.rewind_forward = attributes.heads[i].rewind_forward;
            head.reader.rewind_backward = attributes.heads[i].rewind_backward;
        }
    }
}

// TODO: Implement wrapper over Buffer that will interpolate samples and fade between them when jumps get too far
// <https://www.kvraudio.com/forum/viewtopic.php?t=251962>
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FractionalDelay {
    pointer: f32,
    target: f32,
    pub rewind_forward: Option<f32>,
    pub rewind_backward: Option<f32>,
    // relative_speed: f32,
}

// TODO: Moving slowly from one to another
// TODO: Or fading between with variable speed
// TODO: Implement rewind, can be enabled in either direction.
// NOTE: Rewind is moving to the target in a steady pace.
// Fading is going there instantly, fading between the current and the destination.
impl FractionalDelay {
    pub fn read(
        &mut self,
        buffer: &RingBuffer,
        offset: usize,
        // TODO: Keep these two as part of the fractional delay
    ) -> f32 {
        let a = buffer.peek(self.pointer as usize + offset);
        let b = buffer.peek(self.pointer as usize + 1 + offset);
        let x = a + (b - a) * self.pointer.fract();

        if (self.target - self.pointer).abs() > f32::EPSILON {
            // NOTE(allow): It makes more sense to keep it symetrical.
            #[allow(clippy::collapsible_else_if)]
            if self.pointer < self.target {
                if let Some(speed) = self.rewind_backward {
                    // TODO: Implement acceleration
                    self.pointer += (self.target - self.pointer).min(speed);
                } else {
                    self.pointer = self.target;
                }
            } else {
                if let Some(speed) = self.rewind_forward {
                    self.pointer -= (self.pointer - self.target).min(speed);
                } else {
                    self.pointer = self.target;
                }
            }
        }

        x
    }

    pub fn set_position(&mut self, position: f32) {
        self.target = position;
    }
}
