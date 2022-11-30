use super::calculate;
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

const WOW_DEPTH_RANGE: (f32, f32) = (0.0, 0.2);
const FLUTTER_DEPTH_RANGE: (f32, f32) = (0.0, 0.0015);

impl Store {
    pub fn reconcile_wow_flutter(&mut self) {
        let depth = calculate(
            self.input.wow_flut.value(),
            self.control_for_attribute(AttributeIdentifier::WowFlut),
            (-1.0, 1.0),
            None,
        );

        (self.cache.attributes.wow, self.cache.attributes.flutter) = if depth.is_sign_negative() {
            (calculate(-depth, None, WOW_DEPTH_RANGE, None), 0.0)
        } else {
            (0.0, calculate(depth, None, FLUTTER_DEPTH_RANGE, None))
        };
    }
}
