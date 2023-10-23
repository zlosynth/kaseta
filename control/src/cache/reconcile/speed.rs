use crate::cache::display::{AltAttributeScreen, AttributeScreen, SpeedRange};
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::DelayRange;
use crate::log;
use crate::Store;

impl Store {
    pub fn reconcile_speed(&mut self, needs_save: &mut bool) {
        let original_delay_range = self.cache.options.delay_range;

        if self.input.button.pressed && self.input.speed.activation_movement() {
            let value = self.input.speed.value();
            let (option, display) = if value < 0.5 {
                (DelayRange::Long, SpeedRange::Long)
            } else if value < 0.8 {
                (DelayRange::Short, SpeedRange::Short)
            } else {
                (DelayRange::Audio, SpeedRange::Audio)
            };
            self.cache.options.delay_range = option;
            self.cache
                .display
                .set_alt_menu(AltAttributeScreen::SpeedRange(display));
        }

        let delay_range = self.cache.options.delay_range;
        if delay_range != original_delay_range {
            *needs_save |= true;
            log::info!("Setting delay range={}", delay_range);
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
            let (speed, display) = match self.cache.options.delay_range {
                DelayRange::Long => self.speed_for_long_range(),
                DelayRange::Short => self.speed_for_short_range(),
                DelayRange::Audio => self.speed_for_audio_range(),
            };
            if !self.cache.configuration.default_display_page.is_position() {
                self.show_length_on_display(display);
            }
            self.cache.attributes.speed = speed;
        }
    }

    fn speed_for_audio_range(&mut self) -> (f32, f32) {
        let sum = super::sum(
            self.input.speed.last_value_above_noise,
            self.control_value_for_attribute(AttributeIdentifier::Speed)
                .map(|x| x / 5.0),
        );
        let voct = sum * 7.0;
        let a = 13.73;
        let frequency = a * libm::powf(2.0, voct);
        (1.0 / frequency, sum)
    }

    fn speed_for_short_range(&mut self) -> (f32, f32) {
        let sum = super::sum(
            self.input.speed.last_value_above_noise,
            self.control_value_for_attribute(AttributeIdentifier::Speed)
                .map(|x| x / 5.0),
        );
        let speed = super::calculate_from_sum(sum, (8.0, 0.01), None);
        (speed, sum)
    }

    fn speed_for_long_range(&mut self) -> (f32, f32) {
        let sum = super::sum(
            self.input.speed.last_value_above_noise,
            self.control_value_for_attribute(AttributeIdentifier::Speed)
                .map(|x| x / 5.0),
        );
        if sum < 0.5 {
            const MIN: f32 = 10.0;
            const MAX: f32 = 5.0 * 60.0;
            let phase = 1.0 - sum * 2.0;
            (MIN + phase * (MAX - MIN), sum)
        } else {
            const MIN: f32 = 0.01;
            const MAX: f32 = 8.0;
            let phase = 1.0 - (sum - 0.5) * 2.0;
            (MIN + phase * (MAX - MIN), sum)
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
