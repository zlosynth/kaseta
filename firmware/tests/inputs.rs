#![no_std]
#![no_main]

use kaseta_firmware as _; // memory layout + panic handler
use kaseta_firmware::system::inputs::Inputs;

#[defmt_test::tests]
mod tests {
    use super::sample_until_button_is_clicked;
    use kaseta_firmware::system::inputs::Inputs;
    use kaseta_firmware::system::System;

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
                defmt::assert!($pot > 0.99);
                defmt::info!("OK");
            };
        }

        defmt::info!("Turn all pots to their minimum value, then click the button");
        sample_until_button_is_clicked(inputs);
        defmt::assert!(
            inputs.pots.pre_amp < 0.01
                && inputs.pots.drive < 0.01
                && inputs.pots.bias < 0.01
                && inputs.pots.dry_wet < 0.01
                && inputs.pots.wow_flut < 0.01
                && inputs.pots.speed < 0.01
                && inputs.pots.tone < 0.01
                && inputs.pots.head.iter().all(|h| {
                    h.position < 0.01 && h.volume < 0.01 && h.feedback < 0.01 && h.pan < 0.01
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

    #[test]
    fn all_switches_work(inputs: &mut Inputs) {
        defmt::info!("Set all switches down, then click the button");
        sample_until_button_is_clicked(inputs);
        for i in 0..10 {
            defmt::assert!(!inputs.switches.switch[i].value);
        }
        defmt::info!("OK");

        defmt::info!("Set switches up, from left to right, click the button after each");
        for i in 0..10 {
            sample_until_button_is_clicked(inputs);
            defmt::assert!(inputs.switches.switch[i].value);
            defmt::info!("OK");
        }
    }

    #[test]
    fn button_detects_click(inputs: &mut Inputs) {
        defmt::info!("Click the button");
        sample_until_button_is_clicked(inputs);
        defmt::info!("OK");
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
            defmt::assert!(inputs.cvs.cv[i].value.unwrap() > 0.99);

            defmt::info!("OK");
        }
    }
}

fn sample_until_button_is_clicked(inputs: &mut Inputs) {
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
