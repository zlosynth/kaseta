//! Upsample signal N times and smoothen it using a sinc filter.

use core::fmt;

use sirena::ring_buffer::RingBuffer;
use sirena::signal::{self, Signal};

use super::coefficients::COEFFICIENTS_8;

pub struct Upsampler<const N: usize, const M: usize> {
    factor: usize,
    coefficients: &'static [f32; N],
    buffer: RingBuffer<M>,
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

/// Upsample signal 8x.
pub type Upsampler8 = Upsampler<{ COEFFICIENTS_8.len() }, { COEFFICIENTS_8.len() / 2 + 1 }>;

impl Upsampler8 {
    #[must_use]
    pub fn new_8() -> Self {
        Self {
            factor: 8,
            coefficients: &COEFFICIENTS_8,
            buffer: RingBuffer::new(),
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
            let past_value_index = -(coefficients_index as i32) / self.upsampler.factor as i32;
            let past_value = self.upsampler.buffer.peek(past_value_index);
            let amplification = self.upsampler.coefficients[coefficients_index];
            output += past_value * amplification * self.upsampler.factor as f32;

            coefficients_index += self.upsampler.factor;
        }

        self.upsampler.coefficients_offset += 1;
        self.upsampler.coefficients_offset %= self.upsampler.factor;

        output
    }
}
