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
        self.hysteresis.set_drive(self.drive.next());
        self.hysteresis.set_saturation(self.saturation.next());
        self.hysteresis.set_width(self.width.next());
        self.hysteresis.process(self.source.next())
    }
}
