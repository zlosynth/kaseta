//! Abstraction of all inputs except for audio.
//!
//! All of these are grouped under a single abstraction to allow optimization
//! Through mutex cycling and shared ADC.
//!
//! Audio input is kept outside of this abstraction, since it runs on a
//! different frequency and is triggered through interrupts.

mod button;
mod cvs;
mod debounced;
mod multiplexer;
mod switches;

use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::pac::{ADC1, ADC2};

use self::button::{Button, Pin as ButtonPin};
use self::cvs::CVs;
pub use self::cvs::Pins as CVsPins;
pub use self::multiplexer::Config as MultiplexerConfig;
use self::multiplexer::Multiplexer;
pub use self::switches::Config as SwitchesConfig;
use self::switches::Switches;

pub struct Inputs {
    // pot: [(); 23],
    cvs: CVs,
    button: Button,
    switches: Switches,
    multiplexer: Multiplexer,
    adc_1: Adc<ADC1, Enabled>,
    adc_2: Adc<ADC2, Enabled>,
}

pub struct Config {
    pub cvs: CVsPins,
    pub button: ButtonPin,
    pub switches: SwitchesConfig,
    pub multiplexer: MultiplexerConfig,
    pub adc_1: Adc<ADC1, Enabled>,
    pub adc_2: Adc<ADC2, Enabled>,
}

impl Inputs {
    pub fn new(config: Config) -> Self {
        Self {
            cvs: CVs::new(config.cvs),
            button: Button::new(config.button),
            switches: Switches::new(config.switches),
            multiplexer: Multiplexer::new(config.multiplexer),
            adc_1: config.adc_1,
            adc_2: config.adc_2,
        }
    }

    pub fn sample(&mut self) {
        self.multiplexer.select(0);
        self.cvs.sample(&mut self.adc_1, &mut self.adc_2);
        self.switches.sample(0);
        self.button.sample();
        // TODO: Update all CVs
        // TODO: Update button
        // TODO: Based on cycle, update subset of pots, and switches
        // TODO: Update the two most recently active pots even if it's not their turn
    }
}
