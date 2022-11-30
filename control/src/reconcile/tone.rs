use super::calculate;
use crate::mapping::AttributeIdentifier;
use crate::Cache;

impl Cache {
    pub fn reconcile_tone(&mut self) {
        self.attributes.tone = calculate(
            self.inputs.tone.value(),
            self.control_for_attribute(AttributeIdentifier::Tone),
            (0.0, 1.0),
            None,
        );
    }
}
