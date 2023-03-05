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
    fn pots_move_in_expected_range(inputs: &mut Inputs) {
        macro_rules! assert_pot_is_up {
            ($name:expr, $pot:expr) => {
                defmt::info!("Turn {} all the way up, then click the button", $name);
                sample_until_button_is_clicked(inputs);
                cortex_m::asm::delay(480_000_000 / 2); // Protection against accidental double-clicks
                defmt::assert!($pot > 0.9999, "Assert failed, actual value: {:?}", $pot);
                defmt::info!("OK");
            };
        }

        defmt::info!("Turn all pots to their minimum value, then click the button");
        sample_until_button_is_clicked(inputs);
        defmt::assert!(
            inputs.pots.pre_amp < 0.0001
                && inputs.pots.drive < 0.0001
                && inputs.pots.bias < 0.0001
                && inputs.pots.dry_wet < 0.0001
                && inputs.pots.wow_flut < 0.0001
                && inputs.pots.speed < 0.0001
                && inputs.pots.tone < 0.0001
                && inputs.pots.head.iter().all(|h| {
                    h.position < 0.0001
                        && h.volume < 0.0001
                        && h.feedback < 0.0001
                        && h.pan < 0.0001
                })
        );
        defmt::info!("OK");

        assert_pot_is_up!("Pre-amp", inputs.pots.pre_amp);
        assert_pot_is_up!("Drive", inputs.pots.drive);
        assert_pot_is_up!("Bias", inputs.pots.bias);
        assert_pot_is_up!("Dry/Wet", inputs.pots.dry_wet);
        assert_pot_is_up!("Wow/Flut", inputs.pots.wow_flut);
        assert_pot_is_up!("Speed", inputs.pots.speed);
        assert_pot_is_up!("Tone", inputs.pots.tone);
        for i in 0..4 {
            defmt::info!("For head {}", i);
            assert_pot_is_up!("Position", inputs.pots.head[i].position);
            assert_pot_is_up!("Volume", inputs.pots.head[i].volume);
            assert_pot_is_up!("Feedback", inputs.pots.head[i].feedback);
            assert_pot_is_up!("Pan", inputs.pots.head[i].pan);
        }
    }
}
