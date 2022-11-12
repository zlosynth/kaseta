use crate::system::hal::gpio;

use super::debounced::Debounced;

pub struct Switches {
    switch: [Debounced<4>; 10],
    pins: Pins,
}

// TODO: Use Pins instead of Config
struct Pins {
    switch_1: Switch1Pin,
    multiplexed_witches_2_to_9: MultiplexedSwitches2To9Pin,
    switch_10: Switch10Pin,
}

pub struct Config {
    pub switch_1: Switch1Pin,
    pub multiplexed_switches_2_to_9: MultiplexedSwitches2To9Pin,
    pub switch_10: Switch10Pin,
}

pub type Switch1Pin = gpio::gpiog::PG14<gpio::Input>;
pub type MultiplexedSwitches2To9Pin = gpio::gpiob::PB15<gpio::Input>;
pub type Switch10Pin = gpio::gpioc::PC3<gpio::Input>;

impl Switches {
    pub fn new(config: Config) -> Self {
        Self {
            switch: [
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
                Debounced::new(),
            ],
            pins: Pins {
                switch_1: config.switch_1,
                multiplexed_witches_2_to_9: config.multiplexed_switches_2_to_9,
                switch_10: config.switch_10,
            },
        }
    }

    pub fn sample(&mut self, cycle: u8) {
        match cycle {
            0 => {
                self.switch[0].update(self.pins.switch_1.is_high());
                self.switch[1].update(self.pins.multiplexed_witches_2_to_9.is_high());
            }
            1 => {
                self.switch[2].update(self.pins.multiplexed_witches_2_to_9.is_high());
                self.switch[9].update(self.pins.switch_10.is_high());
            }
            2 => self.switch[3].update(self.pins.multiplexed_witches_2_to_9.is_high()),
            3 => self.switch[4].update(self.pins.multiplexed_witches_2_to_9.is_high()),
            4 => self.switch[5].update(self.pins.multiplexed_witches_2_to_9.is_high()),
            5 => self.switch[6].update(self.pins.multiplexed_witches_2_to_9.is_high()),
            6 => self.switch[7].update(self.pins.multiplexed_witches_2_to_9.is_high()),
            7 => self.switch[8].update(self.pins.multiplexed_witches_2_to_9.is_high()),
            _ => unreachable!(),
        }
    }
}
