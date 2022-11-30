use super::calculate;
use crate::mapping::AttributeIdentifier;
use crate::Store;

impl Store {
    pub fn reconcile_tone(&mut self) {
        self.cache.attributes.tone = calculate(
            self.input.tone.value(),
            self.control_for_attribute(AttributeIdentifier::Tone),
            (0.0, 1.0),
            None,
        );
    }
}
