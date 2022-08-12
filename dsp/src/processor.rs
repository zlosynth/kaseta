//! Main interface for the DSP loop.

use sirena::signal::{self, Signal, SignalClipAmp, SignalMulAmp};

use crate::hysteresis::{self, Hysteresis, SignalApplyHysteresis};
use crate::oversampling::{Downsampler4, SignalDownsample, SignalUpsample, Upsampler4};
use crate::smoothed_value::SmoothedValue;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Processor {
    upsampler: Upsampler4,
    downsampler: Downsampler4,

    hysteresis: Hysteresis,
    drive: SmoothedValue,
    saturation: SmoothedValue,
    width: SmoothedValue,
    makeup: SmoothedValue,
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub drive: f32,
    pub saturation: f32,
    pub width: f32,
}

impl Processor {
    #[allow(clippy::let_and_return)]
    #[must_use]
    pub fn new(fs: f32, attributes: Attributes) -> Self {
        let upsampler = Upsampler4::new_4();
        let downsampler = Downsampler4::new_4();

        const SMOOTHING_STEPS: u32 = 32;
        const OVERSAMPLED_SMOOTHING_STEPS: u32 = 4 * SMOOTHING_STEPS;
        let drive = SmoothedValue::new(0.0, OVERSAMPLED_SMOOTHING_STEPS);
        let saturation = SmoothedValue::new(0.0, OVERSAMPLED_SMOOTHING_STEPS);
        let width = SmoothedValue::new(0.0, OVERSAMPLED_SMOOTHING_STEPS);
        let makeup = SmoothedValue::new(0.0, SMOOTHING_STEPS);
        let hysteresis = Hysteresis::new(fs);

        let mut uninitialized_processor = Self {
            upsampler,
            downsampler,
            hysteresis,
            drive,
            saturation,
            width,
            makeup,
        };

        uninitialized_processor.set_attributes(attributes);
        let processor = uninitialized_processor;

        processor
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
            .downsample(&mut self.downsampler)
            .mul_amp(self.makeup.by_ref());

        for f in block.iter_mut() {
            *f = instrument.next();
        }
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.drive.set(attributes.drive);
        self.saturation.set(attributes.saturation);
        self.width.set(attributes.width);
        self.makeup.set(hysteresis::calculate_makeup(
            attributes.drive,
            attributes.saturation,
            attributes.width,
        ));
    }
}
