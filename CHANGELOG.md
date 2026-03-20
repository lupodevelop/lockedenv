# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
