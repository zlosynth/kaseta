use libm::powf;

#[allow(unused_imports)]
use micromath::F32Ext;

use super::taper;
use crate::cache::display::{AltAttributeScreen, AttributeScreen, PreAmpMode};
use crate::cache::mapping::AttributeIdentifier;
use crate::log;
use crate::Store;

// Pre-amp scales between -20 to +28 dB.
const PRE_AMP_RANGE: (f32, f32) = (0.0, 25.0);

impl Store {
    pub fn reconcile_pre_amp(&mut self, needs_save: &mut bool) {
        let original_enable_oscillator = self.cache.options.enable_oscillator;

        if self.input.button.pressed && self.input.pre_amp.active() {
            self.cache.options.enable_oscillator = self.input.pre_amp.value() > 0.5;
        }

        let enable_oscillator = self.cache.options.enable_oscillator;
        if enable_oscillator != original_enable_oscillator {
            *needs_save |= true;
            if enable_oscillator {
                log::info!("Enabling pre-amp oscillator");
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::PreAmpMode(PreAmpMode::Oscillator));
            } else {
                log::info!("Disabling pre-amp oscillator");
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::PreAmpMode(PreAmpMode::PreAmp));
            }
        }

        if self.cache.options.enable_oscillator {
            // TODO: Smoothen transitions when controlled through pot
            let control = self.control_value_for_attribute(AttributeIdentifier::PreAmp);
            let voct = if let Some(control) = control {
                let pot = self.input.pre_amp.value();

                // Keep the multiplier below 4, so assure that the result won't get
                // into the 5th octave when set on the edge.
                let octave_offset = (pot * 3.95).trunc();

                if self.input.pre_amp.active() {
                    self.cache
                        .display
                        .force_attribute(AttributeScreen::OctaveOffset(octave_offset as usize));
                }

                (octave_offset - 2.0 + control).clamp(0.0, 8.0) + 2.0
            } else {
                let pot = self.input.pre_amp.value();
                if self.input.pre_amp.active() {
                    self.cache
                        .display
                        .force_attribute(AttributeScreen::OscillatorTone(pot));
                }
                pot * 5.0 + 2.0
            };

            let a = 27.5;
            // TODO: Pass voct, for the dsp to convert it, it is better for smoothening
            self.cache.attributes.oscillator = a * powf(2.0, voct);
        } else {
            let sum = super::sum(
                self.input.pre_amp.value(),
                self.control_value_for_attribute(AttributeIdentifier::PreAmp)
                    .map(|x| x / 5.0),
            );

            if self.input.pre_amp.active() {
                self.cache
                    .display
                    .force_attribute(AttributeScreen::PreAmp(sum));
            } else {
                self.cache
                    .display
                    .update_attribute(AttributeScreen::PreAmp(sum));
            }

            self.cache.attributes.pre_amp =
                super::calculate_from_sum(sum, PRE_AMP_RANGE, Some(taper::log));
        }
    }
}
