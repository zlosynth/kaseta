use crate::system::hal::gpio;

pub struct Impulse {
    pin: Pin,
}

pub type Pin = gpio::gpioc::PC13<gpio::Output>;

impl Impulse {
    #[must_use]
    pub fn new(pin: Pin) -> Self {
        Self { pin }
    }

    pub fn set(&mut self, on: bool) {
        self.pin.set_state(on.into());
    }
}
