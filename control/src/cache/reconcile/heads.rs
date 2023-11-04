use super::{calculate, taper};
use crate::cache::display::AttributeScreen;
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::quantization::{quantize, Quantization};
use crate::Store;

impl Store {
    pub fn reconcile_heads(&mut self) {
        for i in 0..4 {
            self.reconcile_position(i);
        }

        let default_display_heads = self.cache.configuration.default_display_page.is_heads();
        let in_audio_range = self.cache.options.delay_range.is_audio();
        if default_display_heads || in_audio_range {
            self.set_screen_for_heads_overview();
        }

        for i in 0..4 {
            self.reconcile_volume(i);
            self.reconcile_feedback(i);
            self.reconcile_pan(i);
        }
    }

    fn reconcile_position(&mut self, i: usize) {
        // TODO: Only update the position if CV has changed significantly (above noise
        // level) from the last value.
        self.cache.attributes.head[i].position = quantize(
            calculate(
                self.input.head[i].position.last_value_above_noise,
                self.control_value_for_attribute(AttributeIdentifier::Position(i))
                    .map(|x| x / 5.0),
                (0.0, 1.0),
                None,
            ),
            Quantization::from((self.cache.options.quantize_6, self.cache.options.quantize_8)),
        );
    }

    fn reconcile_volume(&mut self, i: usize) {
        let volume_sum = super::sum(
            (self.input.head[i].volume.value() - 0.02) / 0.98,
            self.control_value_for_attribute(AttributeIdentifier::Volume(i))
                .map(|x| x / 5.0),
        );
        // The top limit is made to match compressor's treshold.
        self.cache.attributes.head[i].volume =
            super::calculate_from_sum(volume_sum, (0.0, 0.25), Some(taper::log));
        let screen = AttributeScreen::Volume(i, volume_sum);
        if self.input.head[i].volume.activation_movement() {
            self.cache.display.force_attribute(screen);
        } else {
            self.cache.display.update_attribute(screen);
        }
    }

    fn reconcile_feedback(&mut self, i: usize) {
        let feedback_sum = super::sum(
            (self.input.head[i].feedback.value() - 0.02) / 0.98,
            self.control_value_for_attribute(AttributeIdentifier::Feedback(i))
                .map(|x| x / 5.0),
        );
        self.cache.attributes.head[i].feedback =
            super::calculate_from_sum(feedback_sum, (0.0, 1.2), None);
        let screen = AttributeScreen::Feedback(i, feedback_sum);
        if self.input.head[i].feedback.activation_movement() {
            self.cache.display.force_attribute(screen);
        } else {
            self.cache.display.update_attribute(screen);
        }
    }

    fn reconcile_pan(&mut self, i: usize) {
        let pan_sum = super::sum(
            self.input.head[i].pan.value(),
            self.control_value_for_attribute(AttributeIdentifier::Pan(i))
                .map(|x| x / 5.0),
        );
        self.cache.attributes.head[i].pan = super::calculate_from_sum(pan_sum, (0.0, 1.0), None);
        let screen = AttributeScreen::Pan(i, pan_sum);
        if self.input.head[i].pan.activation_movement() {
            self.cache.display.force_attribute(screen);
        } else {
            self.cache.display.update_attribute(screen);
        }
    }

    fn set_screen_for_heads_overview(&mut self) {
        let screen_for_heads_overview = self.screen_for_heads_overview();
        let touched_position = self
            .input
            .head
            .iter()
            .any(|h| h.position.activation_movement());
        if touched_position {
            self.cache
                .display
                .force_attribute(screen_for_heads_overview);
        } else {
            self.cache
                .display
                .update_attribute(screen_for_heads_overview);
        }
        self.cache
            .display
            .set_fallback_attribute(screen_for_heads_overview);
    }

    fn screen_for_heads_overview(&self) -> AttributeScreen {
        // TODO: Handle hysteresis for position
        AttributeScreen::HeadsOverview((
            [
                self.cache.attributes.head[0].volume > 0.00,
                self.cache.attributes.head[1].volume > 0.00,
                self.cache.attributes.head[2].volume > 0.00,
                self.cache.attributes.head[3].volume > 0.00,
            ],
            [
                self.cache.attributes.head[0].feedback > 0.00,
                self.cache.attributes.head[1].feedback > 0.00,
                self.cache.attributes.head[2].feedback > 0.00,
                self.cache.attributes.head[3].feedback > 0.00,
            ],
        ))
    }
}
