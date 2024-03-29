use super::calculate;
use super::taper;
use crate::cache::display::AltAttributeScreen;
use crate::cache::display::AttributeScreen;
use crate::cache::display::WowFlutterPlacement as WowFlutterPlacementScreen;
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::WowFlutterPlacement;
use crate::log;
use crate::Store;

const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 0.2);
const FLUTTER_DEPTH_RANGE: (f32, f32) = (0.0, 0.006);
// Once in 6 seconds to once a second.
const FLUTTER_CHANCE_RANGE: (f32, f32) = (0.0001, 0.0008);

impl Store {
    pub fn reconcile_wow_flutter(&mut self, needs_save: &mut bool) {
        let original_placement = self.cache.options.wow_flutter_placement;

        if self.input.button.pressed && self.input.wow_flut.activation_movement() {
            let value = self.input.wow_flut.value();
            let (placement, screen) = if value < 0.3 {
                (WowFlutterPlacement::Input, WowFlutterPlacementScreen::Input)
            } else if value < 0.6 {
                (WowFlutterPlacement::Read, WowFlutterPlacementScreen::Read)
            } else {
                (WowFlutterPlacement::Both, WowFlutterPlacementScreen::Both)
            };
            self.cache.options.wow_flutter_placement = placement;
            self.cache
                .display
                .set_alt_menu(AltAttributeScreen::WowFlutterPlacement(screen));
        }

        let placement = self.cache.options.wow_flutter_placement;
        if placement != original_placement {
            *needs_save |= true;
            if placement.is_input() {
                log::info!("Setting wow/flutter placement=input");
            } else if placement.is_read() {
                log::info!("Setting wow/flutter placement=read");
            } else {
                log::info!("Setting wow/flutter placement=both");
            }
        }

        let depth = calculate(
            self.input.wow_flut.value(),
            self.control_value_for_attribute(AttributeIdentifier::WowFlut)
                .map(|x| x / 10.0),
            (-1.0, 1.0),
            None,
        );

        if depth.is_sign_negative() {
            self.enable_wow(-depth);
        } else {
            self.enable_flutter(depth);
        };
    }

    fn enable_wow(&mut self, depth: f32) {
        self.cache.attributes.wow = calculate_wow(depth);
        self.cache.attributes.flutter_depth = 0.0;
        self.cache.attributes.flutter_chance = 0.0;

        if self.input.wow_flut.activation_movement() {
            self.cache
                .display
                .force_attribute(AttributeScreen::Wow(depth));
        } else {
            self.cache
                .display
                .update_attribute(AttributeScreen::Wow(depth));
        }
    }

    fn enable_flutter(&mut self, depth: f32) {
        let (flutter_depth, flutter_chance) = calculate_flutter(depth);

        self.cache.attributes.wow = 0.0;
        self.cache.attributes.flutter_depth = flutter_depth;
        self.cache.attributes.flutter_chance = flutter_chance;

        if self.input.wow_flut.activation_movement() {
            self.cache
                .display
                .force_attribute(AttributeScreen::Flutter(depth));
        } else {
            self.cache
                .display
                .update_attribute(AttributeScreen::Flutter(depth));
        }
    }
}

fn calculate_wow(depth: f32) -> f32 {
    const DEAD_ZONE: f32 = 0.1;

    if depth < DEAD_ZONE {
        return 0.0;
    }

    let scaled_depth = (depth - DEAD_ZONE) / (1.0 - DEAD_ZONE);
    calculate(scaled_depth, None, WOW_DEPTH_RANGE, None)
}

fn calculate_flutter(depth: f32) -> (f32, f32) {
    const DEAD_ZONE: f32 = 0.1;

    let flutter_depth = calculate(depth, None, FLUTTER_DEPTH_RANGE, None);
    let flutter_chance = if depth < DEAD_ZONE {
        0.0
    } else {
        let scaled_depth = (depth - DEAD_ZONE) / (1.0 - DEAD_ZONE);
        if scaled_depth > 0.95 {
            1.0
        } else {
            calculate(depth, None, FLUTTER_CHANCE_RANGE, Some(taper::log))
        }
    };

    (flutter_depth, flutter_chance)
}
