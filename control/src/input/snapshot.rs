//! Structures used to pass the current state of hardware peripherals.

/// The current state of all peripherals.
///
/// `InputSnapshot` is meant to be passed from the hardware binding to the
/// control package. It should pass pretty raw data, with two exceptions:
///
/// 1. Detection of plugged control input is done by the caller.
/// 2. Button debouncing is done by the caller.
#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Snapshot {
    pub pre_amp: f32,
    pub drive: f32,
    pub bias: f32,
    pub dry_wet: f32,
    pub wow_flut: f32,
    pub speed: f32,
    pub tone: f32,
    pub head: [SnapshotHead; 4],
    pub control: [Option<f32>; 4],
    pub switch: [bool; 10],
    pub button: bool,
}

#[derive(Debug, Default, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SnapshotHead {
    pub position: f32,
    pub volume: f32,
    pub feedback: f32,
    pub pan: f32,
}
