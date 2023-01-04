use crate::system::hal::gpio;

// XXX: If the length of this changes, it must be reflected in `mod_32`.
const SEQUENCE: [bool; 32] = [
    true, true, false, true, false, false, true, false, true, true, true, false, true, false,
    false, false, false, false, false, true, true, false, true, false, true, true, true, false,
    false, false, false, true,
];

#[derive(defmt::Format)]
pub struct Broadcaster {
    position: usize,
    pin: BroadcasterPin,
}

pub type BroadcasterPin = gpio::gpioc::PC13<gpio::Output>;

impl Broadcaster {
    pub fn new(pin: BroadcasterPin) -> Self {
        let mut broadcaster = Self { position: 0, pin };
        broadcaster.tick(); // Make sure to start in the first position
        broadcaster
    }

    pub fn tick(&mut self) {
        let value = SEQUENCE[self.position];
        self.position = mod_32(self.position + 1);
        self.pin.set_state(value.into());
    }
}

#[derive(Default, defmt::Format)]
pub struct Detector {
    position: usize,
    queue: [bool; SEQUENCE.len()],
    detected_cache: bool,
}

impl Detector {
    pub fn write(&mut self, value: bool) {
        self.queue[self.position] = value;
        self.position = mod_32(self.position + 1);
    }

    pub fn detected(&mut self) -> bool {
        if self.position == 0 {
            let unmatched: u32 = self
                .queue
                .iter()
                .zip(&SEQUENCE)
                .map(|(q, s)| u32::from(q != s))
                .sum();
            self.detected_cache = unmatched <= 2;
        }
        self.detected_cache
    }
}

fn mod_32(x: usize) -> usize {
    x & 0b1_1111
}
