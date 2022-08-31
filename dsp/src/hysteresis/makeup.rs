//! Unify loudness across different hysteresis configurations.
//!
//! This function was selected through fitting optimization performed via
//! `hack/hysteresis.py` and benchmarked through `dsp/benches/makeup.rs`.

#[must_use]
pub fn calculate(drive: f32, saturation: f32, width: f32) -> f32 {
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

    1.0 / (((A1 + A2 * drive + A3 * width * width)
        * (A4 + A5 * saturation + A6 * saturation * saturation))
        / (A7 + A8 * width + A9 * drive * drive)
        + B)
}
