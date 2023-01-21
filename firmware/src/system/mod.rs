pub mod audio;
pub mod inputs;
pub mod outputs;
pub mod randomizer;
pub mod storage;

pub use daisy::hal;
use daisy::sdram::SDRAM;

use daisy::flash::Flash;
use daisy::led::LedUser;
use hal::adc::{AdcSampleTime, Resolution};
use hal::delay::DelayFromCountDownTimer;
use hal::pac::CorePeripherals;
use hal::pac::Peripherals as DevicePeripherals;
use hal::prelude::*;
use hal::time::Hertz;
use systick_monotonic::Systick;

use self::audio::Audio;
use self::inputs::{CVsPins, Config as InputsConfig, Inputs, MultiplexerPins, PotsPins};
use self::outputs::{Config as OutputsConfig, LedsPins, Outputs};
use self::randomizer::Randomizer;

pub struct System {
    pub mono: Systick<1000>,
    pub status_led: LedUser,
    pub inputs: Inputs,
    pub outputs: Outputs,
    pub sdram: SDRAM,
    pub audio: Audio,
    pub flash: Flash,
    pub randomizer: Randomizer,
    pub frequency: Hertz,
}

impl System {
    /// Initialize system abstraction.
    ///
    /// # Panics
    ///
    /// The system can be initialized only once. It panics otherwise.
    #[must_use]
    pub fn init(mut cp: CorePeripherals, dp: DevicePeripherals) -> Self {
        enable_cache(&mut cp);

        let board = daisy::Board::take().unwrap();
        let ccdr = daisy::board_freeze_clocks!(board, dp);
        let pins = daisy::board_split_gpios!(board, ccdr, dp);
        let sdram = daisy::board_split_sdram!(cp, dp, ccdr, pins);
        let audio = Audio::init(daisy::board_split_audio!(ccdr, pins));

        let mut delay = DelayFromCountDownTimer::new(dp.TIM2.timer(
            100.Hz(),
            ccdr.peripheral.TIM2,
            &ccdr.clocks,
        ));

        let (adc_1, adc_2) = {
            let (mut adc_1, mut adc_2) = hal::adc::adc12(
                dp.ADC1,
                dp.ADC2,
                &mut delay,
                ccdr.peripheral.ADC12,
                &ccdr.clocks,
            );
            adc_1.set_resolution(Resolution::SIXTEENBIT);
            adc_1.set_sample_time(AdcSampleTime::T_16);
            adc_2.set_resolution(Resolution::SIXTEENBIT);
            adc_2.set_sample_time(AdcSampleTime::T_16);
            (adc_1.enable(), adc_2.enable())
        };

        let mono = Systick::new(cp.SYST, 480_000_000);
        let status_led = daisy::board_split_leds!(pins).USER;
        let inputs = Inputs::new(InputsConfig {
            cvs: CVsPins {
                cv_1: pins.GPIO.PIN_C6.into_analog(),
                cv_2: pins.GPIO.PIN_C8.into_analog(),
                cv_3: pins.GPIO.PIN_C7.into_analog(),
                cv_4: pins.GPIO.PIN_C9.into_analog(),
            },
            pots: PotsPins {
                multiplexer_1: pins.GPIO.PIN_C2.into_analog(),
                multiplexer_2: pins.GPIO.PIN_C4.into_analog(),
                multiplexer_3: pins.GPIO.PIN_C3.into_analog(),
            },
            button: pins.GPIO.PIN_B9.into_floating_input(),
            multiplexer: MultiplexerPins {
                address_a: pins.GPIO.PIN_A3.into_push_pull_output(),
                address_b: pins.GPIO.PIN_A8.into_push_pull_output(),
                address_c: pins.GPIO.PIN_A9.into_push_pull_output(),
            },
            // FIXME: Based on the layout, this should be B6. There is a mismatch between
            // datasheet https://static1.squarespace.com/static/58d03fdc1b10e3bf442567b8/t/628bc1307a1e2b5bc04af099/1653326133665/ES_Patch_SM_datasheet_v1.0.4.pdf
            // and lib daisy https://github.com/electro-smith/libDaisy/blob/master/src/daisy_patch_sm.cpp
            // report this issue, and fix it in my daisy library.
            probe: pins.GPIO.PIN_B5.into_push_pull_output(),
            adc_1,
            adc_2,
        });
        let outputs = Outputs::new(OutputsConfig {
            leds: LedsPins {
                display: (
                    pins.GPIO.PIN_D9.into_push_pull_output(),
                    pins.GPIO.PIN_D7.into_push_pull_output(),
                    pins.GPIO.PIN_D4.into_push_pull_output(),
                    pins.GPIO.PIN_D2.into_push_pull_output(),
                    pins.GPIO.PIN_D10.into_push_pull_output(),
                    pins.GPIO.PIN_D8.into_push_pull_output(),
                    pins.GPIO.PIN_D3.into_push_pull_output(),
                    pins.GPIO.PIN_D1.into_push_pull_output(),
                ),
                impulse: pins.GPIO.PIN_D6.into_push_pull_output(),
            },
            // FIXME: Based on the layout, this should be B5. There is a mismatch between
            // datasheet https://static1.squarespace.com/static/58d03fdc1b10e3bf442567b8/t/628bc1307a1e2b5bc04af099/1653326133665/ES_Patch_SM_datasheet_v1.0.4.pdf
            // and lib daisy https://github.com/electro-smith/libDaisy/blob/master/src/daisy_patch_sm.cpp
            // report this issue, and fix it in my daisy library.
            impulse: pins.GPIO.PIN_B6.into_push_pull_output(),
        });
        let flash = daisy::board_split_flash!(ccdr, dp, pins);
        let randomizer = Randomizer::new(dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks));
        let frequency = ccdr.clocks.sys_ck();

        Self {
            mono,
            status_led,
            inputs,
            outputs,
            sdram,
            audio,
            flash,
            randomizer,
            frequency,
        }
    }
}

/// AN5212: Improve application performance when fetching instruction and
/// data, from both internal andexternal memories.
fn enable_cache(cp: &mut CorePeripherals) {
    cp.SCB.enable_icache();
    // NOTE: This requires cache management around all use of DMA.
    cp.SCB.enable_dcache(&mut cp.CPUID);
}
