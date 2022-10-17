#[allow(unused_imports)]
use micromath::F32Ext as _;

use crate::math;
use crate::ring_buffer::RingBuffer;
use sirena::memory_manager::MemoryManager;

const MAX_LENGTH: f32 = 10.0;

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

// TODO: Introduce HeadAttributes
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub length: f32,
    pub head_1_position: f32,
    pub head_2_position: f32,
    pub head_3_position: f32,
    pub head_4_position: f32,
    pub head_1_feedback: f32,
    pub head_2_feedback: f32,
    pub head_3_feedback: f32,
    pub head_4_feedback: f32,
    pub head_1_volume: f32,
    pub head_2_volume: f32,
    pub head_3_volume: f32,
    pub head_4_volume: f32,
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

            // NOTE: These would just interpolate, xfade, return what the head is on
            let x1 = self.heads[0].reader.read(&self.buffer, age);
            let x2 = self.heads[1].reader.read(&self.buffer, age);
            let x3 = self.heads[2].reader.read(&self.buffer, age);
            let x4 = self.heads[3].reader.read(&self.buffer, age);

            let mut feedback = 0.0;
            feedback += x1 * self.heads[0].feedback;
            feedback += x2 * self.heads[1].feedback;
            feedback += x3 * self.heads[2].feedback;
            feedback += x4 * self.heads[3].feedback;
            *self.buffer.peek_mut(age) += feedback;

            // NOTE: Must read again now when feedback was written back
            let x1 = self.heads[0].reader.read(&self.buffer, age);
            let x2 = self.heads[1].reader.read(&self.buffer, age);
            let x3 = self.heads[2].reader.read(&self.buffer, age);
            let x4 = self.heads[3].reader.read(&self.buffer, age);

            let mut output = 0.0;
            output += x1 * self.heads[0].volume;
            output += x2 * self.heads[1].volume;
            output += x3 * self.heads[2].volume;
            output += x4 * self.heads[3].volume;
            *x += output;
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.length = attributes.length;
        // TODO: Set target in the reader instead
        // TODO: Convert from time to samples
        self.heads[0]
            .reader
            .set_position(attributes.head_1_position * self.sample_rate);
        self.heads[1]
            .reader
            .set_position(attributes.head_2_position * self.sample_rate);
        self.heads[2]
            .reader
            .set_position(attributes.head_3_position * self.sample_rate);
        self.heads[3]
            .reader
            .set_position(attributes.head_4_position * self.sample_rate);
        self.heads[0].feedback = attributes.head_1_feedback;
        self.heads[1].feedback = attributes.head_2_feedback;
        self.heads[2].feedback = attributes.head_3_feedback;
        self.heads[3].feedback = attributes.head_4_feedback;
        self.heads[0].volume = attributes.head_1_volume;
        self.heads[1].volume = attributes.head_2_volume;
        self.heads[2].volume = attributes.head_3_volume;
        self.heads[3].volume = attributes.head_4_volume;
    }
}

// TODO: Implement wrapper over Buffer that will interpolate samples and fade between them when jumps get too far
// <https://www.kvraudio.com/forum/viewtopic.php?t=251962>
#[derive(Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FractionalDelay {
    a: f32,
    b: f32,
}

// TODO: Moving slowly from one to another
// TODO: Or fading between with variable speed
impl FractionalDelay {
    pub fn read(&self, buffer: &RingBuffer, offset: usize) -> f32 {
        let a = buffer.peek(self.a as usize + offset);
        let b = buffer.peek(self.b as usize + 1 + offset);
        a + (b - a) * self.a.fract()
    }

    pub fn set_position(&mut self, position: f32) {
        self.a = position;
        self.b = position;
    }
}
