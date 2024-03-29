//! DC component blocking filter.
//!
//! Based on <https://ccrma.stanford.edu/~jos/fp/DC_Blocker_Software_Implementations.html>.

// XXX: Assuming 48 kHz sample rate.
const POLE: f32 = 0.995;

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DCBlocker {
    x_m1: f32,
    y_m1: f32,
}

impl DCBlocker {
    pub fn process(&mut self, buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            *x = self.tick(*x);
        }
    }

    pub fn tick(&mut self, x: f32) -> f32 {
        let y = x - self.x_m1 + POLE * self.y_m1;
        self.x_m1 = x;
        self.y_m1 = y;
        y
    }
}
