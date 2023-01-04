//! Abstraction of all inputs except for audio.
//!
//! All of these are grouped under a single abstraction to allow optimization
//! Through mutex cycling and shared ADC.
//!
//! Audio input is kept outside of this abstraction, since it runs on a
//! different frequency and is triggered through interrupts.

mod button;
mod cvs;
mod debounced;
mod multiplexer;
pub mod pots;
mod probe;
mod switches;

use kaseta_control::{InputSnapshot as Snapshot, InputSnapshotHead as SnapshotHead};

use crate::system::hal::adc::{Adc, Enabled};
use crate::system::hal::pac::{ADC1, ADC2};

use self::button::{Button, Pin as ButtonPin};
use self::cvs::CVs;
pub use self::cvs::Pins as CVsPins;
use self::multiplexer::Multiplexer;
pub use self::multiplexer::Pins as MultiplexerPins;
pub use self::pots::Pins as PotsPins;
use self::pots::Pots;
use self::probe::{Broadcaster as ProbeBroadcaster, BroadcasterPin as ProbeBroadcasterPin};
pub use self::switches::Pins as SwitchesPins;
use self::switches::Switches;

// To avoid crosstalk, it is necessary to let multiplexer settle after
// the source was changed.
const STABILIZATION_TICKS: u8 = 3;

pub struct Inputs {
    pub cvs: CVs,
    pub pots: Pots,
    pub button: Button,
    pub switches: Switches,
    multiplexer: Multiplexer,
    probe: ProbeBroadcaster,
    adc_1: Adc<ADC1, Enabled>,
    adc_2: Adc<ADC2, Enabled>,
    cycle: u8,
    stabilization: u8,
}

pub struct Config {
    pub cvs: CVsPins,
    pub pots: PotsPins,
    pub button: ButtonPin,
    pub switches: SwitchesPins,
    pub multiplexer: MultiplexerPins,
    pub probe: ProbeBroadcasterPin,
    pub adc_1: Adc<ADC1, Enabled>,
    pub adc_2: Adc<ADC2, Enabled>,
}

impl Inputs {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            cvs: CVs::new(config.cvs),
            pots: Pots::new(config.pots),
            button: Button::new(config.button),
            switches: Switches::new(config.switches),
            multiplexer: Multiplexer::new(config.multiplexer),
            probe: ProbeBroadcaster::new(config.probe),
            adc_1: config.adc_1,
            adc_2: config.adc_2,
            cycle: 0,
            stabilization: 0,
        }
    }

    pub fn sample(&mut self) {
        self.cvs.sample(&mut self.adc_1, &mut self.adc_2);
        self.button.sample();

        self.stabilization += 1;
        if self.stabilization == STABILIZATION_TICKS {
            self.stabilization = 0;

            self.switches.sample(self.cycle);
            self.pots
                .sample(self.cycle, &mut self.adc_1, &mut self.adc_2);

            // XXX: Selection happens at the end so the signal gets a chance
            // to propagate to mux before the next reading cycle.
            self.cycle = Multiplexer::next_position(self.cycle);
            self.multiplexer.select(self.cycle);
        }

        // XXX: Selection happens at the end so the signal gets a chance
        // to propagate to probe detectors before the next reading cycle.
        self.probe.tick();
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        let mut control = [None; 4];
        for (i, cv) in self.cvs.cv.iter().enumerate() {
            control[i] = cv.value;
        }

        let mut switch = [false; 10];
        for (i, sw) in self.switches.switch.iter().enumerate() {
            switch[i] = sw.value;
        }

        let mut head = [SnapshotHead::default(); 4];
        for (i, h) in self.pots.head.iter().enumerate() {
            head[i] = SnapshotHead {
                position: h.position,
                volume: h.volume,
                feedback: h.feedback,
                pan: h.pan,
            };
        }

        Snapshot {
            pre_amp: self.pots.pre_amp,
            drive: self.pots.drive,
            bias: self.pots.bias,
            dry_wet: self.pots.dry_wet,
            wow_flut: self.pots.wow_flut,
            speed: self.pots.speed,
            tone: self.pots.tone,
            head,
            control,
            switch,
            button: self.button.active,
        }
    }
}
