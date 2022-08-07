//! Downsample signal N times and smoothen it using a sinc filter.

use core::fmt;

use sirena::ring_buffer::RingBuffer;
use sirena::signal::{self, Signal};

use super::coefficients::COEFFICIENTS_8;

pub struct Downsampler<const N: usize> {
    factor: usize,
    coefficients: &'static [f32; N],
    buffer: RingBuffer<N>,
}

impl<const N: usize> fmt::Debug for Downsampler<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Downsampler")
            .field("factor", &self.factor)
            .finish()
    }
}

#[cfg(feature = "defmt")]
impl<const N: usize> defmt::Format for Downsampler<N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Downsampler({=usize})", self.factor);
    }
}

/// Downsample signal 8x.
pub type Downsampler8 = Downsampler<{ COEFFICIENTS_8.len() }>;

impl Downsampler8 {
    #[must_use]
    pub fn new_8() -> Self {
        Self {
            factor: 8,
            coefficients: &COEFFICIENTS_8,
            buffer: RingBuffer::new(),
        }
    }
}

pub trait SignalDownsample: Signal {
    fn downsample<const N: usize>(self, downsampler: &mut Downsampler<N>) -> Downsample<Self, N>
    where
        Self: Sized,
    {
        Downsample {
            source: self,
            downsampler,
        }
    }
}

impl<T> SignalDownsample for T where T: Signal {}

pub struct Downsample<'a, S, const N: usize> {
    source: S,
    downsampler: &'a mut Downsampler<N>,
}

impl<'a, S, const N: usize> Signal for Downsample<'a, S, N>
where
    S: Signal,
{
    fn next(&mut self) -> f32 {
        (0..self.downsampler.factor)
            .for_each(|_| self.downsampler.buffer.write(self.source.next()));

        let mut output = signal::EQUILIBRIUM;

        for (i, coefficient) in self.downsampler.coefficients.iter().enumerate() {
            let past_value_index = -(i as i32);
            let past_value = self.downsampler.buffer.peek(past_value_index);
            output += past_value * coefficient;
        }

        output
    }
}
