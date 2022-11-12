//! Abstraction of all inputs except for audio.
//!
//! All of these are grouped under a single abstraction to allow optimization
//! Through mutex cycling and shared ADC.
//!
//! Audio input is kept outside of this abstraction, since it runs on a
//! different frequency and is triggered through interrupts.

mod button;
mod debounced;
mod multiplexer;
mod switches;

use button::{Button, Pin as ButtonPin};
pub use multiplexer::Config as MultiplexerConfig;
use multiplexer::Multiplexer;
pub use switches::Config as SwitchesConfig;
use switches::Switches;

pub struct Inputs {
    // cv: (CV, CV, CV, CV),
    // pot: [(); 23],
    button: Button,
    switches: Switches,
    multiplexer: Multiplexer,
    // adc1: (),
    // adc2: (),
}

pub struct Config {
    pub button: ButtonPin,
    pub switches: SwitchesConfig,
    pub multiplexer: MultiplexerConfig,
}

impl Inputs {
    pub fn new(config: Config) -> Self {
        Self {
            button: Button::new(config.button),
            switches: Switches::new(config.switches),
            multiplexer: Multiplexer::new(config.multiplexer),
        }
    }

    pub fn sample(&mut self) {
        self.multiplexer.select(0);
        self.switches.sample(0);
        self.button.sample();
        // TODO: Update all CVs
        // TODO: Update button
        // TODO: Based on cycle, update subset of pots, and switches
        // TODO: Update the two most recently active pots even if it's not their turn
    }
}
