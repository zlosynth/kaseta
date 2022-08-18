use crate::memory_manager::MemoryManager;
use crate::ring_buffer::RingBuffer;
use sirena::signal::Signal;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WowFlutter {
    sample_rate: u32,
    buffer: RingBuffer,
}

impl WowFlutter {
    pub fn new(sample_rate: u32, memory_manager: &mut MemoryManager) -> Self {
        let slice = memory_manager.allocate(sample_rate as usize).unwrap();
        let buffer = RingBuffer::from(slice);
        Self {
            sample_rate,
            buffer,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let delayed = self.buffer.peek(-(self.sample_rate as i32) + 1);
        self.buffer.write(x);
        delayed
    }
}

pub trait SignalApplyWowFlutter: Signal {
    fn apply_wow_flutter(self, wow_flutter: &mut WowFlutter) -> ApplyWowFlutter<Self>
    where
        Self: Sized,
    {
        ApplyWowFlutter {
            source: self,
            wow_flutter,
        }
    }
}

impl<T> SignalApplyWowFlutter for T where T: Signal {}

pub struct ApplyWowFlutter<'a, S> {
    source: S,
    wow_flutter: &'a mut WowFlutter,
}

impl<'a, S> Signal for ApplyWowFlutter<'a, S>
where
    S: Signal,
{
    fn next(&mut self) -> f32 {
        self.wow_flutter.process(self.source.next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;

    #[test]
    fn it_delays_signal_by_fixed_interval() {
        use crate::memory_manager::MemoryManager;

        const SAMPLE_RATE: u32 = 10;

        static mut MEMORY: [MaybeUninit<u32>; SAMPLE_RATE as usize] =
            unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

        let mut wow_flutter = WowFlutter::new(SAMPLE_RATE, &mut memory_manager);
        assert_relative_eq!(wow_flutter.process(1.0), 0.0);
        for _ in 0..SAMPLE_RATE - 1 {
            assert_relative_eq!(wow_flutter.process(0.0), 0.0);
        }
        assert_relative_eq!(wow_flutter.process(0.0), 1.0);
        assert_relative_eq!(wow_flutter.process(0.0), 0.0);
    }
}
