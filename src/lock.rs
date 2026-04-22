//! Internal helpers for macro-generated code; not part of the public API.

use crate::{error::EnvLockError, parse::FromEnvStr};
use zeroize::Zeroizing;

/// Read a raw environment variable, returning a zeroizing `String` if present.
#[inline]
fn read_raw(key: &str) -> Result<Option<Zeroizing<String>>, EnvLockError> {
    match std::env::var(key) {
        Ok(val) => Ok(Some(Zeroizing::new(val))),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(EnvLockError::parse_error(
            key.to_string(),
            "<non-unicode>".into(),
            "value contains non-UTF-8 bytes",
        )),
    }
}

/// Parse a zeroized string into `T`, zeroizing the original afterwards.
/// Errors are converted into `EnvLockError`; raw values may be redacted.
#[inline]
fn parse_val<T: FromEnvStr>(key: &str, val: &Zeroizing<String>) -> Result<T, EnvLockError> {
    T::from_env_str(val.as_str()).map_err(|e| {
        let found: String = if T::REDACT_IN_ERRORS {
            "[REDACTED]".into()
        } else {
            val.as_str().to_owned()
        };
        EnvLockError::parse_error(key.to_string(), found, e)
    })
}

/// Read a required value; absence is handled via `T::missing_value`.
#[doc(hidden)]
#[inline]
pub fn __read_required<T: FromEnvStr>(key: &str) -> Result<T, EnvLockError> {
    read_raw(key)?.map_or_else(|| T::missing_value(key), |val| parse_val(key, &val))
}

/// Read a variable, parse it, or return the provided default.
#[doc(hidden)]
#[inline]
pub fn __read_default<T: FromEnvStr>(key: &str, default: T) -> Result<T, EnvLockError> {
    read_raw(key)?.map_or_else(|| Ok(default), |val| parse_val(key, &val))
}

/// Helper forwarding to `T::missing_value` for macro use.
#[doc(hidden)]
#[inline]
pub fn __missing_value<T: FromEnvStr>(key: &str) -> Result<T, EnvLockError> {
    T::missing_value(key)
}
