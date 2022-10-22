//! Oversampling benchmark.
//!
//! Measuring how many DWT cycles it takes for a buffer of 32 random samples to
//! be upsampled by 4 and then downsampled.
//!
//! * Original signal-based implementation: 11936078
//! * Enable icache: 3760126
//! * Enable dcache: 3648398
//! * Move outside workspaces: 188568
//! * Optimized pow2 buffer for upsampling: 142556
//! * Optimized pow2 buffer for downsampling: 92428
//! * Upsampling the whole buffer at once: 71730
//! * Further tweaking of the buffer: 70714
//! * Downsampling the whole buffer at once: 41241
//! * Using SDRAM instead of stack: 44708
//!
//! TODO: Unroll loops
//! TODO: Check more in CMSIS

#![no_main]
#![no_std]
#![allow(clippy::similar_names)]

use kaseta_benches as _;
use kaseta_benches::{op_cyccnt_diff, random_buffer};

use core::mem::MaybeUninit;

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use sirena::memory_manager::MemoryManager;

use kaseta_dsp::oversampling::{Downsampler4, Upsampler4};

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32;

    defmt::println!("Oversampling benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);
    let pins = daisy::board_split_gpios!(board, ccdr, dp);
    let sdram = daisy::board_split_sdram!(cp, dp, ccdr, pins);

    let mut memory_manager = {
        let ram_slice = unsafe {
            let ram_items = sdram.size() / core::mem::size_of::<MaybeUninit<u32>>();
            let ram_ptr = sdram.base_address as *mut MaybeUninit<u32>;
            core::slice::from_raw_parts_mut(ram_ptr, ram_items)
        };
        MemoryManager::from(ram_slice)
    };

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let mut randomizer = dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);
    let mut upsampler = Upsampler4::new_4(&mut memory_manager);
    let mut downsampler = Downsampler4::new_4(&mut memory_manager);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            let input: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);
            let mut output = [0.0; BUFFER_SIZE];

            let mut upsampled = [0.0; BUFFER_SIZE * 4];
            upsampler.process(&input, &mut upsampled);
            downsampler.process(&upsampled, &mut output);
        }
    });

    defmt::println!("Cycles per buffer: {}", cycles / 300);

    kaseta_benches::exit()
}
