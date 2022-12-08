//! Components of user inteface, passing user input to DSP and reactions back.
//!
//! It is mainly targetted to run in a firmware with multiple loops running in
//! different frequencies, passing messages from one to another. However, parts
//! of it may be useful in software as well.

#![no_std]
#![allow(clippy::items_after_statements)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::redundant_closure_for_method_calls)]

#[cfg(test)]
#[macro_use]
extern crate approx;

mod action;
mod cache;
mod input;
mod output;
mod save;
mod store;

pub use crate::input::snapshot::{Snapshot as InputSnapshot, SnapshotHead as InputSnapshotHead};
pub use crate::output::DesiredOutput;
pub use crate::save::{Save, Store as SaveStore};
pub use crate::store::Store;
