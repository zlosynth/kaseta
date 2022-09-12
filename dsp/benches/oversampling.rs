#![allow(clippy::similar_names)]

use core::mem::MaybeUninit;
use criterion::{criterion_group, criterion_main, Criterion};

static mut MEMORY: [MaybeUninit<u32>; 512] = unsafe { MaybeUninit::uninit().assume_init() };

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("oversampling", |b| {
        use heapless::Vec;
        use kaseta_dsp::oversampling::{Downsampler4, Upsampler4};
        use sirena::memory_manager::MemoryManager;
        use sirena::signal::{self, SignalTake};

        const BUFFER_SIZE: usize = 32;

        const FS: f32 = 48_000.0;
        const FREQ: f32 = 100.0;
        let mut buffer: [_; BUFFER_SIZE] = signal::sine(FS, FREQ)
            .take(BUFFER_SIZE)
            .collect::<Vec<_, BUFFER_SIZE>>()
            .as_slice()
            .try_into()
            .unwrap();

        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let mut upsampler = Upsampler4::new_4(&mut memory_manager);
        let mut downsampler = Downsampler4::new_4(&mut memory_manager);

        b.iter(|| {
            let mut upsampled = [0.0; BUFFER_SIZE * 4];
            upsampler.process(&buffer, &mut upsampled);
            downsampler.process(&upsampled, &mut buffer);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
