//! Oscillator benchmark.
//!
//! Measuring how many DWT cycles it takes to populate a buffer of 32
//! samples with oscillator.
//!
//! * Original implementation: 76363
//! * Replacing libm sine with micromath: 2404

#![no_main]
#![no_std]

use core::hint::black_box;

use kaseta_benches as _;
use kaseta_benches::op_cyccnt_diff;

use kaseta_dsp::oscillator::{Attributes, Oscillator};

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32;

    defmt::println!("Oscillator benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let mut oscillator = Oscillator::new(48_000.0);
    oscillator.set_attributes(&Attributes { frequency: 220.0 });

    let mut buffer = [0.0; BUFFER_SIZE];

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            oscillator.populate(black_box(&mut buffer));
        }
    });

    defmt::println!("Cycles per buffer: {}", cycles / 300);

    kaseta_benches::exit()
}
