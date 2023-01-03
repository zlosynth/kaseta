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
    fn impulse_output_acts_as_a_trigger(system: &mut System) {
        let ms = system.frequency.to_kHz();

        defmt::info!("Connect a trigger destination to impulse output, and click the button");
        sample_until_button_is_clicked(&mut system.inputs);

        defmt::info!("Click the button and confirm that there are 4 regular triggers");
        sample_until_button_is_clicked(&mut system.inputs);
        for _ in 0..4 {
            system.outputs.impulse.set(true);
            cortex_m::asm::delay(10 * ms);
            system.outputs.impulse.set(false);
            cortex_m::asm::delay(990 * ms);
        }

        defmt::info!("Click the button to end this test");
        sample_until_button_is_clicked(&mut system.inputs);
    }
}
