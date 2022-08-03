//! Main interface for the DSP loop.

use sirena::signal::{self, Signal, SignalClipAmp};

use crate::hysteresis::{Hysteresis, SignalApplyHysteresis};
use crate::oversampling::{Downsampler8, SignalDownsample, SignalUpsample, Upsampler8};
use crate::smoothed_value::SmoothedValue;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Processor {
    upsampler: Upsampler8,
    downsampler: Downsampler8,

    hysteresis: Hysteresis,
    drive: SmoothedValue,
    saturation: SmoothedValue,
    width: SmoothedValue,
}

impl Processor {
    pub fn new(fs: f32) -> Self {
        let upsampler = Upsampler8::new_8();
        let downsampler = Downsampler8::new_8();

        const SMOOTHING_STEPS: u32 = 32;
        let drive = SmoothedValue::new(1.0, SMOOTHING_STEPS);
        let saturation = SmoothedValue::new(1.0, SMOOTHING_STEPS);
        let width = SmoothedValue::new(1.0, SMOOTHING_STEPS);
        let hysteresis = Hysteresis::new(fs, drive.value(), saturation.value(), width.value());

        Self {
            upsampler,
            downsampler,
            hysteresis,
            drive,
            saturation,
            width,
        }
    }

    pub fn process(&mut self, block: &mut [f32; 32]) {
        let block_copy = *block;

        let mut instrument = signal::from_iter(block_copy.into_iter())
            .clip_amp(10.0)
            .upsample(&mut self.upsampler)
            .apply_hysteresis(
                &mut self.hysteresis,
                self.drive.by_ref(),
                self.saturation.by_ref(),
                self.width.by_ref(),
            )
            .downsample(&mut self.downsampler);

        block.iter_mut().for_each(|f| {
            *f = instrument.next();
        });
    }
}
