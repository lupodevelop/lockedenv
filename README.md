# lockedenv

> Ergonomic, type-safe, freeze-on-load environment variable management for Rust 🦀.

**Read once, parse immediately, freeze forever.**

[![crates.io](https://img.shields.io/crates/v/lockedenv.svg)](https://crates.io/crates/lockedenv) [![docs.rs](https://img.shields.io/docsrs/lockedenv)](https://docs.rs/lockedenv) [![license-MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE-MIT) [![license-Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE-APACHE)

Environment variables are often a source of subtle bugs: they are read multiple times across the codebase, treated as untyped `String`s, and can silently fail if mutated at runtime. Testing them natively with `std::env::set_var` is unsafe in parallel contexts. 

`lockedenv` solves this cleanly: define a struct layout via a macro, enforce type-safe parsing at startup, and pass the generated, immutable struct to your application.

## Quickstart

Add `lockedenv` to your `Cargo.toml`:

```toml
[dependencies]
lockedenv = "0.1"
```

Use the `load!` macro to define and parse your configuration:

```rust
fn main() {
    let config = lockedenv::load! {
        PORT:         u16,
        DATABASE_URL: String,
        DEBUG:        bool                = false,
        TIMEOUT:      std::time::Duration = std::time::Duration::from_secs(30),
        SENTRY_DSN:   Option<String>,
    };

    // The generated 'config' struct implements `Clone` and `Debug`
    println!("Listening on port {} in debug mode: {}", config.PORT, config.DEBUG);
}
```

If a required variable is missing or cannot be parsed, `load!` **panics with a clear message** describing the variable name, the value found, and a hint on how to fix it. Your application cannot boot into an invalid state.

## Core Features

- **Safe:** Eliminates repeated `std::env::var` calls. Validates everything on startup.
- **Type-safe:** Built-in parsers for standard library types (`u16`, `bool`, `IpAddr`, `std::time::Duration`, etc.) and seamlessly extensible via `FromEnvStr`.
- **Zero-Boilerplate Default & Optional values:** Naturally handles `fallback = defaults` and `Option<T>` for transparent absences.
- **Thread-safe testing:** The `from_map!` macro allows you to inject HashMaps into the parser, avoiding the deprecation and threading issues of `std::env::set_var`.
- **Hygienic:** Generates an isolated, anonymous struct ensuring no namespace pollution.

## The Macro Family

`lockedenv` provides straightforward variants for different needs:

```rust
// 1. Panics on missing/bad config (Recommended for standard microservices)
let config = lockedenv::load! { PORT: u16, DS_URL: String };

// 2. Returns Result<_, EnvLockError> to manually handle or propagate failures
let config = lockedenv::try_load! { PORT: u16 }?;
```

### Thread-Safe Testing

In tests, mutating the global environment is an anti-pattern. Let `lockedenv` parse directly from a collection map:

```rust
#[test]
fn test_config_parsing() {
    let map = std::collections::HashMap::from([
        ("PORT".into(), "8080".into())
    ]);

    let config = lockedenv::from_map! { map: map, PORT: u16 };
    assert_eq!(config.PORT, 8080);
}
```

## Supported Types (Zero extra dependencies)

| Rust Type | Syntax Example | Notes |
|-----------|----------------|-------|
| `String`, `char` | `"value"`, `'a'` | |
| Integer primitives | `8080`, `-20` | Native bounds checked |
| Floating point | `"3.14"` | |
| `bool` | `"true"`, `"1"`, `"yes"`, `"false"` | Case-insensitive |
| `std::path::PathBuf` | `"/etc/hosts"` | Does not check disk presence |
| `IpAddr`, `SocketAddr` | `"127.0.0.1"`, `"0.0.0.0:8080"` | |
| `std::time::Duration` | `"30s"`, `"1h30m45s"`, `"500ms"` | Safe functional parser |
| `Vec<T>` | `"a,b,c"`, `"80,443"` | Comma separated lists |
| `lockedenv::Secret<T>` | "password" | Redacts value in `Debug` and `Serialize` logs |
| `Option<T>` | `None` if absent | Overrides `FromEnvStr` absent behavior |

You can add support for your own types by simply implementing `lockedenv::parse::FromEnvStr`.

```rust
use lockedenv::parse::FromEnvStr;

struct Retries(u8);

impl FromEnvStr for Retries {
    type Err = String;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        let n: u8 = s.parse().map_err(|e| format!("{}", e))?;
        if n > 10 {
            return Err("max 10 retries".into());
        }
        Ok(Retries(n))
    }
}
```

### Naming Convention

Field names in the macro match the real environment variable name exactly, including case. By convention variables are `UPPER_SNAKE_CASE` and are accessed the same way on the generated struct:

```rust
let config = lockedenv::load! { DATABASE_URL: String, MAX_CONN: u32 };
println!("{} (max {})", config.DATABASE_URL, config.MAX_CONN);
```

When wrapping in a typed application struct, map at the boundary:

```rust
struct AppConfig { db_url: String, max_conn: u32 }

impl AppConfig {
    fn from_env() -> Self {
        let raw = lockedenv::load! { DATABASE_URL: String, MAX_CONN: u32 = 10 };
        Self { db_url: raw.DATABASE_URL, max_conn: raw.MAX_CONN }
    }
}
```

### Custom Error Hints

When implementing `FromEnvStr`, you can attach runtime hints to parse errors via `EnvLockError::with_hint`. This makes the fail-fast message much clearer for the operator:

```rust
use lockedenv::{parse::FromEnvStr, EnvLockError};

struct Port(u16);

impl FromEnvStr for Port {
    type Err = String;
    fn from_env_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u16>().map(Port).map_err(|_| "not a valid port (0–65535)".into())
    }
}

// Or attach hints after the fact:
let e = EnvLockError::parse_error("TIMEOUT".into(), "5min".into(), "unknown unit")
    .with_hint("use 5m or 5s instead");
// prints: expected type: unknown unit
//         hint: use 5m or 5s instead
```

## Optional Features

Extend `lockedenv` by enabling features in `Cargo.toml`.

| Feature | Description |
|---------|-------------|
| `dotenv` | Unlocks `load_dotenv!("path", { ... })` macros using [`dotenvy`](https://crates.io/crates/dotenvy). |
| `serde`  | Automatically derives `Serialize` and `Deserialize` on your generated configuration struct. Great for debug logging / dumping config state. |
| `watch`  | Provides `lockedenv::watch!` for async, background-thread interval drift detection. Generates a listener delta without heavy file watchers. |
| `url-type`| Connects directly to the [`url`](https://crates.io/crates/url) crate for strong `url::Url` typing. |
| `tracing`| Automatically logs the loaded configuration struct (with redacted secrets) at `INFO` level using the [`tracing`](https://crates.io/crates/tracing) crate upon successful load. |

### Prefixes & Secrets

If your environment variables share a common prefix, declare it once at the macro level:

```rust
// Reads APP_PORT and APP_TOKEN from the environment
let config = lockedenv::load! {
    prefix = "APP_",
    PORT:  u16,
    TOKEN: lockedenv::Secret<String>,
};
```

`Secret<T>` wraps any type and redacts its value in `Debug` output and `serde` serialization — useful when logging the config state at startup:

```rust
// Printing the config is always safe:
println!("{:?}", config); // { PORT: 8080, TOKEN: Secret([REDACTED]) }

// Access the real value when needed:
let token: &str = config.TOKEN.as_ref(); // AsRef<String>
let owned: String = config.TOKEN.clone().into_inner();
let s: lockedenv::Secret<String> = String::from("raw").into(); // From<T>
```

### Feature Showcase: Watcher
Ideal for environments (like K8s or Docker) where external factors could unexpectedly orchestrate config shifts at runtime. Note that dropping the handle stops the watcher cleanly.

```rust
// Requires: lockedenv = { version = "0.1", features = ["watch"] }
let config = lockedenv::load! { TARGET_URL: String };

// Checks every 5 seconds securely in the background.
let _handle = lockedenv::watch!(interval_secs = 5, on_drift = |key, old, new| {
    eprintln!("Drift Alert: {} shifted from {} to {}", key, old, new);
});
```


---

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

---

made with Rust 🦀
