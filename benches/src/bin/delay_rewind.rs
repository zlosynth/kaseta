//! Delay rewind benchmark.
//!
//! Measuring how many DWT cycles it takes per buffer to rewind with quarter
//! speed (two octaves lower) over a 10 seconds long random buffer.
//!
//! * Original implementation: 26879
//! * Removing check making sure speed is always above `f32::EPSILON`: 22002

#![no_main]
#![no_std]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

use kaseta_benches as _;
use kaseta_benches::{op_cyccnt_diff, random_buffer};

use core::mem::MaybeUninit;

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use sirena::memory_manager::MemoryManager;

use kaseta_dsp::delay::{Attributes, Delay, HeadAttributes};

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32;

    // With backward rewinding of 0.5 relative speed, it would cross 10 seconds
    // of delay in 20+ seconds, accounting for inertia.
    const SAMPLE_RATE: f32 = 48_000.0;
    const RELATIVE_SPEED: f32 = 0.5;
    const DELAY: f32 = 10.0;
    const BUFFERS: usize = (SAMPLE_RATE * ((DELAY / RELATIVE_SPEED) * 1.5)) as usize / BUFFER_SIZE;

    defmt::println!("Delay rewind benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let pins = daisy::board_split_gpios!(board, ccdr, dp);
    let sdram = daisy::board_split_sdram!(cp, dp, ccdr, pins);

    let mut memory_manager = {
        let ram_slice = unsafe {
            let ram_items = sdram.size() / core::mem::size_of::<MaybeUninit<u32>>();
            let ram_ptr = sdram.base_address.cast();
            core::slice::from_raw_parts_mut(ram_ptr, ram_items)
        };
        MemoryManager::from(ram_slice)
    };

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let mut randomizer = dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);
    let mut delay = Delay::new(SAMPLE_RATE, &mut memory_manager);
    delay.set_attributes(Attributes {
        length: DELAY,
        heads: [
            HeadAttributes {
                position: 1.0,
                feedback: 0.3,
                volume: 0.8,
                // TODO FIXME: Values of rewind speed don't make sense,
                // I would expect it to be 0.5, not 0.25.
                rewind_backward: Some(RELATIVE_SPEED / 2.0),
                rewind_forward: None,
            },
            HeadAttributes {
                position: 0.0,
                feedback: 0.0,
                volume: 0.0,
                rewind_backward: None,
                rewind_forward: None,
            },
            HeadAttributes {
                position: 0.0,
                feedback: 0.0,
                volume: 0.0,
                rewind_backward: None,
                rewind_forward: None,
            },
            HeadAttributes {
                position: 0.0,
                feedback: 0.0,
                volume: 0.0,
                rewind_backward: None,
                rewind_forward: None,
            },
        ],
    });

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..BUFFERS {
            let mut input: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);
            delay.process(&mut input);
        }
    });

    defmt::println!("Cycles per buffer: {}", cycles / BUFFERS as u32);

    kaseta_benches::exit()
}
