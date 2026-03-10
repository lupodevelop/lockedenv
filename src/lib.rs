//! Type-safe, freeze-on-load environment variable management.
//! Read and parse your environment once at startup into a generated struct.

pub mod error;
pub mod parse;
pub mod lock;
#[cfg(feature = "watch")]
pub mod watcher;
#[cfg(feature = "dotenv")]
pub mod dotenv;

// Re-export commonly used items
pub use error::EnvLockError;
pub use parse::{FromEnvStr, Secret};

#[doc(hidden)]
#[cfg(feature = "serde")]
pub use serde;
#[doc(hidden)]
#[cfg(feature = "tracing")]
pub use tracing;

// ── public macros ─────────────────────────────────────────────────────────────

/// Like `load!` but returns a `Result<_, EnvLockError>` instead of panicking.
#[macro_export]
macro_rules! try_load {
    { prefix = $prefix:literal, $($rest:tt)* } => {
        $crate::load_internal!(@env $prefix, $($rest)* )
    };
    { $($rest:tt)* } => {
        $crate::load_internal!(@env "", $($rest)* )
    };
}

/// Parse the environment and panic on error; returns the generated struct.
#[macro_export]
macro_rules! load {
    { $($rest:tt)* } => {
        $crate::try_load! { $($rest)* }.unwrap_or_else(|e| panic!("{}", e))
    };
}

/// Like `try_load!` but parses from a provided `HashMap` instead of the OS env.
#[macro_export]
macro_rules! try_from_map {
    (map: $map:expr, prefix = $prefix:literal, $($rest:tt)*) => {
        $crate::load_internal!(@map $map, $prefix, $($rest)*)
    };
    (map: $map:expr, $($rest:tt)*) => {
        $crate::load_internal!(@map $map, "", $($rest)*)
    };
}

/// `try_from_map!` variant that panics on failure.
#[macro_export]
macro_rules! from_map {
    (map: $map:expr, $($rest:tt)*) => {
        $crate::try_from_map!(map: $map, $($rest)*).unwrap_or_else(|e| panic!("{}", e))
    };
}

// Internal implementation macro — not part of the public API.
// Exported only because Rust macros require cross-crate visibility.
#[doc(hidden)]
#[macro_export]
macro_rules! load_internal {
    // env-loading case
    (@env $prefix:expr,
        $($key:ident : $ty:ty $(= $def:expr)?),* $(,)? ) => {
        {
            #[allow(non_snake_case)]
            #[derive(Debug, Clone, PartialEq)]
            #[cfg_attr(feature = "serde", derive($crate::serde::Serialize, $crate::serde::Deserialize))]
            struct __EnvLockConfig {
                $( $key: $ty, )*
            }

            let cfg = (|| -> Result<__EnvLockConfig, $crate::EnvLockError> {
                $(
                    #[allow(non_snake_case)]
                    let $key: $ty = $crate::load_internal!(@read_field $ty, concat!($prefix, stringify!($key)) $(, $def)? );
                )*
                let result = __EnvLockConfig { $( $key ),* };
                $crate::load_internal!(@log result);
                Ok(result)
            })();
            cfg
        }
    };

    // from-map case
    (@map $map:expr, $prefix:expr,
        $($key:ident : $ty:ty $(= $def:expr)?),* $(,)? ) => {
        {
            #[allow(non_snake_case)]
            #[derive(Debug, Clone, PartialEq)]
            #[cfg_attr(feature = "serde", derive($crate::serde::Serialize, $crate::serde::Deserialize))]
            struct __EnvLockConfig {
                $( $key: $ty, )*
            }
            let __env_lock_map = &$map;
            let cfg = (|| -> Result<__EnvLockConfig, $crate::EnvLockError> {
                $(
                    #[allow(non_snake_case)]
                    let $key: $ty = {
                        let full_key = concat!($prefix, stringify!($key));
                        let v_opt = __env_lock_map.get(full_key);
                        match v_opt {
                            Some(val) => {
                                <$ty as $crate::parse::FromEnvStr>::from_env_str(val)
                                    .map_err(|e| {
                                        let found = if <$ty as $crate::parse::FromEnvStr>::REDACT_IN_ERRORS {
                                            "[REDACTED]".into()
                                        } else {
                                            val.to_string()
                                        };
                                        $crate::EnvLockError::parse_error(full_key.into(), found, e)
                                    })?
                            }
                            None => $crate::load_internal!(@map_none $ty, full_key $(, $def)?),
                        }
                    };
                )*
                let result = __EnvLockConfig { $( $key ),* };
                $crate::load_internal!(@log result);
                Ok(result)
            })();
            cfg
        }
    };

    (@log $cfg:ident) => {
        #[cfg(feature = "tracing")]
        $crate::tracing::info!(config = ?$cfg, "lockedenv loaded");
    };

    // None branch without default: use missing_value (Option<T> → Ok(None), others → Err)
    (@map_none $ty:ty, $key:expr) => {
        $crate::lock::__missing_value::<$ty>($key)?
    };
    // None branch with explicit default
    (@map_none $ty:ty, $key:expr, $def:expr) => {
        $def
    };

    // helper to read single field from env, with/without default
    (@read_field $ty:ty, $key:expr, $def:expr) => {
        $crate::lock::__read_default::<$ty>($key, $def)?
    };
    (@read_field $ty:ty, $key:expr) => {
        $crate::lock::__read_required::<$ty>($key)?
    };

}

// --- feature: dotenv ---

/// Load a `.env` file into the process environment, then read variables.
/// Panics if the file exists but cannot be parsed.
/// A missing file is silently ignored.
/// Requires feature `dotenv`.
///
/// ```rust,no_run
/// let config = lockedenv::load_dotenv! {
///     path: ".env",
///     PORT: u16,
///     DATABASE_URL: String,
/// };
/// ```
#[cfg(feature = "dotenv")]
#[macro_export]
macro_rules! load_dotenv {
    (path: $path:expr, $($rest:tt)*) => {
        {
            $crate::dotenv::load_file($path).unwrap_or_else(|e| panic!("{}", e));
            $crate::load! { $($rest)* }
        }
    };
}

/// Load a `.env` file into the process environment, then read variables,
/// returning `Result`.
/// Requires feature `dotenv`.
///
/// ```rust,no_run
/// fn main() -> Result<(), lockedenv::EnvLockError> {
///     let config = lockedenv::try_load_dotenv! { path: ".env.local", PORT: u16 }?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "dotenv")]
#[macro_export]
macro_rules! try_load_dotenv {
    (path: $path:expr, $($rest:tt)*) => {
        {
            $crate::dotenv::load_file($path)?;
            $crate::try_load! { $($rest)* }
        }
    };
}

// --- feature: watch ---

/// Start a background drift-detection watcher.
///
/// Returns a [`watcher::WatchHandle`]; drop it (or call `.stop()`) to
/// terminate the thread gracefully.
///
/// `on_drift` receives `(key: &str, old: &str, new: &str)` on each change.
/// `"<removed>"` is passed as `new` when a variable disappears;
/// `"<missing>"` is passed as `old` when a variable is newly added.
///
/// Requires feature `watch`.
///
/// ```rust,no_run
/// let _handle = lockedenv::watch!(interval_secs = 60, on_drift = |key, _old, _new| {
///     eprintln!("env drift detected: {}", key);
/// });
/// // Drop _handle to stop the watcher gracefully.
/// ```
#[cfg(feature = "watch")]
#[macro_export]
macro_rules! watch {
    (interval_secs = $secs:expr, on_drift = $cb:expr) => {
        $crate::watcher::start(std::time::Duration::from_secs($secs), $cb)
    };
    (interval_ms = $ms:expr, on_drift = $cb:expr) => {
        $crate::watcher::start(std::time::Duration::from_millis($ms), $cb)
    };
    (on_drift = $cb:expr) => {
        $crate::watcher::start(std::time::Duration::from_secs(5), $cb)
    };
}
