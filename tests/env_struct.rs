#![allow(non_snake_case, clippy::uninlined_format_args)]
// Tests for the env_struct! macro — named, publicly usable config structs.

fn hmap(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

// Defined at module level — that's the whole point of env_struct!
lockedenv::env_struct! {
    pub struct BasicConfig {
        HOST: String,
        PORT: u16,
        DEBUG: bool = false,
        LABEL: Option<String>,
    }
}

lockedenv::env_struct! {
    pub struct PrefixedConfig {
        prefix = "SVC_",
        HOST: String,
        PORT: u16 = 8080,
    }
}

// ── from_map ──────────────────────────────────────────────────────────────

#[test]
fn env_struct_from_map_basic() {
    let m = hmap(&[("HOST", "localhost"), ("PORT", "3000")]);
    let cfg = BasicConfig::from_map(&m);
    assert_eq!(cfg.HOST, "localhost");
    assert_eq!(cfg.PORT, 3000u16);
    assert!(!cfg.DEBUG);
    assert!(cfg.LABEL.is_none());
}

#[test]
fn env_struct_from_map_with_default_override() {
    let m = hmap(&[("HOST", "0.0.0.0"), ("PORT", "9999"), ("DEBUG", "true")]);
    let cfg = BasicConfig::from_map(&m);
    assert!(cfg.DEBUG);
    assert_eq!(cfg.PORT, 9999u16);
}

#[test]
fn env_struct_from_map_optional_present() {
    let m = hmap(&[("HOST", "h"), ("PORT", "1"), ("LABEL", "production")]);
    let cfg = BasicConfig::from_map(&m);
    assert_eq!(cfg.LABEL, Some("production".into()));
}

#[test]
fn env_struct_from_map_panics_on_missing() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let result = std::panic::catch_unwind(|| BasicConfig::from_map(&m));
    assert!(
        result.is_err(),
        "from_map must panic on missing required field"
    );
}

#[test]
fn env_struct_from_map_panics_on_bad_parse() {
    let m = hmap(&[("HOST", "localhost"), ("PORT", "not_a_port")]);
    let result = std::panic::catch_unwind(|| BasicConfig::from_map(&m));
    assert!(result.is_err(), "from_map must panic on parse error");
}

// ── try_from_map ──────────────────────────────────────────────────────────

#[test]
fn env_struct_try_from_map_ok() {
    let m = hmap(&[("HOST", "127.0.0.1"), ("PORT", "8080")]);
    let cfg = BasicConfig::try_from_map(&m).unwrap();
    assert_eq!(cfg.HOST, "127.0.0.1");
    assert_eq!(cfg.PORT, 8080u16);
}

#[test]
fn env_struct_try_from_map_err_missing() {
    let m = hmap(&[("HOST", "h")]);
    let result = BasicConfig::try_from_map(&m);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("PORT"), "error should name PORT: {msg}");
}

#[test]
fn env_struct_try_from_map_err_parse() {
    let m = hmap(&[("HOST", "h"), ("PORT", "xyz")]);
    let result = BasicConfig::try_from_map(&m);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("PORT"), "error should name PORT: {msg}");
    assert!(msg.contains("xyz"), "error should show bad value: {msg}");
}

// ── struct is a real named type ──────────────────────────────────────────

#[test]
fn env_struct_is_returnable_from_function() {
    fn load_test_config(m: &std::collections::HashMap<String, String>) -> BasicConfig {
        BasicConfig::from_map(m)
    }
    let m = hmap(&[("HOST", "localhost"), ("PORT", "80")]);
    let cfg = load_test_config(&m);
    assert_eq!(cfg.PORT, 80u16);
}

#[test]
fn env_struct_implements_debug_and_clone() {
    let m = hmap(&[("HOST", "h"), ("PORT", "1")]);
    let cfg = BasicConfig::from_map(&m);
    let cloned = cfg.clone();
    assert_eq!(cfg, cloned);
    let dbg = format!("{cfg:?}");
    assert!(dbg.contains("BasicConfig"));
}

#[test]
fn env_struct_fields_are_public() {
    let m = hmap(&[("HOST", "myhost"), ("PORT", "443")]);
    let cfg = BasicConfig::from_map(&m);
    // Direct field access (not just via methods)
    let _host: &str = &cfg.HOST;
    let _port: u16 = cfg.PORT;
}

// ── prefix support ────────────────────────────────────────────────────────

#[test]
fn env_struct_prefix_from_map() {
    let m = hmap(&[("SVC_HOST", "10.0.0.1"), ("SVC_PORT", "9090")]);
    let cfg = PrefixedConfig::from_map(&m);
    assert_eq!(cfg.HOST, "10.0.0.1");
    assert_eq!(cfg.PORT, 9090u16);
}

#[test]
fn env_struct_prefix_uses_default() {
    let m = hmap(&[("SVC_HOST", "h")]);
    let cfg = PrefixedConfig::from_map(&m);
    assert_eq!(cfg.PORT, 8080u16);
}

#[test]
fn env_struct_prefix_without_prefix_key_errors() {
    // "HOST" without prefix should not be found
    let m = hmap(&[("HOST", "h"), ("PORT", "1")]);
    let result = PrefixedConfig::try_from_map(&m);
    assert!(
        result.is_err(),
        "unprefixed key must not match prefixed field"
    );
}

// ── Secret<T> in env_struct ───────────────────────────────────────────────

lockedenv::env_struct! {
    pub struct SecretConfig {
        TOKEN: lockedenv::Secret<String>,
        PORT: u16 = 8080,
    }
}

#[test]
fn env_struct_secret_redacts_in_debug() {
    let m = hmap(&[("TOKEN", "s3cr3t")]);
    let cfg = SecretConfig::from_map(&m);
    let dbg = format!("{cfg:?}");
    assert!(!dbg.contains("s3cr3t"), "Debug must not leak secret: {dbg}");
    assert!(
        dbg.contains("[REDACTED]"),
        "Debug must contain [REDACTED]: {dbg}"
    );
}

#[test]
fn env_struct_secret_parse_error_redacts() {
    // Secret<u32> with bad value must not leak raw value in error
    lockedenv::env_struct! {
        pub struct SecretU32Config { TOKEN: lockedenv::Secret<u32> }
    }
    let m = hmap(&[("TOKEN", "not_a_number")]);
    let err = SecretU32Config::try_from_map(&m).unwrap_err().to_string();
    assert!(err.contains("[REDACTED]"), "error must redact: {err}");
    assert!(
        !err.contains("not_a_number"),
        "error must not leak raw: {err}"
    );
}
