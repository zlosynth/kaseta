use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("oversampling", |b| {
        use kaseta_dsp::hysteresis::{Hysteresis, SignalApplyHysteresis};
        use kaseta_dsp::oversampling::{
            Downsampler8, SignalDownsample, SignalUpsample, Upsampler8,
        };
        use sirena::signal::{self, Signal, SignalTake};

        const BUFFER_SIZE: usize = 32;

        const FS: f32 = 48_000.0;
        const FREQ: f32 = 100.0;
        let mut input = signal::sine(FS, FREQ);

        let mut upsampler = Upsampler8::new_8();
        let mut downsampler = Downsampler8::new_8();

        const DRIVE: f32 = 0.5;
        const SATURATION: f32 = 0.5;
        const WIDTH: f32 = 0.5;
        let mut hysteresis = Hysteresis::new(FS, DRIVE, SATURATION, WIDTH);
        let mut drive = signal::constant(DRIVE);
        let mut saturation = signal::constant(SATURATION);
        let mut width = signal::constant(WIDTH);

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
