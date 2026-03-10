use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;
use zeroize::Zeroize;

/// Trait for converting an environment string into a typed value.
///
/// Implement this for custom types to use them with the `lockedenv` macros.
/// The default `missing_value` treats an absent key as an error; `Option<T>`
/// overrides that behavior.  Set `REDACT_IN_ERRORS = true` to redact raw
/// values from error messages (useful for secrets).
#[allow(clippy::missing_errors_doc)]
pub trait FromEnvStr: Sized {
    /// The error type returned when parsing fails.
    type Err: std::fmt::Display;

    /// Parse the raw string `s` into `Self`.
    fn from_env_str(s: &str) -> Result<Self, Self::Err>;

    /// Called when the corresponding key is absent.
    /// Defaults to an error; `Option<T>` returns `Ok(None)`.
    fn missing_value(key: &str) -> Result<Self, crate::error::EnvLockError> {
        Err(crate::error::EnvLockError::missing(key.to_string()))
    }

    /// When `true`, error messages replace the raw value with `[REDACTED]`.
    ///
    /// Override this in security-sensitive wrappers (e.g. [`Secret`]) so
    /// that raw secrets never leak into logs, tracing output or panic messages.
    const REDACT_IN_ERRORS: bool = false;
}

// impls for standard types

impl FromEnvStr for String {
    type Err = std::convert::Infallible;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.to_owned())
    }
}

macro_rules! impl_fromstr {
    ($ty:ty) => {
        impl FromEnvStr for $ty {
            type Err = <$ty as std::str::FromStr>::Err;
            fn from_env_str(s: &str) -> Result<Self, Self::Err> {
                s.parse()
            }
        }
    };
}

impl_fromstr!(u8);
impl_fromstr!(u16);
impl_fromstr!(u32);
impl_fromstr!(u64);
impl_fromstr!(u128);
impl_fromstr!(usize);
impl_fromstr!(i8);
impl_fromstr!(i16);
impl_fromstr!(i32);
impl_fromstr!(i64);
impl_fromstr!(i128);
impl_fromstr!(isize);
impl_fromstr!(f32);
impl_fromstr!(f64);

impl FromEnvStr for bool {
    type Err = String;
    /// Accepts `true`, `1`, `yes`, `false`, `0`, `no` (case-insensitive, zero-alloc).
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("true") || s == "1" || s.eq_ignore_ascii_case("yes") {
            Ok(true)
        } else if s.eq_ignore_ascii_case("false") || s == "0" || s.eq_ignore_ascii_case("no") {
            Ok(false)
        } else {
            Err(format!("invalid bool: {s}"))
        }
    }
}

impl FromEnvStr for char {
    type Err = String;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        let mut chars = s.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            Ok(c)
        } else {
            Err("expected single character".into())
        }
    }
}

impl FromEnvStr for PathBuf {
    type Err = std::convert::Infallible;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        Ok(PathBuf::from(s))
    }
}

impl FromEnvStr for IpAddr {
    type Err = std::net::AddrParseError;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
    }
}
impl FromEnvStr for Ipv4Addr {
    type Err = std::net::AddrParseError;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
    }
}
impl FromEnvStr for Ipv6Addr {
    type Err = std::net::AddrParseError;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
    }
}
impl FromEnvStr for SocketAddr {
    type Err = std::net::AddrParseError;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
    }
}

impl<T: FromEnvStr> FromEnvStr for Vec<T> {
    type Err = String;

    /// Propagated from `T` so that `Vec<Secret<String>>` still redacts.
    const REDACT_IN_ERRORS: bool = T::REDACT_IN_ERRORS;

    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        if s.trim().is_empty() {
            return Ok(Vec::new());
        }
        s.split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .enumerate()
            .map(|(i, part)| {
                T::from_env_str(part).map_err(|e| {
                    if T::REDACT_IN_ERRORS {
                        format!("item[{i}]: {e}")
                    } else {
                        format!("item[{i}] {part:?}: {e}")
                    }
                })
            })
            .collect()
    }
}

/// A wrapper for sensitive environment variables that:
///
/// - **Redacts** the value in [`Debug`] and (when `serde` is enabled) [`Serialize`] output.
/// - **Zeroes** the inner heap memory on [`Drop`] via [`Zeroize`].
/// - **Prevents** raw values from leaking into error messages
///   ([`REDACT_IN_ERRORS`](FromEnvStr::REDACT_IN_ERRORS) = `true`).
///
/// # Limitations
///
/// - [`std::mem::forget`] bypasses the `Drop` impl and **will not** zeroize
///   the inner value.  This is a fundamental limitation of the `Zeroize`
///   pattern and cannot be solved without a custom allocator.
/// - [`Clone`] creates an independent copy of the secret on the heap.
///   Both copies are zeroized on drop, but the attack surface is doubled.
/// - [`PartialEq`] uses the standard short-circuit comparison of `T`, which
///   is **not constant-time**.  Do not use it in contexts where a timing
///   side-channel could leak information about the secret.
/// - [`Deref`](std::ops::Deref) exposes `&T`, which may implement
///   [`Display`](std::fmt::Display).  `Secret` itself intentionally does
///   **not** implement `Display` to prevent accidental logging.
///
/// # Example
///
/// ```rust
/// let m: std::collections::HashMap<String, String> =
///     [("TOKEN".into(), "secret".into())].into_iter().collect();
/// let cfg = lockedenv::from_map! { map: m, TOKEN: lockedenv::Secret<String> };
/// assert_eq!(cfg.TOKEN.as_ref(), "secret");
/// // Debug never leaks the value:
/// assert!(format!("{:?}", cfg).contains("[REDACTED]"));
/// ```
#[derive(Clone)]
pub struct Secret<T: Zeroize>(T);

impl<T: Zeroize> Secret<T> {
    /// Create a new `Secret` wrapping the given value.
    pub fn new(val: T) -> Self {
        Self(val)
    }

    /// Consume the wrapper and return the inner value.
    ///
    /// Uses [`std::mem::ManuallyDrop`] to bypass the `Drop` impl (which
    /// zeroizes `T`).  The caller takes **full responsibility** for the
    /// returned value and its eventual cleanup.
    #[must_use = "the inner value will not be zeroized if dropped unused"]
    pub fn into_inner(self) -> T {
        let s = std::mem::ManuallyDrop::new(self);
        // SAFETY: `s` is in ManuallyDrop — `Secret<T>`'s Drop will NOT run.
        // We bitwise-copy the inner `T` out.  The ManuallyDrop keeps the
        // original bytes alive but un-dropped; the caller now owns the T.
        unsafe { std::ptr::read(std::ptr::addr_of!(s.0)) }
    }
}

impl<T: Zeroize> std::fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Secret([REDACTED])")
    }
}

/// **Warning:** uses the standard short-circuit comparison of `T` — not
/// constant-time.  Avoid in timing-sensitive contexts.
impl<T: Zeroize + PartialEq> PartialEq for Secret<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Zeroize + Eq> Eq for Secret<T> {}

impl<T: Zeroize> std::ops::Deref for Secret<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Zeroize> From<T> for Secret<T> {
    fn from(val: T) -> Self {
        Self(val)
    }
}

impl<T: Zeroize> AsRef<T> for Secret<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> std::borrow::Borrow<T> for Secret<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

/// Zero the inner value on drop when `T` supports it.
/// This prevents the secret from lingering in heap memory after the struct is dropped.
impl<T: Zeroize> Zeroize for Secret<T> {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

/// Automatically zero the heap when the `Secret` goes out of scope.
/// Applies whenever `T: Zeroize` (e.g. `Secret<String>`, `Secret<Vec<u8>>`).
impl<T: Zeroize> Drop for Secret<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(feature = "serde")]
impl<T: Zeroize> crate::serde::Serialize for Secret<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: crate::serde::Serializer,
    {
        serializer.serialize_str("[REDACTED]")
    }
}

#[cfg(feature = "serde")]
impl<'de, T: Zeroize + crate::serde::Deserialize<'de>> crate::serde::Deserialize<'de> for Secret<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: crate::serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}

impl<T: FromEnvStr + Zeroize> FromEnvStr for Secret<T> {
    type Err = T::Err;

    const REDACT_IN_ERRORS: bool = true;

    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        T::from_env_str(s).map(Self::new)
    }

    fn missing_value(key: &str) -> Result<Self, crate::error::EnvLockError> {
        T::missing_value(key).map(Self::new)
    }
}

/// Splits `s` into an iterator of `(number_str, unit_str)` segments,
/// e.g. `"1h30m"` → `[("1","h"),("30","m")]`.
fn duration_segments(s: &str) -> impl Iterator<Item = Result<(&str, &str), String>> {
    let mut rest = s;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let num_len = rest
            .chars()
            .take_while(char::is_ascii_digit)
            .map(char::len_utf8)
            .sum::<usize>();
        if num_len == 0 {
            return Some(Err(format!("expected digit at {rest:?}")));
        }
        let (n_str, tail) = rest.split_at(num_len);
        let unit_len = tail
            .chars()
            .take_while(char::is_ascii_alphabetic)
            .map(char::len_utf8)
            .sum::<usize>();
        if unit_len == 0 {
            return Some(Err(format!("missing unit after {n_str:?}")));
        }
        let (u_str, next) = tail.split_at(unit_len);
        rest = next;
        Some(Ok((n_str, u_str)))
    })
}

impl FromEnvStr for Duration {
    type Err = String;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("empty duration string".into());
        }
        duration_segments(s).try_fold(Duration::ZERO, |acc, seg| {
            let (n_str, u_str) = seg?;
            let n: u64 = n_str
                .parse()
                .map_err(|_| format!("bad number {n_str:?}"))?;
            let added = match u_str {
                "h"  => Duration::from_secs(n.checked_mul(3600).ok_or("overflow in hours")? ),
                "m"  => Duration::from_secs(n.checked_mul(60).ok_or("overflow in minutes")?),
                "s"  => Duration::from_secs(n),
                "ms" => Duration::from_millis(n),
                other => return Err(format!("unknown duration unit {other:?} — use h, m, s, ms")),
            };
            acc.checked_add(added).ok_or_else(|| "duration total overflow".into())
        })
    }
}

impl<T: FromEnvStr> FromEnvStr for Option<T> {
    type Err = T::Err;

    /// Propagated from `T` so that `Option<Secret<String>>` still redacts.
    const REDACT_IN_ERRORS: bool = T::REDACT_IN_ERRORS;

    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Ok(None)
        } else {
            T::from_env_str(s).map(Some)
        }
    }

    /// An absent `Option` field is `None`, not an error.
    fn missing_value(_key: &str) -> Result<Self, crate::error::EnvLockError> {
        Ok(None)
    }
}

// --- feature: url-type ---

#[cfg(feature = "url-type")]
impl FromEnvStr for url::Url {
    type Err = url::ParseError;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        url::Url::parse(s)
    }
}
