//! Upsample signal N times and smoothen it using a sinc filter.

use core::fmt;

use sirena::ring_buffer::RingBuffer;
use sirena::signal::{self, Signal};

use super::coefficients::*;

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

/// Upsample signal 8x.
pub type Upsampler8 = Upsampler<{ COEFFICIENTS_8.len() }, { COEFFICIENTS_8.len() / 2 + 1 }>;

impl Upsampler8 {
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
        let upsampler = &mut self.upsampler;

        if upsampler.coefficients_offset == 0 {
            upsampler.buffer.write(self.source.next());
        }

        let mut output = signal::EQUILIBRIUM;
        let mut coefficients_index = upsampler.coefficients_offset;

        while coefficients_index < upsampler.coefficients.len() {
            let past_value_index = -(coefficients_index as i32) / upsampler.factor as i32;
            let past_value = upsampler.buffer.peek(past_value_index);
            let amplification = upsampler.coefficients[coefficients_index];
            output += past_value * amplification * upsampler.factor as f32;

            coefficients_index += upsampler.factor;
        }

        upsampler.coefficients_offset += 1;
        upsampler.coefficients_offset %= upsampler.factor;

        output
    }
}
