use super::calculate;
use super::taper;
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

// Pre-amp scales between -20 to +28 dB.
const PRE_AMP_RANGE: (f32, f32) = (0.0, 25.0);

impl Store {
    pub fn reconcile_pre_amp(&mut self) {
        self.cache.attributes.pre_amp = calculate(
            self.input.pre_amp.value(),
            self.control_for_attribute(AttributeIdentifier::PreAmp),
            PRE_AMP_RANGE,
            Some(taper::log),
        );
    }
}
