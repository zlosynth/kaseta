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
    play: bool,
}

#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[allow(clippy::struct_excessive_bools)]
pub struct Attributes {
    pub length: f32,
    pub head_1_position: f32,
    pub head_2_position: f32,
    pub head_3_position: f32,
    pub head_4_position: f32,
    pub head_1_play: bool,
    pub head_2_play: bool,
    pub head_3_play: bool,
    pub head_4_play: bool,
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

            let x1 = self.process_head(0, age);
            let x2 = self.process_head(1, age);
            let x3 = self.process_head(2, age);
            let x4 = self.process_head(3, age);

            *x = x1 + x2 + x3 + x4;
        }
    }

    fn process_head(&self, head_index: usize, age: usize) -> f32 {
        let head = &self.heads[head_index];
        if head.play {
            let head_offset = (self.length * head.position * self.sample_rate) as usize;
            self.buffer.peek(head_offset + age)
        } else {
            0.0
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.length = attributes.length;
        self.heads[0].position = attributes.head_1_position;
        self.heads[1].position = attributes.head_2_position;
        self.heads[2].position = attributes.head_3_position;
        self.heads[3].position = attributes.head_4_position;
        self.heads[0].play = attributes.head_1_play;
        self.heads[1].play = attributes.head_2_play;
        self.heads[2].play = attributes.head_3_play;
        self.heads[3].play = attributes.head_4_play;
    }
}
