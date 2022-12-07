//! Digital signal processing components that must run in real-time.

#![no_std]
#![allow(clippy::items_after_statements)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::similar_names)]

#[cfg(test)]
#[macro_use]
extern crate approx;

pub mod processor;
pub mod random;

// Public for the sake of benchmarks
pub mod delay;
pub mod hysteresis;
pub mod oversampling;

mod math;
mod oscillator;
mod pre_amp;
mod ring_buffer;
mod state_variable_filter;
mod tone;
mod wow_flutter;
