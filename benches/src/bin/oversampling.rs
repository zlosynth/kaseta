//! Oversampling benchmark.
//!
//! Measuring how many DWT cycles it takes for X random samples to be upsampled
//! by 4 and then downsampled.
//!
//! * Original signal-based implementation:

#![no_main]
#![no_std]

use kaseta_benches as _;

use daisy::pac::{DWT};
use daisy::hal::prelude::_stm32h7xx_hal_rng_RngCore;
use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use daisy::hal::rng::Rng;

macro_rules! op_cyccnt_diff {
    ( $cp:expr, $x:expr ) => {
        {
            use core::sync::atomic::{self, Ordering};

            $cp.DCB.enable_trace();
            $cp.DWT.enable_cycle_counter();

            atomic::compiler_fence(Ordering::Acquire);
            let before = DWT::cycle_count();
            $x
            let after = DWT::cycle_count();
            atomic::compiler_fence(Ordering::Release);

            if after >= before {
                after - before
            } else {
                after + (u32::MAX - before)
            }
        }
    };
}


fn random_buffer(randomizer: &mut Rng) -> [f32; 32] {
    let mut buffer = [0.0; 32];
    for x in buffer.iter_mut() {
        let r: u16 = randomizer.gen().unwrap();
        *x = r as f32 / (2 << 14) as f32 - 1.0;
    }
    buffer
}


#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Oversampling benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let mut randomizer = dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..48000 / 32 {
            let _buffer = random_buffer(&mut randomizer);
        }
    });

    defmt::println!("Cycles: {}", cycles);

    kaseta_benches::exit()
}
