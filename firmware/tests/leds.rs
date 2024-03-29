#![no_std]
#![no_main]

use kaseta_firmware as _; // Panic handler

#[defmt_test::tests]
mod tests {
    use kaseta_firmware::system::System;
    use kaseta_firmware::testlib::sample_until_button_is_clicked;

    #[init]
    fn init() -> System {
        let cp = cortex_m::Peripherals::take().unwrap();
        let dp = daisy::pac::Peripherals::take().unwrap();
        System::init(cp, dp)
    }

    #[test]
    fn leds_go_on_and_off(system: &mut System) {
        defmt::info!("Click the button");
        sample_until_button_is_clicked(&mut system.inputs);

        system.outputs.leds.set_display_config([false; 8]);
        system.outputs.leds.set_impulse(false);
        defmt::info!("Click the button if all leds are dimmed");
        sample_until_button_is_clicked(&mut system.inputs);

        system.outputs.leds.set_display_config([true; 8]);
        system.outputs.leds.set_impulse(true);
        defmt::info!("Click the button if all leds are lit up");
        sample_until_button_is_clicked(&mut system.inputs);
    }
}
