use super::debounced::Debounced;
use crate::system::hal::gpio;

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Button {
    pin: Pin,
    debounced: Debounced<4>,
    pub active: bool,
}

pub type Pin = gpio::gpiog::PG14<gpio::Input>;

impl Button {
    pub fn new(pin: Pin) -> Self {
        Self {
            pin,
            debounced: Debounced::new(),
            active: false,
        }
    }

    pub fn sample(&mut self) {
        self.active = self.debounced.update(self.pin.is_low());
    }

    pub fn active_no_filter(&self) -> bool {
        self.pin.is_low()
    }
}
