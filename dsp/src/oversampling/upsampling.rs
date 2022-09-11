//! Upsample signal N times and smoothen it using a sinc filter.

use core::fmt;

use sirena::signal::{self, Signal};
use sirena::memory_manager::MemoryManager;

use crate::ring_buffer::RingBuffer;
use super::coefficients::COEFFICIENTS_4;

pub struct Upsampler<const N: usize, const M: usize> {
    factor: usize,
    coefficients: &'static [f32; N],
    buffer: RingBuffer,
    coefficients_offset: usize,
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
    #[must_use]
    pub fn new_4(memory_manager: &mut MemoryManager) -> Self {
        Self {
            factor: 4,
            coefficients: &COEFFICIENTS_4,
            // TODO: Calculate needed size
            buffer: RingBuffer::from(memory_manager.allocate(256).unwrap()),
            coefficients_offset: 0,
        }
    }
}

pub trait SignalUpsample: Signal {
    fn upsample<const N: usize, const M: usize>(
        self,
        upsampler: &mut Upsampler<N, M>,
    ) -> Upsample<Self, N, M>
    where
        Self: Sized,
    {
        Upsample {
            source: self,
            upsampler,
        }
    }
}

impl<T> SignalUpsample for T where T: Signal {}

pub struct Upsample<'a, S, const N: usize, const M: usize> {
    source: S,
    upsampler: &'a mut Upsampler<N, M>,
}

impl<'a, S, const N: usize, const M: usize> Signal for Upsample<'a, S, N, M>
where
    S: Signal,
{
    fn next(&mut self) -> f32 {
        if self.upsampler.coefficients_offset == 0 {
            self.upsampler.buffer.write(self.source.next());
        }

        let mut output = signal::EQUILIBRIUM;
        let mut coefficients_index = self.upsampler.coefficients_offset;

        while coefficients_index < self.upsampler.coefficients.len() {
            let past_value_index = coefficients_index / self.upsampler.factor;
            let past_value = self.upsampler.buffer.peek(past_value_index);
            let amplification = self.upsampler.coefficients[coefficients_index];
            output += past_value * amplification;

            coefficients_index += self.upsampler.factor;
        }

        self.upsampler.coefficients_offset += 1;
        self.upsampler.coefficients_offset %= self.upsampler.factor;

        output * self.upsampler.factor as f32
    }
}
