use super::calculate;
use super::taper;
use crate::cache::display::AltAttributeScreen;
use crate::cache::display::AttributeScreen;
use crate::cache::display::HysteresisRange::{Limited, Unlimited};
use crate::cache::mapping::AttributeIdentifier;
use crate::log;
use crate::Store;

const DRY_WET_RANGE: (f32, f32) = (0.0, 1.0);
const DRIVE_RANGE: (f32, f32) = (0.1, 1.1);
const SATURATION_RANGE: (f32, f32) = (0.0, 1.0);
const BIAS_RANGE: (f32, f32) = (0.01, 1.0);

impl Store {
    pub fn reconcile_hysteresis(&mut self, needs_save: &mut bool) {
        self.reconcile_range_limitation(needs_save);
        self.reconcile_dry_wet();
        self.reconcile_drive_and_saturation();
        self.reconcile_bias();
    }

    fn reconcile_range_limitation(&mut self, needs_save: &mut bool) {
        let original_unlimited = self.cache.options.unlimited;

        if self.input.button.pressed && self.input.drive.active() {
            self.cache.options.unlimited = self.input.drive.value() > 0.9;
            if self.cache.options.unlimited {
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::HysteresisRange(Unlimited));
            } else {
                self.cache
                    .display
                    .set_alt_menu(AltAttributeScreen::HysteresisRange(Limited));
            }
        }

        let unlimited = self.cache.options.unlimited;
        if original_unlimited != unlimited {
            *needs_save |= true;
            if unlimited {
                log::info!("Enabling unlimited hysteresis");
            } else {
                log::info!("Disabling unlimited hysteresis");
            }
        }
    }

    fn reconcile_dry_wet(&mut self) {
        let dry_wet_sum = super::sum(
            self.input.dry_wet.value(),
            self.control_value_for_attribute(AttributeIdentifier::DryWet)
                .map(|x| x / 5.0),
        );
        self.display_dry_wet(dry_wet_sum);
        self.cache.attributes.dry_wet = super::calculate_from_sum(dry_wet_sum, DRY_WET_RANGE, None);
    }

    fn reconcile_drive_and_saturation(&mut self) {
        // Maximum limit of how much place on the slider is occupied by drive. This
        // gets scaled down based on bias.
        const DRIVE_PORTION: f32 = 1.0 / 2.0;

        let drive_input = super::sum(
            self.input.drive.value(),
            self.control_value_for_attribute(AttributeIdentifier::Drive)
                .map(|x| x / 5.0),
        );
        self.display_drive(drive_input);
        let drive = calculate(
            (drive_input / DRIVE_PORTION).min(1.0),
            None,
            DRIVE_RANGE,
            None,
        );
        self.cache.attributes.drive = drive;

        self.cache.attributes.saturation = calculate(
            ((drive_input - DRIVE_PORTION) / (1.0 - DRIVE_PORTION)).clamp(0.0, 1.0),
            None,
            SATURATION_RANGE,
            None,
        );
    }

    fn reconcile_bias(&mut self) {
        let max_bias = if self.cache.options.unlimited {
            BIAS_RANGE.1
        } else {
            max_bias_for_drive(self.cache.attributes.drive).clamp(BIAS_RANGE.0, BIAS_RANGE.1)
        };
        let bias_sum = super::sum(
            self.input.bias.value(),
            self.control_value_for_attribute(AttributeIdentifier::Bias)
                .map(|x| x / 5.0),
        );
        self.display_bias(bias_sum);
        self.cache.attributes.bias =
            super::calculate_from_sum(bias_sum, (0.01, max_bias), Some(taper::log));
    }

    fn display_dry_wet(&mut self, dry_wet_sum: f32) {
        if self.input.dry_wet.active() {
            self.cache
                .display
                .force_attribute(AttributeScreen::DryWet(dry_wet_sum));
        } else {
            self.cache
                .display
                .update_attribute(AttributeScreen::DryWet(dry_wet_sum));
        }
    }

    fn display_drive(&mut self, drive_input: f32) {
        if self.input.drive.active() {
            self.cache
                .display
                .force_attribute(AttributeScreen::Drive(drive_input));
        } else {
            self.cache
                .display
                .update_attribute(AttributeScreen::Drive(drive_input));
        }
    }

    fn display_bias(&mut self, bias_sum: f32) {
        if self.input.bias.active() {
            self.cache
                .display
                .force_attribute(AttributeScreen::Bias(bias_sum));
        } else {
            self.cache
                .display
                .update_attribute(AttributeScreen::Bias(bias_sum));
        }
    }
}

// Using <https://mycurvefit.com/> quadratic interpolation over data gathered
// while testing the instability peak. The lower bias end was flattened not
// to overdrive too much. Input data (maximum stable bias per drive):
// 1.0 0.9 0.8 0.7 0.6 0.5 0.4 0.3 0.2 0.1
// 0.1 0.992 1.091 1.240 1.240 1.388 1.580 1.580 1.780 1.780
fn max_bias_for_drive(drive: f32) -> f32 {
    const D4B_B: f32 = 1.0;
    const D4B_A1: f32 = 0.0;
    const D4B_A2: f32 = -0.2;

    let drive2 = drive * drive;
    D4B_B + D4B_A1 * drive + D4B_A2 * drive2
}
