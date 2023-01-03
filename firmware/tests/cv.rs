#![no_std]
#![no_main]

use kaseta_firmware as _; // Panic handler

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
    fn cv_inputs_get_detected_and_properly_scale(inputs: &mut Inputs) {
        for i in 0..4 {
            defmt::info!("For CV {}", i + 1);

            defmt::info!("Unplug the cable, then click the button");
            sample_until_button_is_clicked(inputs);
            defmt::assert!(inputs.cvs.cv[i].value.is_none());

            defmt::info!("Plug in a cable, then click the button");
            sample_until_button_is_clicked(inputs);
            defmt::assert!(inputs.cvs.cv[i].value.is_some());

            defmt::info!("Set the input to -5 V, then click the button");
            sample_until_button_is_clicked(inputs);
            defmt::assert!(inputs.cvs.cv[i].value.unwrap() < 0.01);

            defmt::info!("Set the input to +5 V, then click the button");
            sample_until_button_is_clicked(inputs);
            defmt::assert!(inputs.cvs.cv[i].value.unwrap() > 0.98);

            defmt::info!("OK");
        }
    }
}
