use libm::powf;

use super::calculate;
use super::taper;
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

// Pre-amp scales between -20 to +28 dB.
const PRE_AMP_RANGE: (f32, f32) = (0.0, 25.0);

// Five octaves.
const VOCT_RANGE: (f32, f32) = (0.0, 5.0);

impl Store {
    pub fn reconcile_pre_amp(&mut self) {
        if self.input.button.pressed && self.input.pre_amp.active() {
            self.cache.options.enable_oscillator = self.input.pre_amp.value() > 0.5;
        }

        if self.cache.options.enable_oscillator {
            let pot = self.input.pre_amp.value() * 5.0;
            let control = self
                .control_value_for_attribute(AttributeIdentifier::PreAmp)
                .unwrap_or(0.0);
            let voct = (pot + control).clamp(0.0, 5.0) + 2.0;
            let a = 27.5;
            self.cache.attributes.oscillator = a * powf(2.0, voct);
        } else {
            self.cache.attributes.pre_amp = calculate(
                self.input.pre_amp.value(),
                self.control_value_for_attribute(AttributeIdentifier::PreAmp)
                    .map(|x| x / 5.0),
                PRE_AMP_RANGE,
                Some(taper::log),
            );
        }
    }
}
