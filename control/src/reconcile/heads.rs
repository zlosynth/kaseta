use super::calculate;
use crate::mapping::AttributeIdentifier;
use crate::quantization::{quantize, Quantization};
use crate::Cache;

impl Cache {
    pub fn reconcile_heads(&mut self) {
        for i in 0..4 {
            self.reconcile_head(i);
        }
    }

    fn reconcile_head(&mut self, i: usize) {
        self.attributes.head[i].position = quantize(
            calculate(
                self.inputs.head[i].position.value(),
                self.control_for_attribute(AttributeIdentifier::Position(i)),
                (0.0, 1.0),
                None,
            ),
            Quantization::from((self.options.quantize_6, self.options.quantize_8)),
        );
        self.attributes.head[i].volume = calculate(
            self.inputs.head[i].volume.value(),
            self.control_for_attribute(AttributeIdentifier::Volume(i)),
            (0.0, 1.0),
            None,
        );
        self.attributes.head[i].feedback = calculate(
            self.inputs.head[i].feedback.value(),
            self.control_for_attribute(AttributeIdentifier::Feedback(i)),
            (0.0, 1.0),
            None,
        );
        self.attributes.head[i].pan = calculate(
            self.inputs.head[i].pan.value(),
            self.control_for_attribute(AttributeIdentifier::Pan(i)),
            (0.0, 1.0),
            None,
        );
    }
}
