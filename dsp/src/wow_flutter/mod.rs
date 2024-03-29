//! Wow and flutter simulate variable velocity of tape.
//!
//! Changes in speed are caused by mechanical imperfections. Wow represents slow
//! changes (below 4 Hz), whole flutter fast (above 4 Hz).

#[allow(unused_imports)]
use micromath::F32Ext as _;

mod flutter;
mod ornstein_uhlenbeck;
mod wavefolder;
mod wow;

use self::flutter::{Attributes as FlutterAttributes, Flutter};
use self::wow::{Attributes as WowAttributes, Wow};
use crate::math;
use crate::random::Random;
use crate::ring_buffer::RingBuffer;

use sirena::memory_manager::MemoryManager;

const MAX_DEPTH_IN_SECONDS: usize = 1;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WowFlutter {
    sample_rate: u32,
    buffer: RingBuffer,
    wow: Wow,
    flutter: Flutter,
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub wow_depth: f32,
    pub flutter_depth: f32,
    pub flutter_chance: f32,
}

impl WowFlutter {
    pub fn new(sample_rate: u32, memory_manager: &mut MemoryManager) -> Self {
        Self {
            sample_rate,
            buffer: Self::allocate_buffer(Self::buffer_size(sample_rate), memory_manager),
            wow: Wow::new(sample_rate),
            flutter: Flutter::new(sample_rate),
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

    pub fn populate_delays(&mut self, buffer: &mut [f32], random: &mut impl Random) {
        self.roll_dice(random);
        for x in buffer.iter_mut() {
            *x = self.pop_delay(random);
        }
    }

    fn roll_dice(&mut self, random: &mut impl Random) {
        self.flutter.roll_dice(random);
    }

    fn pop_delay(&mut self, random: &mut impl Random) -> f32 {
        let wow_delay = self.wow.pop(random) * self.sample_rate as f32;
        let flutter_delay = self.flutter.pop() * self.sample_rate as f32;
        wow_delay + flutter_delay
    }

    /// Feed the buffer with incoming signal.
    ///
    /// This should be called when process is not. For instance if `pop_delay`
    /// is called directly. Feeding the buffer will prevent pops after the
    /// signal path is rerouted directly to process again.
    pub fn dry_process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            self.buffer.write(*x);
        }
    }

    pub fn process(&mut self, buffer: &mut [f32], delays: &[f32]) {
        for (d, x) in delays.iter().zip(buffer.iter_mut()) {
            let a = self.buffer.peek(*d as usize);
            let b = self.buffer.peek(*d as usize + 1);
            let delayed = a + (b - a) * d.fract();

            self.buffer.write(*x);

            *x = delayed;
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.wow.set_attributes(&attributes.into());
        self.flutter.set_attributes(&attributes.into());
    }

    pub fn buffer_reset(&mut self, start: usize, size: usize) {
        self.buffer.reset(start, size);
    }

    #[must_use]
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl From<Attributes> for WowAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            depth: other.wow_depth,
        }
    }
}

impl From<Attributes> for FlutterAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            depth: other.flutter_depth,
            chance: other.flutter_chance,
        }
    }
}
