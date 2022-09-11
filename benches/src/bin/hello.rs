#![no_main]
#![no_std]

use kaseta_benches as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world!");

    kaseta_benches::exit()
}
