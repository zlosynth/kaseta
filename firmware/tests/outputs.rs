#![no_std]
#![no_main]

use kaseta_firmware as _; // memory layout + panic handler

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
        sample_until_button_is_clicked(&mut system.inputs);
        defmt::info!("Click the button if all leds are lit up");
    }

    #[test]
    fn impulse_output_acts_as_a_trigger(system: &mut System) {
        const MS: u32 = 480_000_000 / 1000;

        defmt::info!("Connect a trigger destination to impulse output, and click the button");
        sample_until_button_is_clicked(&mut system.inputs);

        defmt::info!("Click the button and confirm that there are 4 regular triggers");
        sample_until_button_is_clicked(&mut system.inputs);
        for _ in 0..4 {
            system.outputs.impulse.set(true);
            cortex_m::asm::delay(10 * MS);
            system.outputs.impulse.set(false);
            cortex_m::asm::delay(990 * MS);
        }

        defmt::info!("Click the button to end this test");
        sample_until_button_is_clicked(&mut system.inputs);
    }
}
