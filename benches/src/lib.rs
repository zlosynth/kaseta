#![no_main]
#![no_std]

use daisy::hal as _;
use daisy::hal::prelude::_stm32h7xx_hal_rng_RngCore;
use daisy::hal::rng::Rng;
use defmt_rtt as _;
use panic_probe as _;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes `probe-run` exit with exit-code = 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

#[allow(clippy::cast_lossless)]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::missing_panics_doc)]
pub fn random_buffer<const N: usize>(randomizer: &mut Rng) -> [f32; N] {
    let mut buffer = [0.0; N];
    for x in &mut buffer {
        let r: u16 = randomizer.gen().unwrap();
        *x = r as f32 / (2 << 14) as f32 - 1.0;
    }
    buffer
}

#[macro_export]
macro_rules! op_cyccnt_diff {
    ( $cp:expr, $x:expr ) => {
        {
            use core::sync::atomic::{self, Ordering};
            use daisy::pac::DWT;

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
