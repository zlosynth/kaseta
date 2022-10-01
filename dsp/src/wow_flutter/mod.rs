//! Wow and flutter simulate variable velocity of tape.
//!
//! Changes in speed are caused by mechanical imperfections. Wow represents slow
//! changes (below 4 Hz), whole flutter fast (above 4 Hz).

mod ornstein_uhlenbeck;
mod wow;

use self::wow::Wow;
use crate::math;
use crate::random::Random;
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
    pub wow_amplitude_noise: f32,
}

impl WowFlutter {
    pub fn new(sample_rate: u32, memory_manager: &mut MemoryManager) -> Self {
        Self {
            sample_rate,
            buffer: Self::allocate_buffer(Self::buffer_size(sample_rate), memory_manager),
            wow: Wow::new(sample_rate),
        }
    }

    fn buffer_size(sample_rate: u32) -> usize {
        sample_rate as usize * MAX_DEPTH_IN_SECONDS
    }

    fn allocate_buffer(size: usize, memory_manager: &mut MemoryManager) -> RingBuffer {
        let slice = memory_manager
            .allocate(math::upper_power_of_two(size))
            .unwrap();
        RingBuffer::from(slice)
    }

    pub fn process(&mut self, buffer: &mut [f32], random: &mut impl Random) {
        for x in buffer.iter_mut() {
            let delay = self.wow.pop(random) * self.sample_rate as f32;
            let delayed = self.buffer.peek(delay as usize);
            self.buffer.write(*x);
            *x = delayed;
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.wow.frequency = attributes.wow_frequency;
        self.wow.depth = attributes.wow_depth;
        self.wow.amplitude_ornstein_uhlenbeck.noise = attributes.wow_amplitude_noise;
    }
}
