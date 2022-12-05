mod impulse;
mod leds;

use kaseta_control::DesiredOutput;

use self::impulse::{Impulse, Pin as ImpulsePin};
use self::leds::Leds;
pub use self::leds::Pins as LedsPins;

pub struct Outputs {
    pub leds: Leds,
    pub impulse: Impulse,
}

pub struct Config {
    pub leds: LedsPins,
    pub impulse: ImpulsePin,
}

impl Outputs {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            leds: Leds::new(config.leds),
            impulse: Impulse::new(config.impulse),
        }
    }

    pub fn set(&mut self, desired: &DesiredOutput) {
        self.leds.set_display_config(desired.display);
        self.leds.set_impulse(desired.impulse_led);
        self.impulse.set(desired.impulse_trigger);
    }
}
