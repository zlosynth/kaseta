//! Logging macro wrappers.
//!
//! This module hides logging implementation behind a local macro.

macro_rules! log_info {
    ( $($arg:tt)+ ) => (
        #[cfg(feature = "defmt")]
        defmt::info!($($arg)+);
    );
}

macro_rules! log_warning {
    ( $($arg:tt)+ ) => (
        #[cfg(feature = "defmt")]
        defmt::warn!($($arg)+);
    );
}

pub(crate) use log_info as info;
pub(crate) use log_warning as warn;
