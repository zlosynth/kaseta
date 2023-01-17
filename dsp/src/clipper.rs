//! Simple hard-clipper.

#[derive(Default, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Clipper;

#[derive(Default, Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Reaction {
    pub clipping: bool,
}

impl Clipper {
    pub fn process(buffer: &mut [f32]) -> Reaction {
        let mut reaction = Reaction::default();

        for x in buffer.iter_mut() {
            if *x < -1.0 {
                *x = -1.0;
                reaction.clipping = true;
            } else if *x > 1.0 {
                *x = 1.0;
                reaction.clipping = true;
            }
        }

        reaction
    }
}
