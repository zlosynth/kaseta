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

use core::mem::MaybeUninit;

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngCore;
use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use daisy::hal::rng::Rng;
use sirena::memory_manager::MemoryManager;

use kaseta_dsp::oversampling::{Downsampler4, Upsampler4};

static mut MEMORY: [MaybeUninit<u32>; 512] = unsafe { MaybeUninit::uninit().assume_init() };

fn random_buffer(randomizer: &mut Rng) -> [f32; 32] {
    let mut buffer = [0.0; 32];
    for x in &mut buffer {
        let r: u16 = randomizer.gen().unwrap();
        *x = r as f32 / (2 << 14) as f32 - 1.0;
    }
    buffer
}

#[cortex_m_rt::entry]
fn main() -> ! {
    const BUFFER_SIZE: usize = 32;

    defmt::println!("Oversampling benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);

    let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });

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
