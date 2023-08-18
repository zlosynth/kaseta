//! Compressor benchmark.
//!
//! Measuring how many DWT cycles it takes for a buffer of 32 random samples
//! to be processed by the compressor.
//!
//! * Original implementation: 10793
//! * Replacing `max` with `if else`: 9800

#![no_main]
#![no_std]
#![allow(clippy::similar_names)]

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;

use kaseta_benches as _;
use kaseta_benches::{op_cyccnt_diff, random_buffer};

use kaseta_dsp::compressor::Compressor;

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32;

    defmt::println!("Compressor benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let mut randomizer = dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let mut compressor = Compressor::new(48_000.0);

    let mut buffer_left: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);
    let mut buffer_right: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            compressor.process(&mut buffer_left, &mut buffer_right);
        }
    });

    defmt::println!("Cycles per oversampled buffer: {}", cycles / 300);

    kaseta_benches::exit()
}
