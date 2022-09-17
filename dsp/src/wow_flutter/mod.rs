mod wow;

use self::wow::Wow;
use crate::math;
use crate::ring_buffer::RingBuffer;

use sirena::memory_manager::MemoryManager;

const MAX_DEPTH_IN_SECONDS: usize = 20;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WowFlutter {
    sample_rate: u32,
    buffer: RingBuffer,
    wow: Wow,
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub wow_frequency: f32,
    pub wow_depth: f32,
}

impl WowFlutter {
    pub fn new(sample_rate: u32, memory_manager: &mut MemoryManager) -> Self {
        let slice = memory_manager
            .allocate(math::upper_power_of_two(
                sample_rate as usize * MAX_DEPTH_IN_SECONDS,
            ))
            .unwrap();
        let buffer = RingBuffer::from(slice);
        let wow = Wow::new(sample_rate);
        Self {
            sample_rate,
            buffer,
            wow,
        }
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            let delay = self.wow.pop() * self.sample_rate as f32;
            let delayed = self.buffer.peek(delay as usize);
            self.buffer.write(*x);
            *x = delayed;
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        // TODO: Use smoothed value
        self.wow.frequency = attributes.wow_frequency;
        self.wow.depth = attributes.wow_depth;
    }
}
