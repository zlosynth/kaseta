//! Digital signal processing components that must run in real-time.

#![no_std]
#![allow(clippy::items_after_statements)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]

#[cfg(test)]
#[macro_use]
extern crate approx;

pub mod memory_manager;
pub mod processor;

// Public for the sake of benchmarks
pub mod hysteresis;
pub mod oversampling;

mod ring_buffer;
mod smoothed_value;
mod wow_flutter;
