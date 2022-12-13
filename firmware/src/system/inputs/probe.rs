// TODO: Try to optimize this by removing modulo, either with wrapping
// using pow2 or just comparing with N - 1.

use crate::system::hal::gpio;

const SEQUENCE: [bool; 32] = [
    true, true, false, true, false, false, true, false, true, true, true, false, true, false,
    false, false, false, false, false, true, true, false, true, false, true, true, true, false,
    false, false, false, true,
];

pub struct Broadcaster {
    position: usize,
    pin: BroadcasterPin,
}

pub type BroadcasterPin = gpio::gpioc::PC13<gpio::Output>;

impl Broadcaster {
    pub fn new(pin: BroadcasterPin) -> Self {
        let mut broadcaster = Self { position: 0, pin };
        // Make sure to start in the first position.
        broadcaster.tick();
        broadcaster
    }

    pub fn tick(&mut self) {
        let value = SEQUENCE[self.position];
        self.position = (self.position + 1) % SEQUENCE.len();
        self.pin.set_state(value.into());
    }
}

#[derive(Default)]
pub struct Detector {
    position: usize,
    queue: [bool; SEQUENCE.len()],
    detected_cache: bool,
}

impl Detector {
    pub fn write(&mut self, value: bool) {
        self.queue[self.position] = value;
        self.position = (self.position + 1) % SEQUENCE.len();
    }

    pub fn detected(&mut self) -> bool {
        if self.position == 0 {
            let unmatched: u32 = self
                .queue
                .iter()
                .zip(&SEQUENCE)
                .map(|(q, s)| if q == s { 0 } else { 1 })
                .sum();
            self.detected_cache = unmatched <= 2;
        }
        self.detected_cache
    }
}
