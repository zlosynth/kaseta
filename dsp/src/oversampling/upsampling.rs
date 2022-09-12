//! Upsample signal N times and smoothen it using a sinc filter.

use core::fmt;

use sirena::memory_manager::MemoryManager;

use super::coefficients::COEFFICIENTS_4;
use crate::ring_buffer::RingBuffer;

pub struct Upsampler<const N: usize, const M: usize> {
    factor: usize,
    coefficients: &'static [f32; N],
    buffer: RingBuffer,
}

impl<const N: usize, const M: usize> fmt::Debug for Upsampler<N, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Upsampler")
            .field("factor", &self.factor)
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl<const N: usize, const M: usize> defmt::Format for Upsampler<N, M> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Upsampler({=usize})", self.factor);
    }
}

/// Upsample signal 4x.
pub type Upsampler4 = Upsampler<{ COEFFICIENTS_4.len() }, { COEFFICIENTS_4.len() / 2 + 1 }>;

impl Upsampler4 {
    /// # Panics
    ///
    /// Panics if there is not enough space in the memory manager to allocate a
    /// buffer.
    #[must_use]
    pub fn new_4(memory_manager: &mut MemoryManager) -> Self {
        Self {
            factor: 4,
            coefficients: &COEFFICIENTS_4,
            // TODO: Calculate needed size
            buffer: RingBuffer::from(memory_manager.allocate(256).unwrap()),
        }
    }

    pub fn process(&mut self, input_buffer: &[f32], output_buffer: &mut [f32]) {
        for (i, x) in input_buffer.iter().enumerate() {
            self.buffer.write(*x);
            for coefficients_offset in 0..4 {
                let mut output = 0.0;
                let mut coefficients_index = coefficients_offset;

                while coefficients_index < self.coefficients.len() {
                    let past_value_index = coefficients_index / self.factor;
                    let past_value = self.buffer.peek(past_value_index);
                    let amplification = self.coefficients[coefficients_index];
                    output += past_value * amplification;

                    coefficients_index += self.factor;
                }

                output_buffer[i * self.factor + coefficients_offset] = output * self.factor as f32;
            }
        }
    }
}
