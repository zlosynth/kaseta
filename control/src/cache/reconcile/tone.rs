use super::calculate;
use crate::cache::display::{
    AltAttributeScreen, AttributeScreen, FilterPlacement as FilterPlacementScreen,
};
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::FilterPlacement;
use crate::log;
use crate::Store;

impl Store {
    pub fn reconcile_tone(&mut self, needs_save: &mut bool) {
        let original_placement = self.cache.options.filter_placement;

        if self.input.button.pressed && self.input.tone.activation_movement() {
            let value = self.input.tone.value();
            let (placement, screen) = if value < 0.3 {
                (FilterPlacement::Input, FilterPlacementScreen::Input)
            } else if value < 0.6 {
                (FilterPlacement::Feedback, FilterPlacementScreen::Feedback)
            } else {
                (FilterPlacement::Both, FilterPlacementScreen::Both)
            };
            self.cache.options.filter_placement = placement;
            self.cache
                .display
                .set_alt_menu(AltAttributeScreen::FilterPlacement(screen));
        }

        let placement = self.cache.options.filter_placement;
        if placement != original_placement {
            *needs_save |= true;
            if placement.is_input() {
                log::info!("Setting filter placement=input");
            } else if placement.is_feedback() {
                log::info!("Setting filter placement=feedback");
            } else {
                log::info!("Setting filter placement=both");
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
