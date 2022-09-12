//! Emulation of non-linearities happening on tape.

mod makeup;
pub mod processor;
mod simulation;

pub use processor::Attributes;
pub use processor::State as Hysteresis;
