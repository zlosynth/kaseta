#![no_main]
#![no_std]

use kaseta_firmware as _; // global logger + panicking-behavior

#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use systick_monotonic::Systick;

    use kaseta_firmware::system::System;

    #[monotonic(binds = SysTick, default = true)]
    type Mono = Systick<1000>; // 1 kHz / 1 ms granularity

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("INIT");

        let system = System::init(cx.core);
        let mono = system.mono;

        (Shared {}, Local {}, init::Monotonics(mono))
    }
}
