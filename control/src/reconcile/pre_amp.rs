use super::calculate;
use crate::mapping::AttributeIdentifier;
use crate::taper;
use crate::Cache;

// Pre-amp scales between -20 to +28 dB.
const PRE_AMP_RANGE: (f32, f32) = (0.0, 25.0);

impl Cache {
    pub fn reconcile_pre_amp(&mut self) {
        self.attributes.pre_amp = calculate(
            self.inputs.pre_amp.value(),
            self.control_for_attribute(AttributeIdentifier::PreAmp),
            PRE_AMP_RANGE,
            Some(taper::log),
        );
    }
}
