//! Emulation of non-linearities happening on tape.

pub mod signal;
pub mod simulation;

pub use signal::SignalApplyHysteresis;
pub use simulation::Hysteresis;
