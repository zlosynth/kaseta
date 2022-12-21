macro_rules! info {
    ( $($arg:tt)+ ) => (
        #[cfg(feature = "defmt")]
        defmt::info!($($arg)+);
    );
}

pub(crate) use info;
