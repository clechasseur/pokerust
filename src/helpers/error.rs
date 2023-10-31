//! Helpers related to generation of error messages.

/// Generates a recursive error message for an [`Error`].
///
/// This will start by using the [`Error`]'s [`Display`] implementation to fetch a description
/// for the error. Then, it will recursively go through any [`source`] errors and get their
/// descriptions too.
///
/// If backtrace support is available (see [`backtrace_message`]), it will also attempt to fetch
/// the error's [`Backtrace`](std::backtrace::Backtrace) and include it in the message.
///
/// [`Error`]: std::error::Error
/// [`source`]: std::error::Error::source
/// [`Display`]: std::fmt::Display
#[cfg(not(tarpaulin_include))]
pub fn recursive_error_message<E>(error: &E) -> String
where
    E: std::error::Error,
{
    let mut message = format!("{}", error);

    let mut current: &dyn std::error::Error = error;
    while let Some(source) = current.source() {
        message += &format!("\ncaused by: {}", source);
        current = source;
    }

    if let Some(backtrace_msg) = backtrace_message(error) {
        message += &format!("\n\nBacktrace: {}", backtrace_msg);
    }

    message
}

/// Attempts to get backtrace information for an [`Error`].
///
/// This function will query the given [`Error`] for a [`Backtrace`] component. If it
/// has one, it's converted to a string through its implementation of [`Display`] and returned.
///
/// # Notes
///
/// This function currently requires a `Nightly` Rust version. It is tied to the `backtrace_support`
/// config, which is set by the build script.
///
/// [`Error`]: std::error::Error
/// [`Backtrace`]: std::backtrace::Backtrace
/// [`Display`]: std::fmt::Display
#[cfg(not(tarpaulin_include))]
#[cfg(backtrace_support)]
pub fn backtrace_message<E>(error: &E) -> Option<String>
where
    E: std::error::Error,
{
    // Note: if you're reading this code and wondering why this is not using `provide_ref`,
    // it's because the API has changed recently in Nightly Rust.
    std::error::request_ref::<std::backtrace::Backtrace>(error)
        .map(|backtrace| format!("{:#}", backtrace))
}

#[cfg(not(tarpaulin_include))]
#[cfg(not(backtrace_support))]
#[doc(hidden)]
pub fn backtrace_message<E>(_error: &E) -> Option<String>
where
    E: std::error::Error,
{
    None
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    use super::*;

    #[derive(Debug, Error)]
    #[error("error C")]
    struct ErrorC;

    #[derive(Debug, Error)]
    #[error("error B")]
    struct ErrorB(#[from] ErrorC);

    #[derive(Debug, Error)]
    #[error("error A")]
    struct ErrorA {
        #[from]
        source: ErrorB,

        #[cfg(backtrace_support)]
        backtrace: std::backtrace::Backtrace,
    }

    fn inner_c() -> Result<(), ErrorC> {
        Err(ErrorC)
    }

    fn inner_b() -> Result<(), ErrorB> {
        Ok(inner_c()?)
    }

    fn inner_a() -> Result<(), ErrorA> {
        Ok(inner_b()?)
    }

    mod recursive_error_message {
        use super::*;

        #[test]
        fn test_all() {
            let error = inner_a().unwrap_err();
            let error_message = recursive_error_message(&error);

            assert!(error_message.starts_with("error A\ncaused by: error B\ncaused by: error C"));

            #[cfg(backtrace_support)]
            assert!(error_message.contains("\n\nBacktrace: "));
            #[cfg(not(backtrace_support))]
            assert!(!error_message.contains("\n\nBacktrace: "));
        }
    }
}
