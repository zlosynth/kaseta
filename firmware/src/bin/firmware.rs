#![no_main]
#![no_std]

use kaseta_firmware as _; // global logger + panicking-behavior

#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use daisy::led::{Led, LedUser};
    use fugit::ExtU64;
    use systick_monotonic::Systick;

    use kaseta_firmware::system::System;

    const BLINKS: u8 = 1;

    #[monotonic(binds = SysTick, default = true)]
    type Mono = Systick<1000>; // 1 kHz / 1 ms granularity

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        status_led: LedUser,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("INIT");

        let system = System::init(cx.core, cx.device);
        let mono = system.mono;
        let status_led = system.status_led;

        blink::spawn(true, BLINKS).unwrap();

        (Shared {}, Local { status_led }, init::Monotonics(mono))
    }

    #[task(local = [status_led])]
    fn blink(cx: blink::Context, on: bool, blinks: u8) {
        let time_on = 200.millis();
        let time_off_short = 200.millis();
        let time_off_long = 2.secs();

        if on {
            cx.local.status_led.on();
            blink::spawn_after(time_on, false, blinks).unwrap();
        } else {
            cx.local.status_led.off();
            if blinks > 1 {
                blink::spawn_after(time_off_short, true, blinks - 1).unwrap();
            } else {
                blink::spawn_after(time_off_long, true, BLINKS).unwrap();
            }
        }
    }
}
