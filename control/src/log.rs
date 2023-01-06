//! Logging macro wrappers.
//!
//! This module hides logging implementation behind a local macro.

macro_rules! info {
    ( $($arg:tt)+ ) => (
        #[cfg(feature = "defmt")]
        defmt::info!($($arg)+);
    );
}

pub(crate) use info;
