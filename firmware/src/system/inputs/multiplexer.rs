use crate::system::hal::gpio;

pub struct Multiplexer {
    pins: Pins,
}

pub struct Pins {
    pub address_a: AddressAPin,
    pub address_b: AddressBPin,
    pub address_c: AddressCPin,
}

type AddressAPin = gpio::gpioa::PA0<gpio::Output>;
type AddressBPin = gpio::gpiob::PB14<gpio::Output>;
type AddressCPin = gpio::gpiob::PB15<gpio::Output>;

impl Multiplexer {
    pub fn new(pins: Pins) -> Self {
        Self { pins }
    }

    pub fn select(&mut self, position: u8) {
        let first_bit = (position & 0b1 != 0).into();
        self.pins.address_a.set_state(first_bit);

        let second_bit = (position & 0b10 != 0).into();
        self.pins.address_b.set_state(second_bit);

        let third_bit = (position & 0b100 != 0).into();
        self.pins.address_c.set_state(third_bit);
    }

    pub fn next_position(mut position: u8) -> u8 {
        position += 1;
        position &= 0b111;
        position
    }
}
