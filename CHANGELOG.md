# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — 2026-04-22

### Added

- **`env_struct!` macro** — define a named, publicly accessible configuration struct with
  `load()`, `try_load()`, `from_map()`, and `try_from_map()` associated functions.
  Unlike the anonymous structs produced by `load!`, the generated struct has a real name
  and can be returned from functions, stored in other types, and used as a type annotation.
  Supports `prefix = "..."`, defaults, `Option<T>`, and `Secret<T>` exactly like the other macros.

- **`check!` and `try_check!` macros** — collect *all* parse/missing errors before failing,
  instead of stopping at the first one.
  - `try_check! { ... }` — returns `Ok(config)` on success or `Err(Vec<EnvLockError>)` listing
    every problem found.
  - `check! { ... }` — panics with a single message that lists all errors at once.
  - Both accept `map: <expr>` for HashMap injection and `prefix = "..."` for namespacing,
    matching the existing macro family.

- **Decimal `Duration` support** — `Duration` now accepts fractional units: `"1.5h"`, `"0.5s"`,
  `"2.25m"`, `"1.5ms"`. Conversion uses integer arithmetic (no floating-point); truncation
  to nanosecond precision. Compound segments like `"1.5h30s"` work as expected.

- **`with_hint` on all error variants** — `EnvLockError::with_hint()` now attaches a hint to
  `Missing` and `Dotenv` errors, not just `Parse`. The hint appears in `Display` output for
  all three variants. This is a **backward-compatible** change (the `#[non_exhaustive]`
  attribute on variants prevents external struct construction; adding `hint: Option<String>`
  does not break external consumers).

### Changed

- `EnvLockError::Missing` now includes a `hint: Option<String>` field (initialized to `None`
  by the `missing()` constructor).
- `EnvLockError::Dotenv` now includes a `hint: Option<String>` field (initialized to `None`
  by the `dotenv()` constructor).

### Tests

- Added 46 new tests covering all new features and previously untested behaviour:
  decimal Duration parsing (integer and compound segments), `env_struct!` (11 cases including
  prefix, secrets, public field access, function return), `check!`/`try_check!` (11 cases
  including error collection count, map variant, prefix, Option-is-not-error),
  watcher callback panic survival, and `with_hint` on all error variants.

## [0.2.0] — 2026-03-19

### Changed

- **`watch!` macro now requires an explicit `keys` list** (`keys = ["VAR1", "VAR2"]`).
  The watcher previously called `std::env::vars()` on every tick, scanning the entire
  process environment. It now reads only the declared keys via `std::env::var`, reducing
  per-tick cost from O(all env vars) to O(watched vars).

### Added

- `watcher::start` accepts a `Vec<String>` of keys to monitor.
- New `watch!` macro variants:
  - `watch!(keys = [...], interval_secs = N, on_drift = cb)`
  - `watch!(keys = [...], interval_ms = N, on_drift = cb)`
  - `watch!(keys = [...], on_drift = cb)` — defaults to 5 s interval
- New drift test: `watcher_detects_addition` — verifies a variable added after the
  initial snapshot is reported with `old = "<missing>"`.
- New drift test: `watcher_empty_keys_never_fires` — verifies that a watcher with no
  keys never invokes the callback.

### Fixed

- Removed redundant `zeroize` entry from `[dev-dependencies]` (already in
  `[dependencies]`).

### Documentation

- `Duration` type: documented that decimal values (e.g. `"1.5h"`) are not supported;
  only integer segments with units `h`, `m`, `s`, `ms` are accepted.
- `Option<T>` type: documented that both an absent key and an empty string (`VAR=""`)
  produce `None`.
- `Vec<T>` type: documented that leading, trailing, and consecutive commas produce no
  empty elements (they are silently skipped).
- Watcher section: updated example to use the new `keys = [...]` syntax.

## [0.1.0] — 2025-10-01

### Added

- `load!` / `try_load!` macros for type-safe, freeze-on-load env parsing.
- `from_map!` / `try_from_map!` macros for test-friendly HashMap injection.
- Built-in `FromEnvStr` implementations for: `String`, `char`, integer primitives,
  `f32`/`f64`, `bool` (true/false/1/0/yes/no), `PathBuf`, `IpAddr`, `Ipv4Addr`,
  `Ipv6Addr`, `SocketAddr`, `Duration`, `Vec<T>`, `Option<T>`.
- `Secret<T>` wrapper: redacts in `Debug` and `serde::Serialize`; zeroizes on drop.
- `prefix = "..."` support in all load macros.
- Optional features: `dotenv`, `serde`, `watch`, `url-type`, `tracing`.
- MSRV: Rust 1.70.
