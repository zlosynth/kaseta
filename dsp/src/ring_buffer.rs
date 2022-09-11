//! Write to and read from a ring buffer, keeping data in a static slice.

use core::fmt;

pub struct RingBuffer {
    buffer: &'static mut [f32],
    mask: usize,
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
        assert!(is_power_of_2(buffer.len()));
        let mask = buffer.len() - 1;
        Self {
            buffer,
            mask,
            write_index: 0,
        }
    }
}

impl RingBuffer {
    pub fn write(&mut self, value: f32) {
        self.write_index = (self.write_index + 1) & self.mask;
        self.buffer[self.write_index] = value;
    }

    pub fn peek(&self, relative_index: usize) -> f32 {
        let index = (self.write_index + relative_index) & self.mask;
        self.buffer[index]
    }
}

fn is_power_of_2(n: usize) -> bool {
    if n == 1 {
        return true;
    } else if n % 2 != 0 || n == 0 {
        return false;
    }

    is_power_of_2(n / 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;
    use sirena::memory_manager::MemoryManager;

    #[test]
    fn check_power_of_2() {
        assert!(is_power_of_2(1));
        assert!(is_power_of_2(2));
        assert!(is_power_of_2(8));
        assert!(is_power_of_2(1024));

        assert!(!is_power_of_2(3));
        assert!(!is_power_of_2(10));
    }

    #[test]
    #[should_panic]
    fn initialize_buffer_with_invalid_size() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let slice = memory_manager.allocate(3).unwrap();
        let _buffer = RingBuffer::from(slice);
    }

    #[test]
    fn initialize_buffer() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(8).unwrap();
        let _buffer = RingBuffer::from(slice);
    }

    #[test]
    fn write_to_buffer() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(8).unwrap();
        let mut buffer = RingBuffer::from(slice);

        buffer.write(1.0);
    }

    #[test]
    fn read_from_buffer() {
        static mut MEMORY: [MaybeUninit<u32>; 16] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(8).unwrap();
        let mut buffer = RingBuffer::from(slice);

        buffer.write(1.0);
        buffer.write(2.0);
        buffer.write(3.0);

        assert_relative_eq!(buffer.peek(0), 3.0);
        assert_relative_eq!(buffer.peek(1), 2.0);
        assert_relative_eq!(buffer.peek(2), 1.0);
    }

    #[test]
    fn cross_buffer_end_while_reading() {
        static mut MEMORY: [MaybeUninit<u32>; 128] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(128).unwrap();
        let mut buffer = RingBuffer::from(slice);

        for x in 0..=127 {
            buffer.write(x as f32);
        }

        assert_eq!(buffer.peek(0) as usize, 127);
        assert_eq!(buffer.peek(1) as usize, 127 - 1);
    }
}
