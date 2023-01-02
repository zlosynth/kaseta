//! Simple hard-clipper.

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Clipper;

impl Clipper {
    // TODO: Notify when hitting the limit
    pub fn process(buffer: &mut [f32]) {
        for x in buffer.iter_mut() {
            *x = x.clamp(-1.0, 1.0);
        }
    }
}
