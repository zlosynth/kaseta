use super::calculate;
use crate::cache::display::{AltAttributeScreen, TonePosition};
use crate::cache::mapping::AttributeIdentifier;
use crate::log;
use crate::Store;

impl Store {
    pub fn reconcile_tone(&mut self, needs_save: &mut bool) {
        let original_filter_feedback = self.cache.options.filter_feedback;

        if self.input.button.pressed && self.input.tone.active() {
            self.cache.options.filter_feedback = self.input.tone.value() < 0.5;
        }

        let filter_feedback = self.cache.options.filter_feedback;
        if filter_feedback != original_filter_feedback {
            *needs_save |= true;
            if filter_feedback {
                log::info!("Positioning tone filter=feedback");
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::TonePosition(TonePosition::Feedback));
            } else {
                log::info!("Positioning tone filter=volume");
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::TonePosition(TonePosition::Volume));
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
