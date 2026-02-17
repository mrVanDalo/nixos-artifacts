// Shared macros for the artifacts-cli crate

// Export the macro at the crate root so it can be used anywhere as `string_vec![]`
#[macro_export]
macro_rules! string_vec {
    ($($x:expr),* $(,)?) => {
        vec![$($x.to_string()),*]
    };
}

// Feature-gated logging macros
// When "logging" feature is enabled, these delegate to the log crate
// When disabled, they compile to nothing (zero-cost)

#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {};
}

#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        log::error!($($arg)*)
    };
}

#[cfg(not(feature = "logging"))]
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {};
}

// Allow the file to compile even if not directly referenced besides the macro export
#[allow(dead_code)]
const _MACROS_RS: () = ();
