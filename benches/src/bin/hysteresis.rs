//! Hysteresis benchmark.
//!
//! Measuring how many DWT cycles it takes for a buffer of 128 random samples
//! (x4 oversampling) to be processed by hysteresis simulation.
//!
//! * Original signal-based implementation: 7701331
//! * Enable icache: 3048425
//! * Enable dcache: 2989626
//! * Replacing f64 with f32: 426773
//! * Using RK2 instead of RK4: 356607
//! * Remove attribute smoothening: 301686
//! * Remove signal abstraction: 63257

#![no_main]
#![no_std]
#![allow(clippy::similar_names)]

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;

use kaseta_benches as _;
use kaseta_benches::{op_cyccnt_diff, random_buffer};

use kaseta_dsp::hysteresis::{Attributes, Hysteresis};

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32 * 4;

    defmt::println!("Hysteresis benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let mut randomizer = dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let mut hysteresis = Hysteresis::new(48_000.0);
    hysteresis.set_attributes(Attributes {
        dry_wet: 0.5,
        drive: 0.5,
        saturation: 0.5,
        width: 0.5,
    });

    let mut buffer: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            hysteresis.process(&mut buffer);
        }
    });

    defmt::println!("Cycles per oversampled buffer: {}", cycles / 300);

    kaseta_benches::exit()
}
