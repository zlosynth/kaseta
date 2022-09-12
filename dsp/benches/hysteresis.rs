#![allow(clippy::items_after_statements)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("hysteresis", |b| {
        use kaseta_dsp::hysteresis::{Attributes, Hysteresis};
        use sirena::signal::{self, SignalTake};

        const BUFFER_SIZE: usize = 32;

        const FS: f32 = 48_000.0;
        const FREQ: f32 = 100.0;
        const DRIVE: f32 = 0.5;
        const SATURATION: f32 = 0.5;
        const WIDTH: f32 = 0.5;
        let mut hysteresis = Hysteresis::new(FS);
        hysteresis.set_attributes(Attributes {
            drive: DRIVE,
            saturation: SATURATION,
            width: WIDTH,
        });

        let mut buffer: [f32; BUFFER_SIZE] = signal::sine(FS, FREQ)
            .take(BUFFER_SIZE)
            .collect::<Vec<_>>()
            .as_slice()
            .try_into()
            .unwrap();

        b.iter(|| {
            hysteresis.process(black_box(&mut buffer));
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
