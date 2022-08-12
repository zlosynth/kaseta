//! Emulation of non-linearities happening on tape.

pub mod makeup;
pub mod signal;
pub mod simulation;

pub use makeup::calculate_makeup;
pub use signal::SignalApplyHysteresis;
pub use simulation::Hysteresis;
