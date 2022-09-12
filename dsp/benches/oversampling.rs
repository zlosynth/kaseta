use core::mem::MaybeUninit;
use criterion::{criterion_group, criterion_main, Criterion};

static mut MEMORY: [MaybeUninit<u32>; 512] = unsafe { MaybeUninit::uninit().assume_init() };

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("oversampling", |b| {
        use kaseta_dsp::oversampling::{
            Downsampler4, SignalDownsample, SignalUpsample, Upsampler4,
        };
        use sirena::memory_manager::MemoryManager;
        use sirena::signal::{self, Signal, SignalTake};

        const BUFFER_SIZE: usize = 32;

        const FS: f32 = 48_000.0;
        const FREQ: f32 = 100.0;
        let mut input = signal::sine(FS, FREQ);

        let mut memory_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
        let mut upsampler = Upsampler4::new_4(&mut memory_manager);
        let mut downsampler = Downsampler4::new_4(&mut memory_manager);

        b.iter(|| {
            let _buffer: [f32; BUFFER_SIZE] = input
                .by_ref()
                .upsample(&mut upsampler)
                .downsample(&mut downsampler)
                .take(BUFFER_SIZE)
                .collect::<Vec<_>>()
                .as_slice()
                .try_into()
                .unwrap();
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
