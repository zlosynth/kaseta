use super::calculate;
use crate::cache::display::{AltAttributeScreen, AttributeScreen, TonePosition};
use crate::cache::mapping::AttributeIdentifier;
use crate::log;
use crate::Store;

impl Store {
    pub fn reconcile_tone(&mut self, needs_save: &mut bool) {
        let original_filter_feedback = self.cache.options.filter_feedback;

        if self.input.button.pressed && self.input.tone.activation_movement() {
            self.cache.options.filter_feedback = self.input.tone.value() < 0.5;
            if self.cache.options.filter_feedback {
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::TonePosition(TonePosition::Feedback));
            } else {
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::TonePosition(TonePosition::Volume));
            }
        }

        let filter_feedback = self.cache.options.filter_feedback;
        if filter_feedback != original_filter_feedback {
            *needs_save |= true;
            if filter_feedback {
                log::info!("Positioning tone filter=feedback");
            } else {
                log::info!("Positioning tone filter=volume");
            }
        }

        let phase = calculate(
            self.input.tone.value(),
            // NOTE: Divide -5 to +5 V by 10. This way, when the pot is on its lowest,
            // fully open ADSR would open the filter fully. When the pot is on its max,
            // the same can be done with inverted ADSR.
            self.control_value_for_attribute(AttributeIdentifier::Tone)
                .map(|x| x / 10.0),
            (0.0, 1.0),
            None,
        );

        self.show_tone_on_display(phase);

        self.cache.attributes.tone = phase;
    }

    fn show_tone_on_display(&mut self, phase: f32) {
        if self.input.tone.activation_movement() {
            self.cache
                .display
                .force_attribute(AttributeScreen::Tone(phase));
        } else {
            self.cache
                .display
                .update_attribute(AttributeScreen::Tone(phase));
        }
    }
}
