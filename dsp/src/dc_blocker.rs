//! DC component blocking filter.
//!
//! Based on <https://ccrma.stanford.edu/~jos/fp/DC_Blocker_Software_Implementations.html>.

const POLE: f32 = 0.995;

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DCBlocker {
    xl_m1: f32,
    xr_m1: f32,
    yl_m1: f32,
    yr_m1: f32,
}

impl DCBlocker {
    pub fn process(&mut self, buffer: &mut [(f32, f32)]) {
        for (xl, xr) in buffer.iter_mut() {
            let yl = *xl - self.xl_m1 + POLE * self.yl_m1;
            self.xl_m1 = *xl;
            self.yl_m1 = yl;
            *xl = yl;

            let yr = *xr - self.xr_m1 + POLE * self.yr_m1;
            self.xr_m1 = *xr;
            self.yr_m1 = yr;
            *xr = yr;
        }
    }
}
