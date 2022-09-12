//! Hysteresis benchmark.
//!
//! Measuring how many DWT cycles it takes for a buffer of 128 random samples
//! (x4 oversampling) to be processed by hysteresis simulation.
//!
//! * Original signal-based implementation: 7701331
//! * Enable icache: 3048425
//! * Enable dcache: 2989626
//! * Processing the whole buffer at once:
//!
//! TODO: Unroll loops
//! TODO: Move sinc tables to different memories
//! TODO: Keep ring buffer on stack
//! TODO: Check more in CMSIS
//! TODO: Apply formatting and checks to this module too

#![no_main]
#![no_std]
#![allow(clippy::similar_names)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]

use kaseta_benches as _;
use kaseta_benches::op_cyccnt_diff;

use kaseta_dsp::hysteresis::{Attributes, Hysteresis, SignalApplyHysteresis};
use sirena::signal::{self, Signal};

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32 * 4;

    defmt::println!("Hysteresis benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    const FS: f32 = 48_000.0;
    const FREQ: f32 = 100.0;
    let mut input = signal::sine(FS, FREQ);

    const DRIVE: f32 = 0.5;
    const SATURATION: f32 = 0.5;
    const WIDTH: f32 = 0.5;
    let mut hysteresis = Hysteresis::new(FS);
    hysteresis.set_attributes(Attributes {
        drive: DRIVE,
        saturation: SATURATION,
        width: WIDTH,
    });

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..30 {
            let mut buffer = [0.0; BUFFER_SIZE];
            let mut instrument = input
                .by_ref()
                .apply_hysteresis(&mut hysteresis);
            for x in &mut buffer {
                *x = instrument.next();
            }
        }
    });

    defmt::println!("Cycles per oversampled buffer: {}", cycles / 30);

    kaseta_benches::exit()
}
