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
//!
//! TODO: Unroll loops
//! TODO: Keep ring buffer on stack
//! TODO: Check more in CMSIS
//! TODO: Apply formatting and checks to this module too

#![no_main]
#![no_std]

use kaseta_benches as _;

use core::mem::MaybeUninit;

use daisy::pac::DWT;
use daisy::hal::prelude::_stm32h7xx_hal_rng_RngCore;
use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use daisy::hal::rng::Rng;

use kaseta_dsp::oversampling::{
    Downsampler4, SignalDownsample, SignalUpsample, Upsampler4,
};
use sirena::signal::{self, Signal};

static mut MEMORY: [MaybeUninit<u32>; 512] = unsafe { MaybeUninit::uninit().assume_init() };

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

    const BUFFER_SIZE: usize = 32;

    let mut cp = cortex_m::Peripherals::take().unwrap();
    let dp = daisy::pac::Peripherals::take().unwrap();
    let board = daisy::Board::take().unwrap();
    let ccdr = daisy::board_freeze_clocks!(board, dp);

    use sirena::memory_manager::MemoryManager;
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

            let mut processed_signal = signal::from_iter(input)
                .upsample(&mut upsampler)
                .downsample(&mut downsampler);

            for x in output.iter_mut() {
                *x = processed_signal.next();
            }
        }
    });

    defmt::println!("Cycles per buffer: {}", cycles / 300);

    kaseta_benches::exit()
}
