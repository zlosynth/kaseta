//! Main interface for the DSP loop.

use crate::oversampling::{Downsampler8, SignalDownsample, SignalUpsample, Upsampler8};
use sirena::signal::{self, Signal, SignalClipAmp};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Processor {
    _fs: f32,
    upsampler: Upsampler8,
    downsampler: Downsampler8,
}

impl Processor {
    pub fn new(_fs: f32) -> Self {
        Self {
            _fs,
            upsampler: Upsampler8::new_8(),
            downsampler: Downsampler8::new_8(),
        }
    }

    pub fn process(&mut self, block: &mut [f32; 32]) {
        let block_copy = *block;

        let mut instrument = signal::from_iter(block_copy.into_iter())
            .clip_amp(10.0)
            .upsample(&mut self.upsampler)
            .downsample(&mut self.downsampler);

        block.iter_mut().for_each(|f| {
            *f = instrument.next();
        });
    }
}
