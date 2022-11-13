use crate::system::hal::gpio;

use super::debounced::Debounced;

pub struct Switches {
    pub switch: [Switch; 10],
    pins: Pins,
}

pub struct Switch {
    debouncer: Debounced<4>,
    pub value: bool,
}

pub struct Pins {
    pub switch_1: Switch1Pin,
    pub multiplexed_switches_2_to_9: MultiplexedSwitches2To9Pin,
    pub switch_10: Switch10Pin,
}

pub type Switch1Pin = gpio::gpiog::PG14<gpio::Input>;
pub type MultiplexedSwitches2To9Pin = gpio::gpiob::PB15<gpio::Input>;
pub type Switch10Pin = gpio::gpioc::PC3<gpio::Input>;

impl Switches {
    pub(crate) fn new(pins: Pins) -> Self {
        Self {
            switch: [
                Switch::new(),
                Switch::new(),
                Switch::new(),
                Switch::new(),
                Switch::new(),
                Switch::new(),
                Switch::new(),
                Switch::new(),
                Switch::new(),
                Switch::new(),
            ],
            pins,
        }
    }

    pub(crate) fn sample(&mut self, cycle: u8) {
        match cycle {
            0 => {
                self.switch[0].update(self.pins.switch_1.is_high());
                self.switch[1].update(self.pins.multiplexed_switches_2_to_9.is_high());
            }
            1 => {
                self.switch[2].update(self.pins.multiplexed_switches_2_to_9.is_high());
                self.switch[9].update(self.pins.switch_10.is_high());
            }
            2 => self.switch[3].update(self.pins.multiplexed_switches_2_to_9.is_high()),
            3 => self.switch[4].update(self.pins.multiplexed_switches_2_to_9.is_high()),
            4 => self.switch[5].update(self.pins.multiplexed_switches_2_to_9.is_high()),
            5 => self.switch[6].update(self.pins.multiplexed_switches_2_to_9.is_high()),
            6 => self.switch[7].update(self.pins.multiplexed_switches_2_to_9.is_high()),
            7 => self.switch[8].update(self.pins.multiplexed_switches_2_to_9.is_high()),
            _ => unreachable!(),
        }
    }
}

impl Switch {
    pub(crate) fn new() -> Self {
        Self {
            debouncer: Debounced::new(),
            value: false,
        }
    }

    pub(crate) fn update(&mut self, value: bool) {
        self.value = self.debouncer.update(value);
    }
}
