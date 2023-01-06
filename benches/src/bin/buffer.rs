//! Control buffer benchmark.
//!
//! Measuring how many DWT cycles it takes to perform various
//! operations on a buffer abstraction used by control structures.
//!
//! Write:
//! * Original implementation: 15
//!
//! Read:
//! * Original implementation: 68
//!
//! Read raw:
//! * Original implementation: 28
//! * Using wraparound mask: 22
//!
//! Read previous raw:
//! * Original implementation: 28
//! * Using wraparound mask: 22
//!
//! Reset:
//! * Original implementation: 71
//! * Using `write_volatile` into existing slice: 24
//!
//! Traveled:
//! * Original implementation: 37
//! * Using wraparound mask: 28

#![no_main]
#![no_std]

use core::hint::black_box;

use kaseta_benches as _;
use kaseta_benches::op_cyccnt_diff;

use kaseta_control::Buffer;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Control buffer benchmark");

    let mut cp = cortex_m::Peripherals::take().unwrap();

    cp.SCB.enable_icache();
    cp.SCB.enable_dcache(&mut cp.CPUID);

    let mut buffer: Buffer<16> = Buffer::new();

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            black_box(&mut buffer).write(0.0);
        }
    });
    defmt::println!("Cycles per write: {}", cycles / 300);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            let x = black_box(&mut buffer).read();
            assert_zero(x);
        }
    });
    defmt::println!("Cycles per read: {}", cycles / 300);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            let x = black_box(&mut buffer).read_raw();
            assert_zero(x);
        }
    });
    defmt::println!("Cycles per read raw: {}", cycles / 300);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            let x = black_box(&mut buffer).read_previous_raw();
            assert_zero(x);
        }
    });
    defmt::println!("Cycles per read previous raw: {}", cycles / 300);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            black_box(&mut buffer).reset();
        }
    });
    defmt::println!("Cycles per reset: {}", cycles / 300);

    let cycles = op_cyccnt_diff!(cp, {
        for _ in 0..300 {
            let x = black_box(&mut buffer).traveled();
            assert_zero(x);
        }
    });
    defmt::println!("Cycles per traveled: {}", cycles / 300);

    kaseta_benches::exit()
}

fn assert_zero(x: f32) {
    if x < 0.0 {
        assert!(x > f32::EPSILON);
    } else {
        assert!(x < f32::EPSILON);
    }
}
