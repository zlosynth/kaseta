#![no_main]
#![no_std]

use kaseta_bench as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world!");

    kaseta_bench::exit()
}
