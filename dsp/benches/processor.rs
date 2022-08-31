#![allow(clippy::items_after_statements)]

use criterion::{criterion_group, criterion_main, Criterion, black_box};

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
        });

        b.iter(|| {
            // 100 hz sine at 48000 sample rate
            let buffer: [f32; BUFFER_SIZE] = [
                0.0,
                0.013083333333333334,
                0.026166666666666668,
                0.03925000000000001,
                0.052333333333333336,
                0.06541666666666666,
                0.07850000000000001,
                0.09158333333333334,
                0.10466666666666667,
                0.11775,
                0.13083333333333333,
                0.14391666666666666,
                0.15700000000000003,
                0.17008333333333334,
                0.18316666666666667,
                0.19625,
                0.20933333333333334,
                0.22241666666666668,
                0.2355,
                0.24858333333333332,
                0.26166666666666666,
                0.27475,
                0.28783333333333333,
                0.3009166666666667,
                0.31400000000000006,
                0.32708333333333334,
                0.3401666666666667,
                0.35325,
                0.36633333333333334,
                0.3794166666666667,
                0.3925,
                0.4055833333333334,
            ];

            processor.process(&mut black_box(buffer));
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
