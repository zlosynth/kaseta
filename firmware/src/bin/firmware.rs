#![no_main]
#![no_std]

use kaseta_firmware as _; // global logger + panicking-behavior

#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use core::mem::MaybeUninit;

    use daisy::led::{Led, LedUser};
    use fugit::ExtU64;
    use heapless::spsc::{Consumer, Producer, Queue};
    use sirena::memory_manager::MemoryManager;
    use systick_monotonic::Systick;

    use kaseta_control::{InputSnapshot, Save, Store};
    use kaseta_dsp::processor::{
        Attributes as ProcessorAttributes, Processor, Reaction as ProcessorReaction,
    };
    use kaseta_firmware::system::audio::{Audio, SAMPLE_RATE};
    use kaseta_firmware::system::inputs::Inputs;
    use kaseta_firmware::system::outputs::Outputs;
    use kaseta_firmware::system::randomizer::Randomizer;
    // use kaseta_firmware::system::storage::Storage;
    use kaseta_firmware::system::System;

    const BLINKS: u8 = 1;
    const MS: u32 = 480_000_000 / 1000;

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
        inputs: Inputs,
        outputs: Outputs,
        control: Store,
        // storage: Storage,
        input_snapshot_producer: Producer<'static, InputSnapshot, 6>,
        input_snapshot_consumer: Consumer<'static, InputSnapshot, 6>,
        processor_attributes_producer: Producer<'static, ProcessorAttributes, 6>,
        processor_attributes_consumer: Consumer<'static, ProcessorAttributes, 6>,
        processor_reaction_producer: Producer<'static, ProcessorReaction, 6>,
        processor_reaction_consumer: Consumer<'static, ProcessorReaction, 6>,
    }

    #[init(
        local = [
            input_snapshot_queue: Queue<InputSnapshot, 6> = Queue::new(),
            processor_attributes_queue: Queue<ProcessorAttributes, 6> = Queue::new(),
            processor_reaction_queue: Queue<ProcessorReaction, 6> = Queue::new(),
        ]
    )]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("START");

        let (input_snapshot_producer, input_snapshot_consumer) =
            cx.local.input_snapshot_queue.split();
        let (processor_attributes_producer, processor_attributes_consumer) =
            cx.local.processor_attributes_queue.split();
        let (processor_reaction_producer, processor_reaction_consumer) =
            cx.local.processor_reaction_queue.split();

        let system = System::init(cx.core, cx.device);
        let mono = system.mono;
        let status_led = system.status_led;
        let sdram = system.sdram;
        let mut audio = system.audio;
        let randomizer = system.randomizer;
        let mut inputs = system.inputs;
        // let flash = system.flash;
        let outputs = system.outputs;

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

        // let mut storage = Storage::new(flash);

        let control = {
            let save = if inputs.button.active_no_filter() {
                defmt::info!("RESET");
                let save = Save::default();
                while inputs.button.active_no_filter() {}
                save
            } else {
                Save::default()
                // storage.load_save()
            };

            defmt::info!("INITIALIZE WITH: {:?}", save);
            let mut control = Store::from(save);

            for _ in 0..50 {
                inputs.sample();
                control.warm_up(inputs.snapshot());
                cortex_m::asm::delay(5 * MS);
            }

            control
        };

        audio.spawn();
        blink::spawn(true, BLINKS).unwrap();
        control::spawn().unwrap();
        input::spawn().unwrap();

        (
            Shared {},
            Local {
                status_led,
                processor,
                audio,
                randomizer,
                inputs,
                outputs,
                control,
                // storage,
                input_snapshot_producer,
                input_snapshot_consumer,
                processor_attributes_producer,
                processor_attributes_consumer,
                processor_reaction_producer,
                processor_reaction_consumer,
            },
            init::Monotonics(mono),
        )
    }

    #[task(binds = DMA1_STR1, local = [processor, audio, randomizer, processor_attributes_consumer, processor_reaction_producer], priority = 4)]
    fn dsp(cx: dsp::Context) {
        let processor = cx.local.processor;
        let audio = cx.local.audio;
        let randomizer = cx.local.randomizer;
        let processor_attributes_consumer = cx.local.processor_attributes_consumer;
        let processor_reaction_producer = cx.local.processor_reaction_producer;

        let mut last_attributes = None;
        while let Some(attributes) = processor_attributes_consumer.dequeue() {
            last_attributes = Some(attributes);
        }
        if let Some(attributes) = last_attributes {
            processor.set_attributes(attributes);
        }

        let mut reaction = None;
        audio.update_buffer(|buffer| {
            reaction = Some(processor.process(buffer, randomizer));
        });

        // TODO: In production code, this should not fail - let _ =
        processor_reaction_producer
            .enqueue(reaction.unwrap())
            .ok()
            .unwrap();
    }

    #[task(local = [inputs, input_snapshot_producer], priority = 2)]
    fn input(cx: input::Context) {
        // TODO: Reconcile CV every ms, pots every 4
        input::spawn_after(4.millis()).ok().unwrap();

        let inputs = cx.local.inputs;
        let input_snapshot_producer = cx.local.input_snapshot_producer;

        inputs.sample();
        // TODO: In production code, this should not fail - let _ =, or even use unchecked enqueue
        input_snapshot_producer
            .enqueue(inputs.snapshot())
            .ok()
            .unwrap();
    }

    #[task(local = [control, outputs, input_snapshot_consumer, processor_attributes_producer, processor_reaction_consumer], priority = 3)]
    fn control(cx: control::Context) {
        // TODO: Make sure this is using accurate clock: https://rtic.rs/1/book/en/by-example/monotonic.html
        control::spawn_after(1.millis()).ok().unwrap();

        let control = cx.local.control;
        let outputs = cx.local.outputs;
        let input_snapshot_consumer = cx.local.input_snapshot_consumer;
        let processor_attributes_producer = cx.local.processor_attributes_producer;
        let processor_reaction_consumer = cx.local.processor_reaction_consumer;

        while let Some(reaction) = processor_reaction_consumer.dequeue() {
            control.apply_dsp_reaction(reaction);
        }

        let mut last_snapshot = None;
        while let Some(snapshot) = input_snapshot_consumer.dequeue() {
            last_snapshot = Some(snapshot);
        }
        if let Some(snapshot) = last_snapshot {
            let result = control.apply_input_snapshot(snapshot);
            if let Some(save) = result.save {
                // TODO: In production code, this should not fail - let _ =
                // store::spawn(save).ok().unwrap();
            }
            // TODO: In production code, this should not fail - let _ =, or even use unchecked enqueue
            processor_attributes_producer
                .enqueue(result.dsp_attributes)
                .ok()
                .unwrap();
        }

        let desired_output = control.tick();
        outputs.set(&desired_output);
    }

    // TODO: Currently saves corrupt flash and prevent
    // starts using debugger (reset works ok). Hence disabling
    // this for now. Corrupt Daisy can be fixed by flushing first
    // blinky and then QSPI examples from
    // https://electro-smith.github.io/Programmer/.
    // #[task(local = [storage])]
    // fn store(cx: store::Context, save: Save) {
    //     let storage = cx.local.storage;
    //     // storage.save_save(save);
    // }

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
