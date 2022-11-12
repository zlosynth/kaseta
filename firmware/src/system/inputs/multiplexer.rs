use crate::system::hal::gpio;

type AddressAPin = gpio::gpiob::PB14<gpio::Output>;
type AddressBPin = gpio::gpioa::PA0<gpio::Output>;
type AddressCPin = gpio::gpioa::PA1<gpio::Output>;

pub struct Multiplexer {
    address_a: AddressAPin,
    address_b: AddressBPin,
    address_c: AddressCPin,
}

pub struct Config {
    pub address_a: AddressAPin,
    pub address_b: AddressBPin,
    pub address_c: AddressCPin,
}

impl Multiplexer {
    pub fn new(config: Config) -> Self {
        Self {
            address_a: config.address_a,
            address_b: config.address_b,
            address_c: config.address_c,
        }
    }

    pub fn select(&mut self, position: u8) {
        self.address_a.set_state((position & 0b1 != 0).into());
        self.address_b.set_state((position & 0b10 != 0).into());
        self.address_c.set_state((position & 0b100 != 0).into());
    }
}
