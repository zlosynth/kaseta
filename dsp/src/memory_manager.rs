use core::fmt;
use core::mem::MaybeUninit;

pub struct MemoryManager {
    memory: &'static mut [MaybeUninit<u32>],
    pointer: usize,
}

impl fmt::Debug for MemoryManager {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "MemoryManager(pointer: {})", self.pointer)
    }
}

#[cfg(feature = "defmt")]
impl defmt::Format for MemoryManager {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "MemoryManager(pointer: {})", self.pointer);
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    NotEnoughMemory,
}

impl MemoryManager {
    /// Allocate f32 slice of requested size in memory.
    ///
    /// Allocated memory never gets freed.
    ///
    /// # Errors
    ///
    /// If there is not enough memory left to allocate requested size, the
    /// function returns `Error::NotEnoughMemory`.
    pub fn allocate(&mut self, size: usize) -> Result<&'static mut [f32], Error> {
        #![allow(clippy::similar_names)]

        if self.pointer + size > self.memory.len() {
            return Err(Error::NotEnoughMemory);
        }

        // Safety: The start is taken from given memory, the size is checked.
        let slice_start = core::ptr::addr_of_mut!(self.memory[self.pointer]);
        let maybe_slice_u32 = unsafe { core::slice::from_raw_parts_mut(slice_start, size) };
        self.pointer += size;

        // Safety: Both types are slices with items of identical size.
        let maybe_slice_f32 = unsafe {
            &mut *(maybe_slice_u32 as *mut [MaybeUninit<u32>] as *mut [MaybeUninit<f32>])
        };

        for elem in maybe_slice_f32.iter_mut() {
            elem.write(0.0);
        }

        // Safety: The pointer has valid values in it.
        let slice_f32 = unsafe { &mut *(maybe_slice_f32 as *mut [MaybeUninit<f32>] as *mut [f32]) };

        Ok(slice_f32)
    }
}

impl From<&'static mut [MaybeUninit<u32>]> for MemoryManager {
    fn from(memory: &'static mut [MaybeUninit<u32>]) -> Self {
        Self { memory, pointer: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_allocate_slice() {
        static mut MEMORY: [MaybeUninit<u32>; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice = memory_manager.allocate(5).unwrap();
        assert_eq!(slice.len(), 5);

        slice[1] = 2.0;
        assert_relative_eq!(slice[1], 2.0);
    }

    #[test]
    fn when_two_consecutive_slices_are_allocated_they_dont_overlap() {
        static mut MEMORY: [MaybeUninit<u32>; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let slice_1 = memory_manager.allocate(2).unwrap();
        let slice_2 = memory_manager.allocate(2).unwrap();
        slice_1[0] = 1.0;
        slice_1[1] = 2.0;
        slice_2[0] = 3.0;
        slice_2[1] = 4.0;

        assert_relative_eq!(slice_1[0], 1.0);
        assert_relative_eq!(slice_1[1], 2.0);
        assert_relative_eq!(slice_2[0], 3.0);
        assert_relative_eq!(slice_2[1], 4.0);
    }

    #[test]
    fn it_cannot_allocate_outside_allocated_memory() {
        static mut MEMORY: [MaybeUninit<u32>; 10] = unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        assert!(matches!(memory_manager.allocate(11), Err(_)));
    }
}
