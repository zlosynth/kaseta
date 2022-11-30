use super::calculate;
use super::taper;
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

// TODO: Rename to speed and revert it 1/X
const LENGTH_LONG_RANGE: (f32, f32) = (0.02, 2.0 * 60.0);
const LENGTH_SHORT_RANGE: (f32, f32) = (1.0 / 400.0, 1.0);

impl Store {
    pub fn reconcile_speed(&mut self) {
        if self.input.speed.active() {
            self.cache.tapped_tempo = None;

            self.cache.attributes.speed = calculate(
                self.input.speed.value(),
                self.control_for_attribute(AttributeIdentifier::Speed),
                if self.cache.options.short_delay_range {
                    LENGTH_SHORT_RANGE
                } else {
                    LENGTH_LONG_RANGE
                },
                Some(taper::reverse_log),
            );
        } else if let Some(tapped_tempo) = self.cache.tapped_tempo {
            self.cache.attributes.speed = tapped_tempo;
        }
    }
}
