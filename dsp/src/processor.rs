//! Main interface for the DSP loop.

use sirena::signal::{self, Signal, SignalClipAmp};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Processor {
    _fs: f32,
}

impl Processor {
    pub fn new(_fs: f32) -> Self {
        Self { _fs }
    }

    pub fn process(&mut self, block: &mut [f32; 32]) {
        let block_copy = *block;

        let mut instrument = signal::from_iter(block_copy.into_iter()).clip_amp(10.0);

        block.iter_mut().for_each(|f| {
            *f = instrument.next();
        });
    }
}
