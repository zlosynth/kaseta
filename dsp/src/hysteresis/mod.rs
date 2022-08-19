//! Emulation of non-linearities happening on tape.

pub mod makeup;
pub mod processor;
mod simulation;

pub use makeup::calculate_makeup;
pub use processor::Attributes;
pub use processor::SignalApplyHysteresis;
pub use processor::State as Hysteresis;
