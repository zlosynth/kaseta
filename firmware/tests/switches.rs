#![no_std]
#![no_main]

use kaseta_firmware as _; // memory layout + panic handler

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
    fn all_switches_work(inputs: &mut Inputs) {
        defmt::info!("Set all switches down, then click the button");

        sample_until_button_is_clicked(inputs);
        for i in 0..10 {
            defmt::assert!(
                !inputs.switches.switch[i].value,
                "Switch {:?} is not down",
                i + 1,
            );
        }
        defmt::info!("OK");

        defmt::info!("Set switches up, from left to right, click the button after each");
        for i in 0..10 {
            sample_until_button_is_clicked(inputs);
            defmt::assert!(inputs.switches.switch[i].value);
            defmt::info!("OK");
        }
    }
}
