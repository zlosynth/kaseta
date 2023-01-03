#![no_std]
#![no_main]

use kaseta_firmware as _; // Panic handler.

#[defmt_test::tests]
mod tests {
    use kaseta_firmware::system::inputs::Inputs;
    use kaseta_firmware::system::System;
    use kaseta_firmware::testlib::sample_until_button_is_clicked;

    #[init]
    fn init() -> Inputs {
        let cp = cortex_m::Peripherals::take().unwrap();
        let dp = daisy::pac::Peripherals::take().unwrap();
        System::init(cp, dp).inputs
    }

    #[test]
    fn button_detects_click(inputs: &mut Inputs) {
        defmt::info!("Click the button");
        sample_until_button_is_clicked(inputs);
        defmt::info!("OK");
    }
}
