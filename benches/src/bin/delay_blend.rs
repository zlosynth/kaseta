//! Delay blend benchmark.
//!
//! Measuring how many DWT cycles it takes per buffer to blend between two
//! delay positions.
//!
//! * Original implementation: 56075

#![no_main]
#![no_std]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use kaseta_benches as _;
use kaseta_benches::{op_cyccnt_diff, random_buffer};

use core::mem::MaybeUninit;

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use sirena::memory_manager::MemoryManager;

use kaseta_dsp::delay::{Attributes, Delay, HeadAttributes};

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32;
    const SAMPLE_RATE: f32 = 48_000.0;
    const STEPS: usize = 100;

    defmt::println!("Delay blend benchmark");

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

    let cycles = op_cyccnt_diff!(cp, {
        for i in 0..STEPS {
            let mut input: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);
            delay.set_attributes(Attributes {
                length: 30.0,
                heads: [
                    HeadAttributes {
                        position: i as f32 / STEPS as f32,
                        feedback: 0.3,
                        volume: 0.8,
                        rewind_backward: None,
                        rewind_forward: None,
                    },
                    HeadAttributes {
                        position: i as f32 / STEPS as f32,
                        feedback: 0.0,
                        volume: 0.0,
                        rewind_backward: None,
                        rewind_forward: None,
                    },
                    HeadAttributes {
                        position: i as f32 / STEPS as f32,
                        feedback: 0.0,
                        volume: 0.0,
                        rewind_backward: None,
                        rewind_forward: None,
                    },
                    HeadAttributes {
                        position: i as f32 / STEPS as f32,
                        feedback: 0.0,
                        volume: 0.0,
                        rewind_backward: None,
                        rewind_forward: None,
                    },
                ],
            });
            delay.process(&mut input);
        }
    });

    defmt::println!("Cycles per buffer: {}", cycles / STEPS as u32);

    kaseta_benches::exit()
}
