use super::calculate;
use super::taper;
use crate::cache::mapping::AttributeIdentifier;
use crate::Store;

// TODO: Rename to speed and revert it 1/X
const LENGTH_LONG_RANGE: (f32, f32) = (0.02, 2.0 * 60.0);
const LENGTH_SHORT_RANGE: (f32, f32) = (1.0 / 400.0, 1.0);

impl Store {
    pub fn reconcile_speed(&mut self) {
        self.cache.options.short_delay_range = self.input.switch[1];

        let control_index = self.control_index_for_attribute(AttributeIdentifier::Speed);
        let clock_tempo = control_index
            .map(|i| &self.cache.clock_detectors[i])
            .and_then(|d| d.detected_tempo());

        if let Some(clock_tempo) = clock_tempo {
            let c_i = f32_to_usize_5(self.input.speed.value());
            let coefficient = [4.0, 2.0, 1.0, 0.5, 0.25][c_i];
            self.cache.attributes.speed = (clock_tempo as f32 / 1000.0) * coefficient;
        } else if let Some(tapped_tempo) = self.cache.tapped_tempo {
            self.cache.attributes.speed = tapped_tempo;
        } else {
            self.cache.attributes.speed = calculate(
                self.input.speed.value(),
                self.control_value_for_attribute(AttributeIdentifier::Speed),
                if self.cache.options.short_delay_range {
                    LENGTH_SHORT_RANGE
                } else {
                    LENGTH_LONG_RANGE
                },
                Some(taper::log),
            );
        }
    }
}

// TODO: Keep this in Pot, so it can manage its own hysteresis
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
