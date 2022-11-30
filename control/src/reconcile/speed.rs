use super::calculate;
use crate::mapping::AttributeIdentifier;
use crate::taper;
use crate::Store;

// TODO: Rename to speed and revert it 1/X
const LENGTH_LONG_RANGE: (f32, f32) = (0.02, 2.0 * 60.0);
const LENGTH_SHORT_RANGE: (f32, f32) = (1.0 / 400.0, 1.0);

impl Store {
    pub fn reconcile_speed(&mut self) {
        if self.inputs.speed.active() {
            self.tapped_tempo = None;

            self.attributes.speed = calculate(
                self.inputs.speed.value(),
                self.control_for_attribute(AttributeIdentifier::Speed),
                if self.options.short_delay_range {
                    LENGTH_SHORT_RANGE
                } else {
                    LENGTH_LONG_RANGE
                },
                Some(taper::reverse_log),
            );
        } else if let Some(tapped_tempo) = self.tapped_tempo {
            self.attributes.speed = tapped_tempo;
        }
    }
}
