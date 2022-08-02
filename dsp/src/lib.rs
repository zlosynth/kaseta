//! Digital signal processing components that must run in real-time.

#[cfg(test)]
#[macro_use]
extern crate approx;

pub mod hysteresis;
pub mod oversampling;
pub mod processor;
pub mod smoothed_value;
