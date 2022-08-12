#![allow(clippy::items_after_statements)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let drive = 0.9;
    let saturation = 0.6;
    let width = 0.3;

    let mut group = c.benchmark_group("makeup");
    group.bench_function("func_n_exp", |b| {
        b.iter(|| {
            let _amplitude = func_n_exp(black_box(drive), black_box(saturation), black_box(width));
        });
    });
    group.bench_function("func_d_exp", |b| {
        b.iter(|| {
            let _amplitude = func_d_exp(black_box(drive), black_box(saturation), black_box(width));
        });
    });
    group.bench_function("func_q_pow3", |b| {
        b.iter(|| {
            let _amplitude = func_q_pow3(black_box(drive), black_box(saturation), black_box(width));
        });
    });
    group.bench_function("func_j_pow2", |b| {
        b.iter(|| {
            let _amplitude = func_j_pow2(black_box(drive), black_box(saturation), black_box(width));
        });
    });
    group.finish();
}

fn func_n_exp(d: f32, s: f32, w: f32) -> f32 {
    const A1: f32 = -4.985_852_2;
    const A2: f32 = 2.335_565_8;
    const A3: f32 = -0.465_393_3;
    const A4: f32 = 1.773_630_7;
    const A5: f32 = -1.477_059_4;
    const A6: f32 = 1.049_913_5;
    const A7: f32 = -4.527_344_7;
    const A8: f32 = -1.754_272_9;
    const A9: f32 = 2.294_329;
    const B: f32 = 0.208_471_02;
    ((A1 + A2 * libm::powf(d, A3)) * (A4 + A5 * libm::powf(s, A6))) / (A7 + A8 * libm::powf(w, A9))
        + B
}

fn func_d_exp(d: f32, s: f32, w: f32) -> f32 {
    const A1: f32 = -0.004_693_44;
    const A2: f32 = -0.878_490_1;
    const A3: f32 = -0.388_254_23;
    const A4: f32 = -0.035_151_355;
    const A5: f32 = 0.000_442_267_95;
    const A6: f32 = 0.348_119;
    const A7: f32 = 2_938.31;
    const A8: f32 = 9.306_614e-5;
    const A9: f32 = -0.082_052_104;
    const A10: f32 = 2.160_181_8;
    const A11: f32 = -0.118_728_99;
    const A12: f32 = 9.119_869;
    const B: f32 = -2_937.017_8;

    A1 * d
        + A2 * s
        + A3 * w
        + A4 * d * s
        + A5 * d * w
        + A6 * s * w
        + A7 * libm::powf(d, A8)
        + A9 * libm::powf(s, A10)
        + A11 * libm::powf(w, A12)
        + B
}

fn func_q_pow3(d: f32, s: f32, w: f32) -> f32 {
    const A1: f32 = 0.276_828_14;
    const A2: f32 = -0.414_518_27;
    const A3: f32 = -0.292_250_72;
    const A4: f32 = -0.020_574_56;
    const A5: f32 = -0.154_595_36;
    const A6: f32 = 0.258_215_04;
    const A7: f32 = 0.000_482_339_67;
    const A8: f32 = -0.011_520_979;
    const A9: f32 = -0.424_466_8;
    const A10: f32 = -0.145_059_91;
    const A11: f32 = -0.009_476_376;
    const A12: f32 = 0.061_161_27;
    const A13: f32 = 0.004_975_765;
    const A14: f32 = 0.008_775_611;
    const A15: f32 = 0.000_442_968_74;
    const A16: f32 = 0.000_354_571_3;
    const A17: f32 = -0.015_112_773_5;
    const A18: f32 = 0.289_532_63;
    const A19: f32 = 0.001_244_070_6;
    const B: f32 = 0.780_788_84;

    A1 * d
        + A2 * s
        + A3 * w
        + A4 * libm::powf(d, 2.0)
        + A5 * libm::powf(s, 2.0)
        + A6 * libm::powf(w, 2.0)
        + A7 * libm::powf(d, 3.0)
        + A8 * libm::powf(s, 3.0)
        + A9 * libm::powf(w, 3.0)
        + A10 * d * s
        + A11 * d * w
        + A12 * s * w
        + A13 * libm::powf(d, 2.0) * s
        + A14 * d * libm::powf(s, 2.0)
        + A15 * libm::powf(d, 2.0) * w
        + A16 * d * libm::powf(w, 2.0)
        + A17 * libm::powf(s, 2.0) * w
        + A18 * s * libm::powf(w, 2.0)
        + A19 * d * s * w
        + B
}

fn func_j_pow2(d: f32, s: f32, w: f32) -> f32 {
    const A1: f32 = 1.367_927_7;
    const A2: f32 = 0.912_466_17;
    const A3: f32 = -1.437_861_1;
    const A4: f32 = 1.124_105_8;
    const A5: f32 = -0.985_749_2;
    const A6: f32 = -0.066_880_5;
    const A7: f32 = 3.673_698_2;
    const A8: f32 = 1.490_835_9;
    const A9: f32 = 0.032_865_584;
    const B: f32 = 0.365_093_5;

    ((A1 + A2 * d + A3 * libm::powf(w, 2.0)) * (A4 + A5 * s + A6 * libm::powf(s, 2.0)))
        / (A7 + A8 * w + A9 * libm::powf(d, 2.0))
        + B
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
