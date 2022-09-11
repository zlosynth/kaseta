//! Main interface for the DSP loop.

use sirena::memory_manager::MemoryManager;
use sirena::signal::{self, Signal, SignalClipAmp, SignalMulAmp};

use crate::hysteresis::{Attributes as HysteresisAttributes, Hysteresis, SignalApplyHysteresis};
use crate::oversampling::{Downsampler4, SignalDownsample, SignalUpsample, Upsampler4};
use crate::smoothed_value::SmoothedValue;
use crate::wow_flutter::{Attributes as WowFlutterAttributes, SignalApplyWowFlutter, WowFlutter};

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Processor {
    upsampler: Upsampler4,
    downsampler: Downsampler4,
    pre_amp: SmoothedValue,
    hysteresis: Hysteresis,
    wow_flutter: WowFlutter,
}

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Attributes {
    pub pre_amp: f32,
    pub drive: f32,
    pub saturation: f32,
    pub width: f32,
    pub wow_frequency: f32,
    pub wow_depth: f32,
}

impl Processor {
    #[allow(clippy::let_and_return)]
    #[must_use]
    pub fn new(fs: f32, memory_manager: &mut MemoryManager) -> Self {
        let upsampler = Upsampler4::new_4(memory_manager);
        let downsampler = Downsampler4::new_4(memory_manager);

        const SMOOTHING_STEPS: u32 = 32;
        let pre_amp = SmoothedValue::new(0.0, SMOOTHING_STEPS);

        let hysteresis = Hysteresis::new(fs);

        let wow_flutter = WowFlutter::new(fs as u32, memory_manager);

        let mut uninitialized_processor = Self {
            upsampler,
            downsampler,
            pre_amp,
            hysteresis,
            wow_flutter,
        };

        uninitialized_processor.set_attributes(Attributes::default());
        let processor = uninitialized_processor;

        processor
    }

    pub fn process(&mut self, block: &mut [f32; 32]) {
        let block_copy = *block;

        let mut instrument = signal::from_iter(block_copy.into_iter())
            .apply_wow_flutter(&mut self.wow_flutter)
            .mul_amp(self.pre_amp.by_ref())
            .clip_amp(25.0);

        let mut buffer_1 = [0.0; 32];
        for x in buffer_1.iter_mut() {
            *x = instrument.next();
        }

        let mut buffer_2 = [0.0; 32 * 4];
        self.upsampler.process(&buffer_1, &mut buffer_2);

        let mut instrument = signal::from_iter(buffer_2.into_iter())
            .apply_hysteresis(&mut self.hysteresis);

        let mut buffer_3 = [0.0; 32 * 4];
        for x in buffer_3.iter_mut() {
            *x = instrument.next();
        }

        self.downsampler.process(&buffer_3, &mut block[..]);
    }

    pub fn set_attributes(&mut self, attributes: Attributes) {
        self.pre_amp.set(attributes.pre_amp);
        self.hysteresis.set_attributes(attributes.into());
        self.wow_flutter.set_attributes(attributes.into());
    }
}

impl From<Attributes> for HysteresisAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            drive: other.drive,
            saturation: other.saturation,
            width: other.width,
        }
    }
}

impl From<Attributes> for WowFlutterAttributes {
    fn from(other: Attributes) -> Self {
        Self {
            wow_frequency: other.wow_frequency,
            wow_depth: other.wow_depth,
        }
    }
}
