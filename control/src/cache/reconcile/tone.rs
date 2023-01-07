use super::calculate;
use crate::cache::display::{AltAttributeScreen, TonePosition};
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

impl Store {
    pub fn reconcile_tone(&mut self) {
        if self.input.button.pressed && self.input.tone.active() {
            if self.input.tone.value() > 0.5 {
                self.cache.options.filter_feedback = false;
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::TonePosition(TonePosition::Volume));
            } else {
                self.cache.options.filter_feedback = true;
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::TonePosition(TonePosition::Feedback));
            }
        }

        self.cache.attributes.tone = calculate(
            self.input.tone.value(),
            self.control_value_for_attribute(AttributeIdentifier::Tone),
            (0.0, 1.0),
            None,
        );
    }
}
