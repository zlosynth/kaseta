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

    use kaseta_control::{InputSnapshot, Store};
    use kaseta_dsp::processor::{
        Attributes as ProcessorAttributes, Processor, Reaction as ProcessorReaction,
    };
    use kaseta_firmware::system::audio::{Audio, SAMPLE_RATE};
    use kaseta_firmware::system::inputs::Inputs;
    use kaseta_firmware::system::outputs::Outputs;
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
        inputs: Inputs,
        outputs: Outputs,
        control: Store,
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
        defmt::info!("INIT");

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
        let audio = system.audio;
        let randomizer = system.randomizer;
        let mut inputs = system.inputs;
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

        let mut control = Store::new();

        for _ in 0..20 {
            inputs.sample();
            control.warm_up(inputs.snapshot());
        }

        blink::spawn(true, BLINKS).unwrap();

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
        let inputs = cx.local.inputs;
        let input_snapshot_producer = cx.local.input_snapshot_producer;

        inputs.sample();
        let _ = input_snapshot_producer.enqueue(inputs.snapshot());
    }

    #[task(local = [control, outputs, input_snapshot_consumer, processor_attributes_producer, processor_reaction_consumer], priority = 3)]
    fn control(cx: control::Context) {
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
            if let Some(_save) = result.save {
                todo!();
            }
            let _ = processor_attributes_producer.enqueue(result.dsp_attributes);
        }

        let desired_output = control.tick();
        outputs.set(&desired_output);
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
