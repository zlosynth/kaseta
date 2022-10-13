use sirena::memory_manager::MemoryManager;

#[allow(unused_imports)]
use micromath::F32Ext as _;

use crate::math;
use crate::ring_buffer::RingBuffer;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Compressor {
    original_peaks: RingBuffer,
    processed_peaks: RingBuffer,
}

impl Compressor {
    pub fn new(sample_rate: u32, memory_manager: &mut MemoryManager) -> Self {
        // The average should capture at least one wave cycle, considering 20 Hz
        // as the lowest frequency.
        let buffer_len = math::upper_power_of_two(sample_rate as usize / 20);
        Self {
            original_peaks: RingBuffer::from(memory_manager.allocate(buffer_len).unwrap()),
            processed_peaks: RingBuffer::from(memory_manager.allocate(buffer_len).unwrap()),
        }
    }

    pub fn prepare(&mut self, buffer: &[f32]) {
        feed_buffer_with_abs(buffer, &mut self.original_peaks);
    }

    pub fn process(&mut self, buffer: &mut [f32]) {
        feed_buffer_with_abs(buffer, &mut self.processed_peaks);

        let original_peak = self.original_peak();
        let processed_peak = self.processed_peak();

        let ratio = original_peak / processed_peak;
        for x in buffer.iter_mut() {
            *x *= ratio;
        }
    }

    fn original_peak(&self) -> f32 {
        find_max(self.original_peaks.buffer())
    }

    fn processed_peak(&self) -> f32 {
        find_max(self.processed_peaks.buffer())
    }
}

// TODO: Try if basic iteration makes it faster
fn find_max(buffer: &[f32]) -> f32 {
    buffer.iter().map(|x| x.abs()).fold(0.0, |a, b| a.max(b))
}

// TODO: Try if custom abs works faster
fn feed_buffer_with_abs(source_buffer: &[f32], destination_buffer: &mut RingBuffer) {
    for x in source_buffer {
        destination_buffer.write(x.abs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;

    fn sine_block(amplitude: f32) -> [f32; 32] {
        let mut buffer = [0.0; 32];
        for (i, x) in buffer.iter_mut().enumerate() {
            *x = libm::sinf(2.0 * core::f32::consts::PI * i as f32 / 32.0) * amplitude;
        }
        buffer
    }

    fn max(buffer: &[f32]) -> f32 {
        buffer.iter().fold(0.0, |a, b| a.max(*b))
    }

    #[test]
    fn it_initialize() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let _compressor = Compressor::new(100, &mut memory_manager);
    }

    #[test]
    fn it_can_measure_peak() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let mut compressor = Compressor::new(100, &mut memory_manager);
        compressor.prepare(&sine_block(1.0));
        assert_relative_eq!(compressor.original_peak(), 1.0);
    }

    #[test]
    fn it_increases_buffer_below_original() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let mut compressor = Compressor::new(100, &mut memory_manager);
        compressor.prepare(&sine_block(1.0));
        let mut buffer = sine_block(0.5);
        compressor.process(&mut buffer);
        assert_relative_eq!(max(&buffer), 1.0);
    }

    #[test]
    fn it_silences_buffer_above_original() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let mut compressor = Compressor::new(100, &mut memory_manager);
        compressor.prepare(&sine_block(1.0));
        let mut buffer = sine_block(1.5);
        compressor.process(&mut buffer);
        assert_relative_eq!(max(&buffer), 1.0);
    }
}
