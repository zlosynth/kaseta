#![no_main]
#![no_std]
#![allow(clippy::no_mangle_with_rust_abi)] // rtic::app fails this.

use kaseta_firmware as _; // Global logger and panicking behavior.

#[rtic::app(device = stm32h7xx_hal::pac, peripherals = true, dispatchers = [EXTI0, EXTI1, EXTI2])]
mod app {
    use core::mem::MaybeUninit;

    use daisy::hal::time::Hertz;
    use daisy::led::{Led, LedUser};
    use daisy::sdram::SDRAM;
    use fugit::ExtU64;
    use heapless::spsc::{Consumer, Producer, Queue};
    use sirena::memory_manager::MemoryManager;
    use systick_monotonic::Systick;

    use kaseta_control::{DesiredOutput, InputSnapshot, Save, Store};
    use kaseta_dsp::processor::{
        Attributes as ProcessorAttributes, Processor, Reaction as ProcessorReaction,
    };
    use kaseta_firmware::system::audio::{Audio, SAMPLE_RATE};
    use kaseta_firmware::system::inputs::Inputs;
    use kaseta_firmware::system::outputs::Outputs;
    use kaseta_firmware::system::randomizer::Randomizer;
    use kaseta_firmware::system::storage::Storage;
    use kaseta_firmware::system::System;

    // Single blinks on the PCB's LED signalize the first revision.
    const BLINKS: u8 = 1;

    // Slice for shorter buffers that will be stored in the main memory.
    #[link_section = ".sram"]
    static mut MEMORY: [MaybeUninit<u32>; 96 * 1024] =
        unsafe { MaybeUninit::uninit().assume_init() };

    // 1 kHz / 1 ms granularity for task scheduling.
    #[monotonic(binds = SysTick, default = true)]
    type Mono = Systick<1000>;

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
        storage: Storage,
        input_snapshot_producer: Producer<'static, InputSnapshot, 8>,
        input_snapshot_consumer: Consumer<'static, InputSnapshot, 8>,
        processor_attributes_producer: Producer<'static, ProcessorAttributes, 8>,
        processor_attributes_consumer: Consumer<'static, ProcessorAttributes, 8>,
        processor_reaction_producer: Producer<'static, ProcessorReaction, 8>,
        processor_reaction_consumer: Consumer<'static, ProcessorReaction, 8>,
    }

    #[init(
        local = [
            input_snapshot_queue: Queue<InputSnapshot, 8> = Queue::new(),
            processor_attributes_queue: Queue<ProcessorAttributes, 8> = Queue::new(),
            processor_reaction_queue: Queue<ProcessorReaction, 8> = Queue::new(),
        ]
    )]
    fn init(cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("Starting the firmware, initializing resources");

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
        let flash = system.flash;
        let mut outputs = system.outputs;

        let processor = initialize_dsp_processor(sdram);
        let mut storage = Storage::new(flash);
        let (control, save) = initialize_control_store(&mut inputs, &mut storage, system.frequency);

        defmt::info!("Initialization was completed, starting tasks");

        // Buy some time to avoid clicks on boot.
        boot_animation(&mut outputs);

        audio.spawn();
        blink::spawn(true, BLINKS).unwrap();
        control::spawn().unwrap();
        input::spawn().unwrap();
        // Force-save initial configuration. This is required in case reset was initiated.
        store::spawn(save).ok().unwrap();

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
                storage,
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

    fn boot_animation(outputs: &mut Outputs) {
        const MS: u32 = 480_000_000 / 1000;
        const STEP: u32 = 130 * MS;

        let mut set_leds = |leds| {
            outputs.set(&DesiredOutput {
                display: leds,
                impulse_led: true,
                impulse_trigger: false,
            });
        };

        for x in [
            [true, false, false, false, false, false, false, true],
            [false, true, false, false, false, false, true, false],
            [false, false, true, false, false, true, false, false],
            [false, false, false, true, true, false, false, false],
            [false, false, true, false, false, true, false, false],
            [false, true, false, false, false, false, true, false],
            [true, false, false, false, false, false, false, true],
        ] {
            set_leds(x);
            cortex_m::asm::delay(STEP);
        }
    }

    #[task(
        binds = DMA1_STR1,
        local = [
            processor,
            audio,
            randomizer,
            processor_attributes_consumer,
            processor_reaction_producer,
        ],
        priority = 4,
    )]
    fn dsp(cx: dsp::Context) {
        let processor = cx.local.processor;
        let audio = cx.local.audio;
        let randomizer = cx.local.randomizer;
        let processor_attributes_consumer = cx.local.processor_attributes_consumer;
        let processor_reaction_producer = cx.local.processor_reaction_producer;

        warn_about_queue_capacity("processor_attributes", processor_attributes_consumer);

        if let Some(attributes) = dequeue_last(processor_attributes_consumer) {
            processor.set_attributes(attributes);
        }

        let mut reaction = None;
        audio.update_buffer(|buffer| {
            reaction = Some(processor.process(buffer, randomizer));
        });

        processor_reaction_producer
            .enqueue(reaction.unwrap())
            .ok()
            .unwrap();
    }

    #[task(
        local = [
            inputs,
            input_snapshot_producer,
        ],
        priority = 2,
    )]
    fn input(cx: input::Context) {
        input::spawn_after(1.millis()).ok().unwrap();

        let inputs = cx.local.inputs;
        let input_snapshot_producer = cx.local.input_snapshot_producer;

        inputs.sample();

        input_snapshot_producer
            .enqueue(inputs.snapshot())
            .ok()
            .unwrap();
    }

    #[task(
        local = [
            control,
            outputs,
            input_snapshot_consumer,
            processor_attributes_producer,
            processor_reaction_consumer,
        ],
        priority = 3,
    )]
    fn control(cx: control::Context) {
        control::spawn_after(1.millis()).ok().unwrap();

        let control = cx.local.control;
        let outputs = cx.local.outputs;
        let input_snapshot_consumer = cx.local.input_snapshot_consumer;
        let processor_attributes_producer = cx.local.processor_attributes_producer;
        let processor_reaction_consumer = cx.local.processor_reaction_consumer;

        warn_about_queue_capacity("input_snapshot", input_snapshot_consumer);
        warn_about_queue_capacity("processor_reaction", processor_reaction_consumer);

        while let Some(reaction) = processor_reaction_consumer.dequeue() {
            control.apply_dsp_reaction(reaction);
        }

        if let Some(snapshot) = dequeue_last(input_snapshot_consumer) {
            let result = control.apply_input_snapshot(snapshot);
            if let Some(save) = result.save {
                store::spawn(save).ok().unwrap();
            }
            processor_attributes_producer
                .enqueue(result.dsp_attributes)
                .ok()
                .unwrap();
        }

        let desired_output = control.tick();
        outputs.set(&desired_output);
    }

    #[task(local = [storage])]
    fn store(cx: store::Context, save: Save) {
        let storage = cx.local.storage;
        storage.save_save(save);
    }

    #[task(local = [status_led])]
    fn blink(cx: blink::Context, on: bool, mut blinks_left: u8) {
        let time_on = 200.millis();
        let time_off_short = 200.millis();
        let time_off_long = 2.secs();

        if on {
            cx.local.status_led.on();
            blink::spawn_after(time_on, false, blinks_left).unwrap();
        } else {
            cx.local.status_led.off();
            blinks_left -= 1;
            if blinks_left > 0 {
                blink::spawn_after(time_off_short, true, blinks_left).unwrap();
            } else {
                blink::spawn_after(time_off_long, true, BLINKS).unwrap();
            }
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn initialize_dsp_processor(sdram: SDRAM) -> Processor {
        let mut sdram_manager = initialize_sdram_manager(sdram);
        let mut stack_manager = initialize_stack_manager();
        Processor::new(SAMPLE_RATE as f32, &mut stack_manager, &mut sdram_manager)
    }

    fn initialize_sdram_manager(sdram: SDRAM) -> MemoryManager {
        let ram_slice = convert_sdram_into_u32_slice(sdram);
        MemoryManager::from(ram_slice)
    }

    #[allow(clippy::needless_pass_by_value)] // This function takes over the ownership of the memory
    fn convert_sdram_into_u32_slice(sdram: SDRAM) -> &'static mut [MaybeUninit<u32>] {
        unsafe {
            let ram_items = sdram.size() / core::mem::size_of::<MaybeUninit<u32>>();
            let ram_ptr = sdram.base_address.cast::<core::mem::MaybeUninit<u32>>();
            core::slice::from_raw_parts_mut(ram_ptr, ram_items)
        }
    }

    fn initialize_stack_manager() -> MemoryManager {
        MemoryManager::from(unsafe { &mut MEMORY[..] })
    }

    fn initialize_control_store(
        inputs: &mut Inputs,
        storage: &mut Storage,
        frequency: Hertz,
    ) -> (Store, Save) {
        let save = retrieve_save(inputs, storage);
        let mut control = Store::from(save);
        warm_up_control(&mut control, inputs, frequency);
        (control, save)
    }

    fn retrieve_save(inputs: &mut Inputs, storage: &mut Storage) -> Save {
        // XXX: This must be called even if not used, so the storage gets
        // initialized with the latest used version.
        let latest_save = storage.load_save();
        if is_button_held(inputs) {
            defmt::info!("Reset was initiated");
            wait_until_button_is_released(inputs);
            Save::default()
        } else {
            latest_save
        }
    }

    fn is_button_held(inputs: &mut Inputs) -> bool {
        inputs.button.active_no_filter()
    }

    fn wait_until_button_is_released(inputs: &mut Inputs) {
        while inputs.button.active_no_filter() {}
    }

    fn warm_up_control(control: &mut Store, inputs: &mut Inputs, frequency: Hertz) {
        let ms = frequency.to_kHz();
        for _ in 0..50 {
            inputs.sample();
            control.warm_up(inputs.snapshot());
            cortex_m::asm::delay(5 * ms);
        }
    }

    fn dequeue_last<T, const N: usize>(consumer: &mut Consumer<'static, T, N>) -> Option<T> {
        let mut last_item = None;
        while let Some(attributes) = consumer.dequeue() {
            last_item = Some(attributes);
        }
        last_item
    }

    fn warn_about_queue_capacity<T, const N: usize>(
        name: &str,
        consumer: &mut Consumer<'static, T, N>,
    ) {
        if consumer.len() > consumer.capacity() / 2 {
            defmt::warn!(
                "Queue={:?} is above the half of its capacity {:?}/{:?}",
                name,
                consumer.len(),
                consumer.capacity()
            );
        }
    }
}
