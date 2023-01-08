use super::calculate;
use crate::cache::display::AttributeScreen;
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::quantization::{quantize, Quantization};
use crate::cache::AttributesHead;
use crate::Store;

impl Store {
    pub fn reconcile_heads(&mut self) {
        for i in 0..4 {
            self.reconcile_head(i);
        }

        self.cache
            .display
            .set_fallback_attribute(self.screen_for_heads());
    }

    fn reconcile_head(&mut self, i: usize) {
        self.cache.attributes.head[i].position = quantize(
            calculate(
                self.input.head[i].position.value(),
                self.control_value_for_attribute(AttributeIdentifier::Position(i)),
                (0.0, 1.0),
                None,
            ),
            Quantization::from((self.cache.options.quantize_6, self.cache.options.quantize_8)),
        );

        self.cache.attributes.head[i].volume = calculate(
            self.input.head[i].volume.value(),
            self.control_value_for_attribute(AttributeIdentifier::Volume(i)),
            (0.0, 1.0),
            None,
        );

        self.cache.attributes.head[i].feedback = calculate(
            self.input.head[i].feedback.value(),
            self.control_value_for_attribute(AttributeIdentifier::Feedback(i)),
            (0.0, 1.0),
            None,
        );

        self.cache.attributes.head[i].pan = calculate(
            self.input.head[i].pan.value(),
            self.control_value_for_attribute(AttributeIdentifier::Pan(i)),
            (0.0, 1.0),
            None,
        );
    }

    fn screen_for_heads(&self) -> AttributeScreen {
        let ordered_heads = self.heads_ordered_by_position();

        AttributeScreen::Positions((
            [
                ordered_heads[0].volume > 0.05,
                ordered_heads[1].volume > 0.05,
                ordered_heads[2].volume > 0.05,
                ordered_heads[3].volume > 0.05,
            ],
            [
                ordered_heads[0].feedback > 0.05,
                ordered_heads[1].feedback > 0.05,
                ordered_heads[2].feedback > 0.05,
                ordered_heads[3].feedback > 0.05,
            ],
        ))
    }

    fn heads_ordered_by_position(&self) -> [&AttributesHead; 4] {
        let mut ordered_heads = [
            &self.cache.attributes.head[0],
            &self.cache.attributes.head[1],
            &self.cache.attributes.head[2],
            &self.cache.attributes.head[3],
        ];
        for i in 0..ordered_heads.len() {
            for j in 0..ordered_heads.len() - 1 - i {
                if ordered_heads[j].position > ordered_heads[j + 1].position {
                    ordered_heads.swap(j, j + 1);
                }
            }
        }
        ordered_heads
    }
}
