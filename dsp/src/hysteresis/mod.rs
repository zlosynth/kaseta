//! Emulation of non-linearities happening on tape.

pub mod makeup;
pub mod signal;

// Public for the sake of benchmarks
pub mod simulation;

pub use makeup::calculate_makeup;
pub use signal::SignalApplyHysteresis;
