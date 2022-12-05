use crate::system::hal::gpio;

pub struct Leds {
    pins: Pins,
}

pub struct Pins {
    pub display: (
        Display1Pin,
        Display2Pin,
        Display3Pin,
        Display4Pin,
        Display5Pin,
        Display6Pin,
        Display7Pin,
        Display8Pin,
    ),
    pub impulse: ImpulsePin,
}

pub type DisplayConfig = [bool; 8];

type Display1Pin = gpio::gpioc::PC3<gpio::Output>;
type Display2Pin = gpio::gpiod::PD2<gpio::Output>;
type Display3Pin = gpio::gpioc::PC9<gpio::Output>;
type Display4Pin = gpio::gpioc::PC11<gpio::Output>;
type Display5Pin = gpio::gpiod::PD3<gpio::Output>;
type Display6Pin = gpio::gpioc::PC2<gpio::Output>;
type Display7Pin = gpio::gpioc::PC10<gpio::Output>;
type Display8Pin = gpio::gpiob::PB4<gpio::Output>;
type ImpulsePin = gpio::gpioc::PC12<gpio::Output>;

impl Leds {
    #[must_use]
    pub fn new(pins: Pins) -> Self {
        Self { pins }
    }

    pub fn set_display_config(&mut self, config: DisplayConfig) {
        self.pins.display.0.set_state(config[0].into());
        self.pins.display.1.set_state(config[1].into());
        self.pins.display.2.set_state(config[2].into());
        self.pins.display.3.set_state(config[3].into());
        self.pins.display.4.set_state(config[4].into());
        self.pins.display.5.set_state(config[5].into());
        self.pins.display.6.set_state(config[6].into());
        self.pins.display.7.set_state(config[7].into());
    }

    pub fn set_impulse(&mut self, on: bool) {
        self.pins.impulse.set_state(on.into());
    }
}
