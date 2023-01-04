use crate::system::inputs::Inputs;

pub fn sample_until_button_is_clicked(inputs: &mut Inputs) {
    loop {
        let was_down = inputs.button.active;
        inputs.sample();
        let is_down = inputs.button.active;
        if !was_down && is_down {
            break;
        }
        cortex_m::asm::delay(480_000_000 / 1000);
    }
}
