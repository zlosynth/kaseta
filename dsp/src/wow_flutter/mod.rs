mod wow;

use self::wow::Wow;
use crate::memory_manager::MemoryManager;
use crate::ring_buffer::RingBuffer;
use sirena::signal::Signal;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct WowFlutter {
    sample_rate: u32,
    buffer: RingBuffer,
    wow: Wow,
}

impl WowFlutter {
    pub fn new(sample_rate: u32, memory_manager: &mut MemoryManager) -> Self {
        let slice = memory_manager.allocate(sample_rate as usize).unwrap();
        let buffer = RingBuffer::from(slice);
        let wow = Wow::new(sample_rate);
        Self {
            sample_rate,
            buffer,
            wow,
        }
    }

    pub fn process(&mut self, x: f32) -> f32 {
        let delay = self.wow.pop() * self.sample_rate as f32;
        let delayed = self.buffer.peek(-delay as i32);
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
