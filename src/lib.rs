//! Type-safe, freeze-on-load environment variable management.
//! Read and parse your environment once at startup into a generated struct.

#[cfg(feature = "dotenv")]
pub mod dotenv;
pub mod error;
pub mod lock;
pub mod parse;
#[cfg(feature = "watch")]
pub mod watcher;

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

/// Define a named, publicly accessible configuration struct with built-in
/// `load`, `try_load`, `from_map`, and `try_from_map` associated functions.
///
/// Unlike the anonymous structs produced by [`load!`], the struct defined here
/// has a real name and can be stored, returned from functions, and used as a
/// type in other signatures.
///
/// # Syntax
///
/// ```rust,no_run
/// lockedenv::env_struct! {
///     pub struct AppConfig {
///         HOST: String,
///         PORT: u16 = 8080,
///         TOKEN: lockedenv::Secret<String>,
///         LABEL: Option<String>,
///     }
/// }
///
/// fn main() {
///     let cfg = AppConfig::load();    // panics on error
///     println!("{}", cfg.HOST);
/// }
/// ```
///
/// A `prefix` can be supplied to strip a common namespace:
///
/// ```rust,no_run
/// lockedenv::env_struct! {
///     pub struct SvcConfig {
///         prefix = "SVC_",
///         HOST: String,
///         PORT: u16,
///     }
/// }
/// // reads SVC_HOST and SVC_PORT from the environment
/// ```
#[macro_export]
macro_rules! env_struct {
    // With prefix
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            prefix = $prefix:literal,
            $($key:ident : $ty:ty $(= $def:expr)?),* $(,)?
        }
    ) => {
        $crate::__env_struct_impl! { $(#[$meta])* $vis $name $prefix [ $($key : $ty $(= $def)?),* ] }
    };
    // Without prefix
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident {
            $($key:ident : $ty:ty $(= $def:expr)?),* $(,)?
        }
    ) => {
        $crate::__env_struct_impl! { $(#[$meta])* $vis $name "" [ $($key : $ty $(= $def)?),* ] }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __env_struct_impl {
    (
        $(#[$meta:meta])* $vis:vis $name:ident $prefix:literal [ $($key:ident : $ty:ty $(= $def:expr)?),* $(,)? ]
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "serde", derive($crate::serde::Serialize, $crate::serde::Deserialize))]
        $vis struct $name {
            $(pub $key: $ty,)*
        }

        impl $name {
            /// Load from OS environment variables; panics on any error.
            pub fn load() -> Self {
                Self::try_load().unwrap_or_else(|e| panic!("{}", e))
            }

            /// Load from OS environment variables; returns `Result`.
            pub fn try_load() -> Result<Self, $crate::EnvLockError> {
                $(
                    #[allow(non_snake_case)]
                    let $key: $ty = $crate::load_internal!(
                        @read_field $ty, concat!($prefix, stringify!($key)) $(, $def)?
                    );
                )*
                #[cfg(feature = "tracing")]
                {
                    let __result = Self { $($key: $key.clone()),* };
                    $crate::tracing::info!(config = ?__result, "lockedenv loaded");
                }
                Ok(Self { $($key),* })
            }

            /// Load from a `HashMap`; panics on any error.
            pub fn from_map(__map: &std::collections::HashMap<String, String>) -> Self {
                Self::try_from_map(__map).unwrap_or_else(|e| panic!("{}", e))
            }

            /// Load from a `HashMap`; returns `Result`.
            pub fn try_from_map(
                __map: &std::collections::HashMap<String, String>,
            ) -> Result<Self, $crate::EnvLockError> {
                $(
                    #[allow(non_snake_case)]
                    let $key: $ty = {
                        let __full_key = concat!($prefix, stringify!($key));
                        match __map.get(__full_key) {
                            Some(__val) => {
                                <$ty as $crate::parse::FromEnvStr>::from_env_str(__val)
                                    .map_err(|e| {
                                        let __found = if <$ty as $crate::parse::FromEnvStr>::REDACT_IN_ERRORS {
                                            "[REDACTED]".into()
                                        } else {
                                            __val.to_string()
                                        };
                                        $crate::EnvLockError::parse_error(
                                            __full_key.into(),
                                            __found,
                                            e,
                                        )
                                    })?
                            }
                            None => $crate::load_internal!(@map_none $ty, __full_key $(, $def)?),
                        }
                    };
                )*
                Ok(Self { $($key),* })
            }
        }
    };
}

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

/// Like [`try_load!`] but collects **all** parse/missing errors instead of stopping
/// at the first one.  Returns `Ok(config)` when every field parsed successfully,
/// or `Err(Vec<EnvLockError>)` listing every problem found.
///
/// Also accepts a `map:` argument for HashMap injection (useful in tests):
///
/// ```rust,no_run
/// match lockedenv::try_check! { HOST: String, PORT: u16, DB: String } {
///     Ok(cfg)  => { /* use cfg */ }
///     Err(errs) => {
///         for e in &errs { eprintln!("{e}"); }
///         std::process::exit(1);
///     }
/// }
/// ```
#[macro_export]
macro_rules! try_check {
    // map — with prefix (must precede env rules; `map:` literal distinguishes them)
    { map: $map:expr, prefix = $prefix:literal, $($rest:tt)* } => {
        $crate::__check_internal!(@map $map, $prefix, $($rest)*)
    };
    // map — no prefix
    { map: $map:expr, $($key:ident : $ty:ty $(= $def:expr)?),* $(,)? } => {
        $crate::__check_internal!(@map $map, "", $($key: $ty $(= $def)?),*)
    };
    // env — with prefix
    { prefix = $prefix:literal, $($rest:tt)* } => {
        $crate::__check_internal!(@env $prefix, $($rest)*)
    };
    // env — no prefix
    { $($key:ident : $ty:ty $(= $def:expr)?),* $(,)? } => {
        $crate::__check_internal!(@env "", $($key: $ty $(= $def)?),*)
    };
}

/// Like [`load!`] but panics with **all** errors listed, not just the first.
/// Accepts the same syntax as [`try_check!`].
#[macro_export]
macro_rules! check {
    // map — with prefix
    { map: $map:expr, prefix = $prefix:literal, $($rest:tt)* } => {
        $crate::try_check! { map: $map, prefix = $prefix, $($rest)* }
            .unwrap_or_else(|__errs| {
                let __msg = __errs.iter()
                    .map(|e| format!("  - {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                panic!("{} configuration error(s):\n{}", __errs.len(), __msg)
            })
    };
    // map — no prefix
    { map: $map:expr, $($rest:tt)* } => {
        $crate::try_check! { map: $map, $($rest)* }
            .unwrap_or_else(|__errs| {
                let __msg = __errs.iter()
                    .map(|e| format!("  - {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                panic!("{} configuration error(s):\n{}", __errs.len(), __msg)
            })
    };
    // env
    { $($rest:tt)* } => {
        $crate::try_check! { $($rest)* }
            .unwrap_or_else(|__errs| {
                let __msg = __errs.iter()
                    .map(|e| format!("  - {}", e))
                    .collect::<Vec<_>>()
                    .join("\n");
                panic!("{} configuration error(s):\n{}", __errs.len(), __msg)
            })
    };
}

// Hidden helper macros for check!/try_check!

#[doc(hidden)]
#[macro_export]
macro_rules! __check_internal {
    // ── env variant ──────────────────────────────────────────────────────────
    (@env $prefix:expr, $($key:ident : $ty:ty $(= $def:expr)?),* $(,)?) => {{
        #[allow(non_snake_case)]
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "serde", derive($crate::serde::Serialize, $crate::serde::Deserialize))]
        struct __CheckConfig { $($key: $ty,)* }

        let mut __errors: Vec<$crate::EnvLockError> = Vec::new();

        $(
            #[allow(non_snake_case)]
            let $key: Result<$ty, $crate::EnvLockError> =
                $crate::__check_read_env!($ty, concat!($prefix, stringify!($key)) $(, $def)?);
            if let Err(ref __e) = $key {
                __errors.push(__e.clone());
            }
        )*

        if __errors.is_empty() {
            // All Ok — unwrap is safe: each $key is Ok when __errors is empty.
            #[allow(clippy::unwrap_used)]
            Ok(__CheckConfig { $($key: $key.unwrap()),* })
        } else {
            Err(__errors)
        }
    }};

    // ── map variant ──────────────────────────────────────────────────────────
    (@map $map:expr, $prefix:expr, $($key:ident : $ty:ty $(= $def:expr)?),* $(,)?) => {{
        #[allow(non_snake_case)]
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "serde", derive($crate::serde::Serialize, $crate::serde::Deserialize))]
        struct __CheckConfig { $($key: $ty,)* }

        let __check_map = &$map;
        let mut __errors: Vec<$crate::EnvLockError> = Vec::new();

        $(
            #[allow(non_snake_case)]
            let $key: Result<$ty, $crate::EnvLockError> = {
                let __full_key = concat!($prefix, stringify!($key));
                match __check_map.get(__full_key) {
                    Some(__val) => {
                        <$ty as $crate::parse::FromEnvStr>::from_env_str(__val)
                            .map_err(|e| {
                                let __found = if <$ty as $crate::parse::FromEnvStr>::REDACT_IN_ERRORS {
                                    "[REDACTED]".into()
                                } else {
                                    __val.to_string()
                                };
                                $crate::EnvLockError::parse_error(__full_key.into(), __found, e)
                            })
                    }
                    None => $crate::__check_map_none!($ty, __full_key $(, $def)?),
                }
            };
            if let Err(ref __e) = $key {
                __errors.push(__e.clone());
            }
        )*

        if __errors.is_empty() {
            #[allow(clippy::unwrap_used)]
            Ok(__CheckConfig { $($key: $key.unwrap()),* })
        } else {
            Err(__errors)
        }
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __check_read_env {
    ($ty:ty, $key:expr, $def:expr) => {
        $crate::lock::__read_default::<$ty>($key, $def)
    };
    ($ty:ty, $key:expr) => {
        $crate::lock::__read_required::<$ty>($key)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __check_map_none {
    ($ty:ty, $key:expr, $def:expr) => {
        Ok::<$ty, $crate::EnvLockError>($def)
    };
    ($ty:ty, $key:expr) => {
        $crate::lock::__missing_value::<$ty>($key)
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

/// Start a background drift-detection watcher for a specific set of keys.
///
/// Returns a [`watcher::WatchHandle`]; drop it (or call `.stop()`) to
/// terminate the thread gracefully.
///
/// Only variables listed in `keys` are monitored.  On each tick the watcher
/// reads exactly those keys via [`std::env::var`] — O(watched vars) instead
/// of O(all env vars).
///
/// `on_drift` receives `(key: &str, old: &str, new: &str)` on each change.
/// `"<removed>"` is passed as `new` when a variable disappears;
/// `"<missing>"` is passed as `old` when a variable is newly added.
///
/// Requires feature `watch`.
///
/// ```rust,no_run
/// let _handle = lockedenv::watch!(
///     keys = ["PORT", "DATABASE_URL"],
///     interval_secs = 60,
///     on_drift = |key, _old, _new| {
///         eprintln!("env drift detected: {}", key);
///     }
/// );
/// // Drop _handle to stop the watcher gracefully.
/// ```
#[cfg(feature = "watch")]
#[macro_export]
macro_rules! watch {
    (keys = [$($key:expr),* $(,)?], interval_secs = $secs:expr, on_drift = $cb:expr) => {
        $crate::watcher::start(
            vec![$($key.to_string()),*],
            std::time::Duration::from_secs($secs),
            $cb,
        )
    };
    (keys = [$($key:expr),* $(,)?], interval_ms = $ms:expr, on_drift = $cb:expr) => {
        $crate::watcher::start(
            vec![$($key.to_string()),*],
            std::time::Duration::from_millis($ms),
            $cb,
        )
    };
    (keys = [$($key:expr),* $(,)?], on_drift = $cb:expr) => {
        $crate::watcher::start(
            vec![$($key.to_string()),*],
            std::time::Duration::from_secs(5),
            $cb,
        )
    };
}
