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
mod pots;
mod switches;

use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::pac::{ADC1, ADC2};

use self::button::{Button, Pin as ButtonPin};
use self::cvs::CVs;
pub use self::cvs::Pins as CVsPins;
use self::multiplexer::Multiplexer;
pub use self::multiplexer::Pins as MultiplexerPins;
pub use self::pots::Pins as PotsPins;
use self::pots::Pots;
pub use self::switches::Pins as SwitchesPins;
use self::switches::Switches;

pub struct Inputs {
    cvs: CVs,
    pots: Pots,
    button: Button,
    switches: Switches,
    multiplexer: Multiplexer,
    adc_1: Adc<ADC1, Enabled>,
    adc_2: Adc<ADC2, Enabled>,
}

pub struct Config {
    pub cvs: CVsPins,
    pub pots: PotsPins,
    pub button: ButtonPin,
    pub switches: SwitchesPins,
    pub multiplexer: MultiplexerPins,
    pub adc_1: Adc<ADC1, Enabled>,
    pub adc_2: Adc<ADC2, Enabled>,
}

impl Inputs {
    pub fn new(config: Config) -> Self {
        Self {
            cvs: CVs::new(config.cvs),
            pots: Pots::new(config.pots),
            button: Button::new(config.button),
            switches: Switches::new(config.switches),
            multiplexer: Multiplexer::new(config.multiplexer),
            adc_1: config.adc_1,
            adc_2: config.adc_2,
        }
    }

    pub fn sample(&mut self) {
        self.multiplexer.select(0);
        self.switches.sample(0);
        self.pots.sample(0, &mut self.adc_1, &mut self.adc_2);
        self.cvs.sample(&mut self.adc_1, &mut self.adc_2);
        self.button.sample();
    }
}
