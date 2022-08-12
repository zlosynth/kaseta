//! Apply hysteresis to signal.

use sirena::signal::Signal;

use super::simulation::Hysteresis;

pub trait SignalApplyHysteresis: Signal {
    fn apply_hysteresis<A>(
        self,
        hysteresis: &mut Hysteresis,
        drive: A,
        saturation: A,
        width: A,
    ) -> ApplyHysteresis<Self, A>
    where
        Self: Sized,
    {
        ApplyHysteresis {
            source: self,
            hysteresis,
            drive,
            saturation,
            width,
        }
    }
}

impl<T> SignalApplyHysteresis for T where T: Signal {}

pub struct ApplyHysteresis<'a, S, A> {
    source: S,
    hysteresis: &'a mut Hysteresis,
    drive: A,
    saturation: A,
    width: A,
}

impl<'a, S, A> Signal for ApplyHysteresis<'a, S, A>
where
    S: Signal,
    A: Signal,
{
    fn next(&mut self) -> f32 {
        let drive = self.drive.next();
        let saturation = self.saturation.next();
        let width = self.width.next();

        self.hysteresis.set_drive(drive);
        self.hysteresis.set_saturation(saturation);
        self.hysteresis.set_width(width);

        let makeup = calculate_makeup(drive, saturation, width);

        self.hysteresis.process(self.source.next()) * makeup
    }
}

// TODO: Move this to its own module
// TODO: Apply this in FS, not in oversampled signal
fn calculate_makeup(drive: f32, saturation: f32, width: f32) -> f32 {
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

    1.0 / (((A1 + A2 * drive + A3 * libm::powf(width, 2.0))
        * (A4 + A5 * saturation + A6 * libm::powf(saturation, 2.0)))
        / (A7 + A8 * width + A9 * libm::powf(drive, 2.0))
        + B)
}
