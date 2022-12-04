#![no_main]
#![no_std]

use kaseta_firmware as _; // global logger + panicking-behavior

#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use core::mem::MaybeUninit;

    use daisy::led::{Led, LedUser};
    use fugit::ExtU64;
    use sirena::memory_manager::MemoryManager;
    use systick_monotonic::Systick;

    use kaseta_dsp::processor::Processor;
    use kaseta_firmware::system::audio::{Audio, SAMPLE_RATE};
    use kaseta_firmware::system::randomizer::Randomizer;
    use kaseta_firmware::system::System;

    const BLINKS: u8 = 1;

    #[monotonic(binds = SysTick, default = true)]
    type Mono = Systick<1000>; // 1 kHz / 1 ms granularity

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        status_led: LedUser,
        processor: Processor,
        audio: Audio,
        randomizer: Randomizer,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("INIT");

        let system = System::init(cx.core, cx.device);
        let mono = system.mono;
        let status_led = system.status_led;
        let sdram = system.sdram;
        let audio = system.audio;
        let randomizer = system.randomizer;

        #[allow(clippy::cast_precision_loss)]
        let processor = {
            let mut memory_manager = {
                let ram_slice = unsafe {
                    let ram_items = sdram.size() / core::mem::size_of::<MaybeUninit<u32>>();
                    let ram_ptr = sdram.base_address.cast::<core::mem::MaybeUninit<u32>>();
                    core::slice::from_raw_parts_mut(ram_ptr, ram_items)
                };
                MemoryManager::from(ram_slice)
            };
            Processor::new(SAMPLE_RATE as f32, &mut memory_manager)
        };

        blink::spawn(true, BLINKS).unwrap();

        (
            Shared {},
            Local {
                status_led,
                processor,
                audio,
                randomizer,
            },
            init::Monotonics(mono),
        )
    }

    #[task(binds = DMA1_STR1, local = [processor, audio, randomizer], priority = 4)]
    fn dsp(cx: dsp::Context) {
        let processor = cx.local.processor;
        let audio = cx.local.audio;
        let randomizer = cx.local.randomizer;

        audio.update_buffer(|buffer| {
            processor.process(buffer, randomizer);
        });
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
