use super::calculate;
use super::taper;
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 0.2);
const FLUTTER_DEPTH_RANGE: (f32, f32) = (0.0, 0.002);
// Once in 6 seconds to once a second.
const FLUTTER_CHANCE_RANGE: (f32, f32) = (0.0001, 0.0008);

impl Store {
    pub fn reconcile_wow_flutter(&mut self) {
        let depth = calculate(
            self.input.wow_flut.value(),
            self.control_value_for_attribute(AttributeIdentifier::WowFlut),
            (-1.0, 1.0),
            None,
        );

        if depth.is_sign_negative() {
            self.cache.attributes.wow = calculate(-depth, None, WOW_DEPTH_RANGE, None);
            self.cache.attributes.flutter_depth = 0.0;
            self.cache.attributes.flutter_chance = 0.0;
        } else {
            self.cache.attributes.wow = 0.0;
            self.cache.attributes.flutter_depth = calculate(depth, None, FLUTTER_DEPTH_RANGE, None);
            self.cache.attributes.flutter_chance = if depth > 0.95 {
                1.0
            } else {
                calculate(depth, None, FLUTTER_CHANCE_RANGE, Some(taper::log))
            };
        };
    }
}
