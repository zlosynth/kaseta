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
    position: f32,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub length: f32,
    pub head_1_position: f32,
    pub head_2_position: f32,
    pub head_3_position: f32,
    pub head_4_position: f32,
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

    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter() {
            self.buffer.write(*x);
        }

        let buffer_len = buffer.len();
        for (i, x) in buffer.iter_mut().enumerate() {
            let age = buffer_len - i;

            let head_1_offset = (self.length * self.heads[0].position * self.sample_rate) as usize;
            let x1 = self.buffer.peek(head_1_offset + age);

            let head_2_offset = (self.length * self.heads[1].position * self.sample_rate) as usize;
            let x2 = self.buffer.peek(head_2_offset + age);

            let head_3_offset = (self.length * self.heads[2].position * self.sample_rate) as usize;
            let x3 = self.buffer.peek(head_3_offset + age);

            let head_4_offset = (self.length * self.heads[3].position * self.sample_rate) as usize;
            let x4 = self.buffer.peek(head_4_offset + age);

            *x = x1 + x2 + x3 + x4;
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.length = attributes.length;
        self.heads[0].position = attributes.head_1_position;
        self.heads[1].position = attributes.head_2_position;
        self.heads[2].position = attributes.head_3_position;
        self.heads[3].position = attributes.head_4_position;
    }
}
