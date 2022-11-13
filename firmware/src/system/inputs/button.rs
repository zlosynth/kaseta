use super::debounced::Debounced;
use crate::system::hal::gpio;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Button {
    pin: Pin,
    debounced: Debounced<4>,
    active: bool,
    clicked: bool,
}

pub type Pin = gpio::gpiog::PG13<gpio::Input>;

impl Button {
    pub fn new(pin: Pin) -> Self {
        Self {
            pin,
            debounced: Debounced::new(),
            active: false,
            clicked: false,
        }
    }

    pub fn sample(&mut self) {
        let was_active = self.active;
        self.active = self.debounced.update(self.pin.is_low());
        self.clicked = !was_active && self.active;
    }

    pub fn clicked(&self) -> bool {
        self.clicked
    }
}
