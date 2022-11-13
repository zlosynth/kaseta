#![no_std]
#![no_main]

use kaseta_firmware as _; // memory layout + panic handler

#[defmt_test::tests]
mod tests {
    use kaseta_firmware::system::System;

    #[init]
    fn init() -> System {
        let cp = cortex_m::Peripherals::take().unwrap();
        let dp = daisy::pac::Peripherals::take().unwrap();

        System::init(cp, dp)
    }

    #[test]
    fn pots_move_in_expected_range(_system: &mut System) {
        defmt::panic!("TODO");
    }

    #[test]
    fn all_switches_work(_system: &mut System) {
        defmt::info!("Set all switches down");
        defmt::info!("OK");
        defmt::info!("Set switches up, from left to right");
        for i in 0..10 {
            defmt::info!("OK {}", i);
        }
    }

    #[test]
    fn button_detects_click(_system: &mut System) {
        defmt::info!("Don't touch the button");
        defmt::info!("OK");
        defmt::info!("Click the button");
        defmt::info!("OK");
    }

    #[test]
    fn cv_inputs_get_detected_and_properly_scale(_system: &mut System) {
        defmt::panic!("TODO");
    }
}
