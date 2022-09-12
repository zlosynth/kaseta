#![no_main]
#![no_std]

use daisy::hal as _;
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
