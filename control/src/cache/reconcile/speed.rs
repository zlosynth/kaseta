use crate::cache::display::{AltAttributeScreen, AttributeScreen, SpeedRange};
use crate::cache::mapping::AttributeIdentifier;
use crate::log;
use crate::Store;

impl Store {
    pub fn reconcile_speed(&mut self, needs_save: &mut bool) {
        let original_short_delay_range = self.cache.options.short_delay_range;

        if self.input.button.pressed && self.input.speed.activation_movement() {
            self.cache.options.short_delay_range = self.input.speed.value() > 0.8;
            if self.cache.options.short_delay_range {
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::SpeedRange(SpeedRange::Short));
            } else {
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::SpeedRange(SpeedRange::Long));
            }
        }

        let short_delay_range = self.cache.options.short_delay_range;
        if short_delay_range != original_short_delay_range {
            *needs_save |= true;
            if short_delay_range {
                log::info!("Setting delay range=short");
            } else {
                log::info!("Setting delay range=long");
            }
        }

        let control_index = self.control_index_for_attribute(AttributeIdentifier::Speed);
        let clock_detector = control_index.map(|i| &self.cache.clock_detectors[i]);
        let clock_tempo = clock_detector.and_then(|d| d.detected_tempo());

        let just_triggered_clock = clock_detector.map_or(false, |c| c.just_detected());
        let just_triggered_tap = self.cache.tap_detector.just_detected();
        self.cache.attributes.reset_impulse = just_triggered_clock || just_triggered_tap;

        if let Some(clock_tempo) = clock_tempo {
            let c_i = f32_to_usize_5(self.input.speed.value());
            let coefficient = [1.0, 1.0 / 2.0, 1.0 / 4.0, 1.0 / 8.0, 1.0 / 16.0][c_i];
            self.cache.attributes.speed = (clock_tempo as f32 / 1000.0) * coefficient;
        } else if let Some(tapped_tempo) = self.cache.tapped_tempo {
            self.cache.attributes.speed = tapped_tempo;
        } else {
            self.cache.attributes.speed = if self.cache.options.short_delay_range {
                let sum = super::sum(
                    self.input.speed.last_value_above_noise,
                    self.control_value_for_attribute(AttributeIdentifier::Speed)
                        .map(|x| x / 5.0),
                );

                self.show_length_on_display(sum);

                let voct = sum * 7.0;
                let a = 13.73;
                let frequency = a * libm::powf(2.0, voct);
                1.0 / frequency
            } else {
                let sum = super::sum(
                    self.input.speed.last_value_above_noise,
                    self.control_value_for_attribute(AttributeIdentifier::Speed)
                        .map(|x| x / 5.0),
                );

                if sum < 0.25 {
                    self.show_length_on_display(0.0);
                    5.0 * 60.0
                } else if sum < 0.5 {
                    self.show_length_on_display(0.25);
                    60.0
                } else {
                    const MIN: f32 = 0.01;
                    const MAX: f32 = 10.0;
                    let phase = 1.0 - (sum - 0.5) * 2.0;
                    self.show_length_on_display(sum);
                    MIN + phase * (MAX - MIN)
                }
            };
        }
    }

    fn show_length_on_display(&mut self, phase: f32) {
        if self.input.speed.activation_movement() {
            self.cache
                .display
                .force_attribute(AttributeScreen::Speed(1.0 - phase));
        } else {
            self.cache
                .display
                .update_attribute(AttributeScreen::Speed(1.0 - phase));
        }
    }
}

fn f32_to_usize_5(x: f32) -> usize {
    if x < 1.0 / 5.0 {
        0
    } else if x < 2.0 / 5.0 {
        1
    } else if x < 3.0 / 5.0 {
        2
    } else if x < 4.0 / 5.0 {
        3
    } else {
        4
    }
}
