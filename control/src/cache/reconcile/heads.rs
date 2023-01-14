use super::{calculate, taper};
use crate::cache::display::AttributeScreen;
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::quantization::{quantize, Quantization};
use crate::cache::AttributesHead;
use crate::Store;

impl Store {
    pub fn reconcile_heads(&mut self) {
        for i in 0..4 {
            self.reconcile_position(i);
        }

        let ordered_heads = self.heads_ordered_by_position();
        self.set_screen_for_positions(&ordered_heads);
        let relative_position_by_index = relative_position_by_index(&ordered_heads);

        for (i, relative_position) in relative_position_by_index.iter().enumerate() {
            self.reconcile_volume(i, *relative_position);
            self.reconcile_feedback(i, *relative_position);
            self.reconcile_pan(i, *relative_position);
        }
    }

    fn reconcile_position(&mut self, i: usize) {
        self.cache.attributes.head[i].position = quantize(
            calculate(
                self.input.head[i].position.value(),
                self.control_value_for_attribute(AttributeIdentifier::Position(i)),
                (0.0, 1.0),
                None,
            ),
            Quantization::from((self.cache.options.quantize_6, self.cache.options.quantize_8)),
        );
    }

    fn reconcile_volume(&mut self, i: usize, relative_position: usize) {
        let volume_sum = super::sum(
            (self.input.head[i].volume.value() - 0.02) / 0.98,
            self.control_value_for_attribute(AttributeIdentifier::Volume(i)),
        );
        // The top limit is made to match compressor's treshold.
        self.cache.attributes.head[i].volume =
            super::calculate_from_sum(volume_sum, (0.0, 0.25), Some(taper::log));
        let screen = AttributeScreen::Volume(i, relative_position, volume_sum);
        if self.input.head[i].volume.active() {
            self.cache.display.force_attribute(screen);
        } else {
            self.cache.display.update_attribute(screen);
        }
    }

    fn reconcile_feedback(&mut self, i: usize, relative_position: usize) {
        let feedback_sum = super::sum(
            (self.input.head[i].feedback.value() - 0.02) / 0.98,
            self.control_value_for_attribute(AttributeIdentifier::Feedback(i)),
        );
        self.cache.attributes.head[i].feedback =
            super::calculate_from_sum(feedback_sum, (0.0, 1.2), None);
        let screen = AttributeScreen::Feedback(i, relative_position, feedback_sum);
        if self.input.head[i].feedback.active() {
            self.cache.display.force_attribute(screen);
        } else {
            self.cache.display.update_attribute(screen);
        }
    }

    fn reconcile_pan(&mut self, i: usize, relative_position: usize) {
        let pan_sum = super::sum(
            self.input.head[i].pan.value(),
            self.control_value_for_attribute(AttributeIdentifier::Pan(i)),
        );
        self.cache.attributes.head[i].pan = super::calculate_from_sum(pan_sum, (0.0, 1.0), None);
        let screen = AttributeScreen::Pan(i, relative_position, pan_sum);
        if self.input.head[i].pan.active() {
            self.cache.display.force_attribute(screen);
        } else {
            self.cache.display.update_attribute(screen);
        }
    }

    fn heads_ordered_by_position(&self) -> [(usize, AttributesHead); 4] {
        let mut ordered_heads = [
            (0, self.cache.attributes.head[0]),
            (1, self.cache.attributes.head[1]),
            (2, self.cache.attributes.head[2]),
            (3, self.cache.attributes.head[3]),
        ];
        for i in 0..ordered_heads.len() {
            for j in 0..ordered_heads.len() - 1 - i {
                if ordered_heads[j].1.position > ordered_heads[j + 1].1.position {
                    ordered_heads.swap(j, j + 1);
                }
            }
        }
        ordered_heads
    }

    fn set_screen_for_positions(&mut self, ordered_heads: &[(usize, AttributesHead); 4]) {
        let screen_for_positions = screen_for_positions(ordered_heads);
        let touched_position = self.input.head.iter().any(|h| h.position.active());
        if touched_position {
            self.cache.display.force_attribute(screen_for_positions);
        } else {
            self.cache.display.update_attribute(screen_for_positions);
        }
        self.cache
            .display
            .set_fallback_attribute(screen_for_positions);
    }
}

fn screen_for_positions(ordered_heads: &[(usize, AttributesHead); 4]) -> AttributeScreen {
    AttributeScreen::Positions((
        [
            ordered_heads[0].1.volume > 0.00,
            ordered_heads[1].1.volume > 0.00,
            ordered_heads[2].1.volume > 0.00,
            ordered_heads[3].1.volume > 0.00,
        ],
        [
            ordered_heads[0].1.feedback > 0.00,
            ordered_heads[1].1.feedback > 0.00,
            ordered_heads[2].1.feedback > 0.00,
            ordered_heads[3].1.feedback > 0.00,
        ],
    ))
}

fn relative_position_by_index(ordered_heads: &[(usize, AttributesHead); 4]) -> [usize; 4] {
    let mut relative_position_by_index = [0; 4];
    for (relative_position, (i, _)) in ordered_heads.iter().enumerate() {
        relative_position_by_index[*i] = relative_position;
    }
    relative_position_by_index
}
