//! Write to and read from a ring buffer, keeping data in a static slice.

use core::fmt;

pub struct RingBuffer {
    buffer: &'static mut [f32],
    write_index: usize,
}

impl fmt::Debug for RingBuffer {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "RingBuffer(write_index: {})", self.write_index,)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for RingBuffer {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "RingBuffer(write_index: {})", self.write_index,);
    }
}

impl From<&'static mut [f32]> for RingBuffer {
    fn from(buffer: &'static mut [f32]) -> Self {
        Self {
            buffer,
            write_index: 0,
        }
    }
}

impl RingBuffer {
    pub fn write(&mut self, value: f32) {
        self.write_index %= self.buffer.len();
        self.buffer[self.write_index] = value;
        self.write_index += 1;
    }

    pub fn peek(&self, relative_index: i32) -> f32 {
        let index = (self.write_index as i32 + relative_index - 1)
            .wrapping_rem_euclid(self.buffer.len() as i32) as usize;
        self.buffer[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;
    use sirena::memory_manager::MemoryManager;

    #[test]
    fn foo() {
        static mut MEMORY: [MaybeUninit<u32>; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let slice = memory_manager.allocate(3).unwrap();

        let mut buffer = RingBuffer::from(slice);
        assert_relative_eq!(buffer.peek(0), 0.0);
        assert_relative_eq!(buffer.peek(-1), 0.0);

        buffer.write(1.0);
        buffer.write(2.0);
        assert_relative_eq!(buffer.peek(0), 2.0);
        assert_relative_eq!(buffer.peek(-1), 1.0);
        assert_relative_eq!(buffer.peek(-2), 0.0);
    }

    #[test]
    fn initialize_buffer() {
        static mut MEMORY: [MaybeUninit<u32>; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(3).unwrap();
        let _buffer = RingBuffer::from(slice);
    }

    #[test]
    fn write_to_buffer() {
        static mut MEMORY: [MaybeUninit<u32>; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(3).unwrap();
        let mut buffer = RingBuffer::from(slice);

        buffer.write(1.0);
    }

    #[test]
    fn read_from_buffer() {
        static mut MEMORY: [MaybeUninit<u32>; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(3).unwrap();
        let mut buffer = RingBuffer::from(slice);

        buffer.write(1.0);
        buffer.write(2.0);
        buffer.write(3.0);

        assert_relative_eq!(buffer.peek(0), 3.0);
        assert_relative_eq!(buffer.peek(-1), 2.0);
        assert_relative_eq!(buffer.peek(-2), 1.0);
    }

    #[test]
    fn cross_buffer_end_while_reading() {
        static mut MEMORY: [MaybeUninit<u32>; 200] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(101).unwrap();
        let mut buffer = RingBuffer::from(slice);

        for x in 0..=100 {
            buffer.write(x as f32);
        }

        assert_eq!(buffer.peek(0) as usize, 100);
        assert_eq!(buffer.peek(-1) as usize, 100 - 1);
    }
}
