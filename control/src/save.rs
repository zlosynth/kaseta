use crate::cache::mapping::Mapping;
use crate::cache::{Calibrations, Configuration, TappedTempo};

/// TODO: Docs
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Save {
    pub mapping: Mapping,
    pub calibrations: Calibrations,
    pub configuration: Configuration,
    pub tapped_tempo: TappedTempo,
}
