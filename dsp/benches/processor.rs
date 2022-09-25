#![allow(clippy::items_after_statements)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("processor", |b| {
        use core::mem::MaybeUninit;
        use kaseta_dsp::processor::{Attributes, Processor};
        use sirena::memory_manager::MemoryManager;

        const BUFFER_SIZE: usize = 32;
        const FS: f32 = 48_000.0;

        static mut MEMORY: [MaybeUninit<u32>; 48000 * 10] =
            unsafe { MaybeUninit::uninit().assume_init() };
        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let mut processor = Processor::new(FS, &mut memory_manager);
        processor.set_attributes(Attributes {
            pre_amp: 2.0,
            drive: 5.0,
            saturation: 0.8,
            width: 0.7,
            wow_frequency: 1.0,
            wow_depth: 0.01,
            delay_length: 1.0,
            delay_head_1_position: 0.0,
            delay_head_2_position: 0.0,
            delay_head_3_position: 0.0,
            delay_head_4_position: 0.0,
            delay_head_1_play: true,
            delay_head_2_play: true,
            delay_head_3_play: true,
            delay_head_4_play: true,
            delay_head_1_feedback: true,
            delay_head_2_feedback: true,
            delay_head_3_feedback: true,
            delay_head_4_feedback: true,
            delay_head_1_feedback_amount: 1.0,
            delay_head_2_feedback_amount: 1.0,
            delay_head_3_feedback_amount: 1.0,
            delay_head_4_feedback_amount: 1.0,
        });

        b.iter(|| {
            // 100 hz sine at 48000 sample rate
            let buffer: [f32; BUFFER_SIZE] = [
                0.0,
                0.013_083_333,
                0.026_166_666,
                0.039_25,
                0.052_333_333,
                0.065_416_664,
                0.078_5,
                0.091_583_334,
                0.104_666_665,
                0.11775,
                0.130_833_33,
                0.143_916_67,
                0.157,
                0.170_083_33,
                0.183_166_67,
                0.19625,
                0.209_333_33,
                0.222_416_67,
                0.2355,
                0.248_583_33,
                0.261_666_66,
                0.27475,
                0.287_833_33,
                0.300_916_67,
                0.314,
                0.327_083_32,
                0.340_166_66,
                0.35325,
                0.366_333_34,
                0.379_416_67,
                0.3925,
                0.405_583_32,
            ];

            processor.process(&mut black_box(buffer));
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
