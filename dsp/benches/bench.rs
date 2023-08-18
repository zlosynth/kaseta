use core::mem::MaybeUninit;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::Rng;
use sirena::memory_manager::MemoryManager;

use kaseta_dsp::processor::{Attributes, AttributesHead, Processor};
use kaseta_dsp::random::Random;

struct KasetaRandom;

impl Random for KasetaRandom {
    fn normal(&mut self) -> f32 {
        let mut rng = rand::thread_rng();
        rng.gen()
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    const FS: usize = 48000;
    static mut MEMORY: [MaybeUninit<u32>; FS * 4 * 60 * 6] =
        unsafe { MaybeUninit::uninit().assume_init() };
    let mut stack_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
    let mut sdram_manager = MemoryManager::from(unsafe { &mut MEMORY[..] });
    let mut rng = rand::thread_rng();

    let mut buffer = [(0.0, 0.0); 32];
    #[allow(clippy::cast_precision_loss)]
    let mut processor = Processor::new(FS as f32, &mut stack_manager, &mut sdram_manager);

    c.bench_function("Bench", |b| {
        b.iter(|| {
            processor.set_attributes(Attributes {
                pre_amp: 0.5,
                drive: 0.5,
                saturation: 0.5,
                bias: 0.5,
                dry_wet: 0.5,
                wow: 1.0,
                flutter_depth: 1.0,
                flutter_chance: 1.0,
                speed: 0.5,
                tone: 0.5,
                head: [AttributesHead {
                    position: 0.1,
                    volume: 1.0,
                    feedback: 1.0,
                    pan: 0.4,
                }; 4],
                ..Attributes::default()
            });

            buffer
                .iter_mut()
                .for_each(|(_x, y)| *y = rng.gen::<f32>() * 2.0 - 1.0);
            processor.process(black_box(&mut buffer), &mut KasetaRandom);

            buffer
                .iter_mut()
                .for_each(|(_x, y)| *y = rng.gen::<f32>() * 2.0 - 1.0);
            processor.process(black_box(&mut buffer), &mut KasetaRandom);

            buffer
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
