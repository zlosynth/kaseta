use crate::system::inputs::Inputs;

pub fn sample_until_button_is_clicked(inputs: &mut Inputs) {
    loop {
        let was_down = inputs.button.active;
        inputs.sample();
        let is_down = inputs.button.active;
        if !was_down && is_down {
            break;
        }
        // FIXME: At least 4 ms of break are needed, otherwise there is
        // crosstalk between channels. With 3 ms it can turn parameter by 1 %.
        // With 1 ms it was even 25 %.
        cortex_m::asm::delay(4 * (480_000_000 / 1000));
    }
}
