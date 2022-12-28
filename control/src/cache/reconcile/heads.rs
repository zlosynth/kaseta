use super::calculate;
use crate::cache::mapping::AttributeIdentifier;
use crate::cache::quantization::{quantize, Quantization};
use crate::Store;

impl Store {
    pub fn reconcile_heads(&mut self) {
        self.cache.options.rewind = self.input.switch[2];
        self.cache.options.quantize_8 = self.input.switch[3];
        self.cache.options.quantize_6 = self.input.switch[4];
        self.cache.options.random_impulse = self.input.switch[5];

        for i in 0..4 {
            self.reconcile_head(i);
        }
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
}
