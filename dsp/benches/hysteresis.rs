#![allow(clippy::items_after_statements)]

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("hysteresis", |b| {
        use kaseta_dsp::hysteresis::{Attributes, Hysteresis, SignalApplyHysteresis};
        use sirena::signal::{self, Signal, SignalTake};

        const BUFFER_SIZE: usize = 32;

        const FS: f32 = 48_000.0;
        const FREQ: f32 = 100.0;
        let mut input = signal::sine(FS, FREQ);

        const DRIVE: f32 = 0.5;
        const SATURATION: f32 = 0.5;
        const WIDTH: f32 = 0.5;
        let mut hysteresis = Hysteresis::new(FS);
        hysteresis.set_attributes(Attributes {
            drive: DRIVE,
            saturation: SATURATION,
            width: WIDTH,
        });

        b.iter(|| {
            let _buffer: [f32; BUFFER_SIZE] = input
                .by_ref()
                .apply_hysteresis(&mut hysteresis)
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
