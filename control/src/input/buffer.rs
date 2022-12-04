//! Smoothening of control data and travel distance measurement.
//!
//! # TODO
//!
//! * Write a benchmark
//! * See whether using memory manager and slices makes it faster
//! * Leverage pow2 sized arrays and binary optimization.
//! * Allow requesting average/travel for a specific age.
//! * Try optimizing reset with plain block of zero bytes.

/// Buffer meant for smoothening and history tracking.
///
/// This is not optimized for large buffers, but should be ok for smoothening
/// and travel distance measuring on input control voltage to up to 32 samples.
#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub(crate) struct Buffer<const N: usize> {
    buffer: [f32; N],
    pointer: usize,
}

impl<const N: usize> Default for Buffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Buffer<N> {
    pub fn new() -> Self {
        Self {
            buffer: [0.0; N],
            pointer: 0,
        }
    }

    pub fn write(&mut self, value: f32) {
        self.buffer[self.pointer] = value;
        self.pointer = (self.pointer + 1) % N;
    }

    pub fn read(&self) -> f32 {
        let sum: f32 = self.buffer.iter().sum();
        sum / N as f32
    }

    pub fn read_raw(&self) -> f32 {
        let newest = (self.pointer as i32 - 1).rem_euclid(N as i32) as usize;
        self.buffer[newest]
    }

    pub fn read_previous_raw(&self) -> f32 {
        let previous = (self.pointer as i32 - 2).rem_euclid(N as i32) as usize;
        self.buffer[previous]
    }

    pub fn reset(&mut self) {
        self.buffer = [0.0; N];
    }

    pub fn traveled(&self) -> f32 {
        let newest = (self.pointer as i32 - 1).rem_euclid(N as i32) as usize;
        let oldest = self.pointer;
        self.buffer[newest] - self.buffer[oldest]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_reads_it_returns_average() {
        let mut buffer: Buffer<4> = Buffer::new();
        buffer.write(4.0);
        buffer.write(8.0);
        buffer.write(16.0);
        buffer.write(32.0);
        assert_relative_eq!(buffer.read(), 15.0);
    }

    #[test]
    fn when_reads_raw_it_returns_last() {
        let mut buffer: Buffer<4> = Buffer::new();
        buffer.write(1.0);
        buffer.write(2.0);
        assert_relative_eq!(buffer.read_raw(), 2.0);
    }

    #[test]
    fn when_measures_traveled_it_returns_distance_from_oldest() {
        let mut buffer: Buffer<4> = Buffer::new();

        buffer.write(1.0);
        buffer.write(2.0);
        buffer.write(3.0);
        buffer.write(4.0);
        assert_relative_eq!(buffer.traveled(), 3.0);

        buffer.write(4.0);
        buffer.write(3.0);
        buffer.write(2.0);
        buffer.write(1.0);
        assert_relative_eq!(buffer.traveled(), -3.0);
    }
}
