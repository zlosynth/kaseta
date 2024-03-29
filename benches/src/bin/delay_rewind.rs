//! Delay rewind benchmark.
//!
//! Measuring how many DWT cycles it takes per buffer to rewind with quarter
//! speed (two octaves lower) over a 10 seconds long random buffer.
//!
//! * Original implementation: 26879
//! * Removing check making sure speed is always above `f32::EPSILON`: 22002
//! * Calling `set_attribute` on every buffer: 25967
//! * Rewinding on all heads: 36082
//! * With introduced impulses (and probably something else too): 33008
//! * After applying wow and flutter on both input and on read: 74741
//! * After applying filter on both the input and feedback: 77349
//! * After replacing max in the compressor with if: 76177
//! * After using a lookup table for dB to linear: 83393
//! * After using a lookup table for linear to dB: 69309

#![no_main]
#![no_std]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]

use kaseta_benches as _;
use kaseta_benches::{op_cyccnt_diff, random_buffer};

use core::mem::MaybeUninit;

use daisy::hal::prelude::_stm32h7xx_hal_rng_RngExt;
use sirena::memory_manager::MemoryManager;

use kaseta_dsp::delay::{Attributes, Delay, FilterPlacement, HeadAttributes, WowFlutterPlacement};
use kaseta_dsp::random::Random;
use kaseta_dsp::tone::Tone2;
use kaseta_dsp::wow_flutter::WowFlutter;

// Slice for shorter buffers that will be stored in the main memory.
#[link_section = ".sram"]
static mut MEMORY: [MaybeUninit<u32>; 96 * 1024] = unsafe { MaybeUninit::uninit().assume_init() };

struct RandomStub;

impl Random for RandomStub {
    fn normal(&mut self) -> f32 {
        1.0
    }
}

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

    let mut sdram_manager = {
        let ram_slice = unsafe {
            let ram_items = sdram.size() / core::mem::size_of::<MaybeUninit<u32>>();
            let ram_ptr = sdram.base_address.cast();
            core::slice::from_raw_parts_mut(ram_ptr, ram_items)
        };
        MemoryManager::from(ram_slice)
    };
    let mut stack_manager = { MemoryManager::from(unsafe { &mut MEMORY[..] }) };

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let mut randomizer = dp.RNG.constrain(ccdr.peripheral.RNG, &ccdr.clocks);
    let mut delay = Delay::new(SAMPLE_RATE, &mut sdram_manager);
    let mut tone = Tone2::new(SAMPLE_RATE);
    let mut wow_flutter = WowFlutter::new(48_000, &mut stack_manager);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..BUFFERS {
            delay.set_attributes(Attributes {
                length: DELAY,
                heads: [
                    HeadAttributes {
                        position: 1.0,
                        feedback: 0.3,
                        volume: 0.8,
                        pan: 0.5,
                        rewind_backward: Some(RELATIVE_SPEED / 2.0),
                        rewind_forward: Some(RELATIVE_SPEED / 2.0),
                    },
                    HeadAttributes {
                        position: 1.0,
                        feedback: 0.3,
                        volume: 0.8,
                        pan: 0.5,
                        rewind_backward: Some(RELATIVE_SPEED / 2.0),
                        rewind_forward: Some(RELATIVE_SPEED / 2.0),
                    },
                    HeadAttributes {
                        position: 1.0,
                        feedback: 0.3,
                        volume: 0.8,
                        pan: 0.5,
                        rewind_backward: Some(RELATIVE_SPEED / 2.0),
                        rewind_forward: Some(RELATIVE_SPEED / 2.0),
                    },
                    HeadAttributes {
                        position: 1.0,
                        feedback: 0.3,
                        volume: 0.8,
                        pan: 0.5,
                        rewind_backward: Some(RELATIVE_SPEED / 2.0),
                        rewind_forward: Some(RELATIVE_SPEED / 2.0),
                    },
                ],
                reset_impulse: false,
                random_impulse: false,
                filter_placement: FilterPlacement::Both,
                wow_flutter_placement: WowFlutterPlacement::Both,
                reset_buffer: false,
                paused: false,
            });
            let mut input: [f32; BUFFER_SIZE] = random_buffer(&mut randomizer);
            let mut output_left: [f32; BUFFER_SIZE] = [0.0; BUFFER_SIZE];
            let mut output_right: [f32; BUFFER_SIZE] = [0.0; BUFFER_SIZE];
            delay.process(
                &mut input,
                &mut output_left,
                &mut output_right,
                &mut tone,
                &mut wow_flutter,
                &mut RandomStub,
            );
        }
    });

    defmt::println!("Cycles per buffer: {}", cycles / BUFFERS as u32);

    kaseta_benches::exit()
}
