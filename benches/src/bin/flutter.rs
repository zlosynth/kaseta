//! Flutter benchmark.
//!
//! Measuring how many DWT cycles it takes for a buffer of 32 random samples
//! to be processed by flutter.
//!
//! * Original implementation: 162429

#![no_main]
#![no_std]
#![allow(clippy::similar_names)]

use core::mem::MaybeUninit;

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use sirena::memory_manager::MemoryManager;

use kaseta_benches as _;
use kaseta_benches::{op_cyccnt_diff, random_buffer};

use kaseta_dsp::random::Random;
use kaseta_dsp::wow_flutter::{Attributes, WowFlutter};

struct RandomStub;

impl Random for RandomStub {
    fn normal(&mut self) -> f32 {
        1.0
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32;

    defmt::println!("Flutter benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let pins = daisy::board_split_gpios!(board, ccdr, dp);
    let sdram = daisy::board_split_sdram!(cp, dp, ccdr, pins);
    let mut randomizer = dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);

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

    let mut wow_flutter = WowFlutter::new(48_000, &mut memory_manager);
    wow_flutter.set_attributes(Attributes {
        wow_depth: 0.0,
        flutter_depth: 1.0,
    });

    let mut buffer: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            wow_flutter.process(&mut buffer, &mut RandomStub);
        }
    });

    defmt::println!("Cycles per buffer: {}", cycles / 300);

    kaseta_benches::exit()
}
