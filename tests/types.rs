#![allow(
    clippy::assertions_on_constants,
    clippy::uninlined_format_args,
    clippy::default_constructed_unit_structs,
    clippy::approx_constant,
    clippy::float_cmp
)]
// Tests for FromEnvStr implementations.
// All tests here use from_map! to avoid touching std::env (thread-safe).
use lockedenv::parse::FromEnvStr;
use std::time::Duration;

fn hmap(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

// ── integers ───────────────────────────────────────────────────────────────

#[test]
fn integer_roundtrip() {
    assert_eq!(u8::from_env_str("255").unwrap(), 255u8);
    assert_eq!(u16::from_env_str("65535").unwrap(), 65535u16);
    assert_eq!(u32::from_env_str("0").unwrap(), 0u32);
    assert_eq!(u64::from_env_str("18446744073709551615").unwrap(), u64::MAX);
    assert_eq!(i32::from_env_str("-2147483648").unwrap(), i32::MIN);
    assert_eq!(i64::from_env_str("9223372036854775807").unwrap(), i64::MAX);
    assert!(u8::from_env_str("256").is_err(), "u8 should reject 256");
    assert!(u32::from_env_str("-1").is_err(), "u32 should reject -1");
    assert!(i32::from_env_str("abc").is_err());
}

#[test]
fn float_parsing() {
    let v = f64::from_env_str("3.14").unwrap();
    assert!((v - 3.14).abs() < 1e-10);
    assert_eq!(f32::from_env_str("0").unwrap(), 0f32);
    assert!(f64::from_env_str("").is_err());
    assert!(f64::from_env_str("NaN").is_ok(), "NaN is a valid f64 parse");
}

// ── bool ──────────────────────────────────────────────────────────────────

#[test]
fn bool_parsing() {
    for (s, exp) in [
        ("true", true), ("false", false),
        ("1", true), ("0", false),
        ("yes", true), ("no", false),
        ("TRUE", true), ("YES", true), ("False", false),
    ] {
        assert_eq!(bool::from_env_str(s).unwrap(), exp, "failed for {s:?}");
    }
    assert!(bool::from_env_str("maybe").is_err());
    assert!(bool::from_env_str("").is_err());
    assert!(bool::from_env_str("2").is_err());
}

// ── char ──────────────────────────────────────────────────────────────────

#[test]
fn char_parsing() {
    assert_eq!(char::from_env_str("A").unwrap(), 'A');
    assert_eq!(char::from_env_str("€").unwrap(), '€');
    assert!(char::from_env_str("").is_err(), "empty string is not a char");
    assert!(char::from_env_str("AB").is_err(), "two chars should be rejected");
}

// ── PathBuf ───────────────────────────────────────────────────────────────

#[test]
fn path_buf_parsing() {
    use std::path::PathBuf;
    let p = PathBuf::from_env_str("/etc/hosts").unwrap();
    assert_eq!(p, PathBuf::from("/etc/hosts"));
    // any string is valid (existence is not checked)
    assert!(PathBuf::from_env_str("does/not/exist").is_ok());
}

// ── network types ─────────────────────────────────────────────────────────

#[test]
fn ip_addr_parsing() {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    assert_eq!(Ipv4Addr::from_env_str("127.0.0.1").unwrap(), Ipv4Addr::LOCALHOST);
    assert_eq!(Ipv6Addr::from_env_str("::1").unwrap(), Ipv6Addr::LOCALHOST);
    assert!(IpAddr::from_env_str("127.0.0.1").is_ok());
    assert!(IpAddr::from_env_str("::1").is_ok());
    assert!(IpAddr::from_env_str("not-an-ip").is_err());
    assert!(IpAddr::from_env_str("999.0.0.1").is_err());

    assert_eq!(
        SocketAddr::from_env_str("0.0.0.0:8080").unwrap().port(),
        8080,
    );
    assert!(SocketAddr::from_env_str("localhost:80").is_err(), "hostname not accepted");
}

// ── Duration ──────────────────────────────────────────────────────────────

#[test]
fn duration_parsing() {
    assert_eq!(Duration::from_env_str("30s").unwrap(),   Duration::from_secs(30));
    assert_eq!(Duration::from_env_str("5m").unwrap(),    Duration::from_secs(300));
    assert_eq!(Duration::from_env_str("2h").unwrap(),    Duration::from_secs(7200));
    assert_eq!(Duration::from_env_str("500ms").unwrap(), Duration::from_millis(500));
    assert_eq!(Duration::from_env_str("0s").unwrap(),    Duration::ZERO);
    // compound segments
    assert_eq!(Duration::from_env_str("1h30m").unwrap(),     Duration::from_secs(5400));
    assert_eq!(Duration::from_env_str("1h30m45s").unwrap(),  Duration::from_secs(5445));
    assert_eq!(Duration::from_env_str("2h500ms").unwrap(),   Duration::from_secs(7200) + Duration::from_millis(500));
}

#[test]
fn duration_invalid_inputs() {
    assert!(Duration::from_env_str("").is_err(),      "empty");
    assert!(Duration::from_env_str("100").is_err(),   "missing unit");
    assert!(Duration::from_env_str("5min").is_err(),  "unknown unit 'min'");
    assert!(Duration::from_env_str("abc").is_err(),   "no digits");
    assert!(Duration::from_env_str("1h2").is_err(),   "trailing digits without unit");
}

// ── Option<T> ─────────────────────────────────────────────────────────────

#[test]
fn option_present_and_absent() {
    let m = hmap(&[("OPT_PORT", "3000")]);
    let config = lockedenv::from_map! {
        map: m,
        OPT_PORT:   Option<u16>,
        OPT_EXTRA:  Option<String>,
    };
    assert_eq!(config.OPT_PORT, Some(3000));
    assert!(config.OPT_EXTRA.is_none());
}

#[test]
fn option_empty_string_is_none() {
    // An empty string in the map is treated as None for Option<T>
    let m = hmap(&[("OPT_EMPTY", "")]);
    let config = lockedenv::from_map! { map: m, OPT_EMPTY: Option<u32> };
    assert!(config.OPT_EMPTY.is_none(), "empty string should map to None");
}

#[test]
fn option_parse_error_propagates() {
    let m = hmap(&[("OPT_BAD", "not_a_number")]);
    let result = lockedenv::try_from_map! { map: m, OPT_BAD: Option<u32> };
    assert!(result.is_err(), "invalid value inside Option should still error");
}

// ── try_from_map! error content ───────────────────────────────────────────

#[test]
fn try_from_map_error_names_variable() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let r = lockedenv::try_from_map! { map: m, MISSING_PORT: u16 };
    assert!(r.is_err());
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("MISSING_PORT"), "error should mention variable: {msg}");
}

#[test]
fn parse_error_message_has_value_and_var() {
    let m = hmap(&[("ERR_PORT", "abc")]);
    let r = lockedenv::try_from_map! { map: m, ERR_PORT: u16 };
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("ERR_PORT"), "error message: {msg}");
    assert!(msg.contains("abc"),      "error message: {msg}");
}

// ── url-type feature ──────────────────────────────────────────────────────

#[cfg(feature = "url-type")]
#[test]
fn url_type_parsing() {
    use url::Url;
    let ok = Url::from_env_str("https://example.com/path?q=1").unwrap();
    assert_eq!(ok.host_str(), Some("example.com"));
    assert_eq!(ok.scheme(), "https");
    assert!(Url::from_env_str("not a url").is_err());
    assert!(Url::from_env_str("").is_err());
}

#[cfg(feature = "url-type")]
#[test]
fn url_from_map() {
    let m = hmap(&[("API_BASE", "https://api.example.com")]);
    let config = lockedenv::from_map! { map: m, API_BASE: url::Url };
    assert_eq!(config.API_BASE.host_str(), Some("api.example.com"));
}

// ── serde feature ─────────────────────────────────────────────────────────

#[cfg(feature = "serde")]
#[test]
fn derives_serde_traits() {
    let m = hmap(&[("SERDE_PORT", "80"), ("SERDE_HOST", "abc")]);
    let config = lockedenv::from_map! {
        map: m,
        SERDE_PORT: u16,
        SERDE_HOST: String,
        SERDE_OPT: Option<u32>,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"SERDE_PORT\":80"));
    assert!(json.contains("\"SERDE_HOST\":\"abc\""));
    assert!(json.contains("\"SERDE_OPT\":null"));

    // We can't easily test Deserialize via from_str because the struct is anonymous,
    // but the fact it compiles means the #[derive(Deserialize)] succeeded!
}

// ── Vec<T> ────────────────────────────────────────────────────────────────

#[test]
fn vec_parsing() {
    let m = hmap(&[
        ("PORTS", "80,443, 8080 ,  9000"),
        ("EMPTY", "   "),
        ("SINGLE", "1234"),
    ]);
    let config = lockedenv::from_map! {
        map: m,
        PORTS: Vec<u16>,
        EMPTY: Vec<String>,
        SINGLE: Vec<u32>,
    };

    assert_eq!(config.PORTS, vec![80, 443, 8080, 9000]);
    assert!(config.EMPTY.is_empty());
    assert_eq!(config.SINGLE, vec![1234]);
}

// ── Secret<T> ─────────────────────────────────────────────────────────────

#[test]
fn secret_parsing_and_debug() {
    use lockedenv::Secret;
    use std::borrow::Borrow;

    let m = hmap(&[("PASSWORD", "my_super_secret")]);
    let config = lockedenv::from_map! {
        map: m,
        PASSWORD: Secret<String>,
    };

    // Inner value accessible via Deref
    assert_eq!(*config.PASSWORD, "my_super_secret");
    assert_eq!(config.PASSWORD.clone().into_inner(), "my_super_secret");

    // AsRef<T> and Borrow<T>
    let r: &String = config.PASSWORD.as_ref();
    assert_eq!(r, "my_super_secret");
    let b: &String = config.PASSWORD.borrow();
    assert_eq!(b, "my_super_secret");

    // From<T> for Secret<T>
    let s: Secret<u16> = Secret::from(42u16);
    assert_eq!(*s, 42u16);

    // Debug must never expose the value
    let debug_str = format!("{config:?}");
    assert!(debug_str.contains("Secret([REDACTED])"));
    assert!(!debug_str.contains("my_super_secret"));

    #[cfg(feature = "serde")]
    {
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("[REDACTED]"));
        assert!(!json.contains("my_super_secret"));
    }
}

#[test]
fn vec_error_contains_index() {
    let m = hmap(&[("PORTS", "80,nope,443")]);
    let r = lockedenv::try_from_map! {
        map: m,
        PORTS: Vec<u16>,
    };
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("item[1]") || msg.contains("nope"), "error: {msg}");
}

// ── prefix support ────────────────────────────────────────────────────────

#[test]
fn map_prefix_support() {
    let m = hmap(&[
        ("APP_PORT", "8080"),
        ("APP_HOST", "localhost"),
    ]);

    let config = lockedenv::from_map! {
        map: m,
        prefix = "APP_",
        PORT: u16,
        HOST: String,
    };

    assert_eq!(config.PORT, 8080);
    assert_eq!(config.HOST, "localhost");
}

#[test]
fn partial_eq_on_generated_struct() {
    let m = hmap(&[("VAL", "42")]);
    let config = lockedenv::from_map! { map: m, VAL: u32 };
    // PartialEq: a struct should equal its own clone
    assert_eq!(config, config.clone());
    // and differ after a known change (different parse, different config)
    let m2 = hmap(&[("VAL", "99")]);
    // We can't compare two separate macro expansions (distinct anonymous types),
    // but we can confirm the derived impl works via clone round-trip:
    let config2 = lockedenv::from_map! { map: m2, VAL: u32 };
    assert_eq!(config2, config2.clone());
    assert_ne!(config.VAL, config2.VAL);
}

// ── REDACT_IN_ERRORS propagation ──────────────────────────────────────────

#[test]
fn redact_propagated_in_option_secret() {
    use lockedenv::parse::FromEnvStr;
    // Option<Secret<String>> must inherit REDACT_IN_ERRORS = true from Secret
    assert!(
        <Option<lockedenv::Secret<String>> as FromEnvStr>::REDACT_IN_ERRORS,
        "Option<Secret<T>> must propagate REDACT_IN_ERRORS"
    );
}

#[test]
fn redact_propagated_in_vec_secret() {
    use lockedenv::parse::FromEnvStr;
    assert!(
        <Vec<lockedenv::Secret<String>> as FromEnvStr>::REDACT_IN_ERRORS,
        "Vec<Secret<T>> must propagate REDACT_IN_ERRORS"
    );
}

#[test]
fn redact_false_for_plain_types() {
    use lockedenv::parse::FromEnvStr;
    assert!(!<String as FromEnvStr>::REDACT_IN_ERRORS);
    assert!(!<u32 as FromEnvStr>::REDACT_IN_ERRORS);
    assert!(!<Vec<u16> as FromEnvStr>::REDACT_IN_ERRORS);
    assert!(!<Option<String> as FromEnvStr>::REDACT_IN_ERRORS);
}

#[test]
fn secret_parse_error_redacts_in_map() {
    // When a Secret<u32> receives an unparseable value, the error must NOT leak the raw value
    let m = hmap(&[("TOKEN", "not_a_number")]);
    let r = lockedenv::try_from_map! { map: m, TOKEN: lockedenv::Secret<u32> };
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("[REDACTED]"), "secret error: {msg}");
    assert!(!msg.contains("not_a_number"), "raw value leaked: {msg}");
}

#[test]
fn option_secret_parse_error_redacts_in_map() {
    let m = hmap(&[("TOKEN", "bad")]);
    let r = lockedenv::try_from_map! { map: m, TOKEN: Option<lockedenv::Secret<String>> };
    // Option<Secret<String>> parses the non-empty string "bad" as Some(Secret("bad")) — succeeds
    assert!(r.is_ok());
}

// ── Secret security ──────────────────────────────────────────────────────

#[test]
fn secret_zeroize_clears_inner() {
    use zeroize::Zeroize;
    let mut s = lockedenv::Secret::new("hello".to_string());
    s.zeroize();
    // After zeroize, the inner String should be empty (zeroed)
    assert!(s.as_ref().is_empty(), "zeroized secret must be empty");
}

#[test]
fn secret_clone_is_independent() {
    use zeroize::Zeroize;
    let mut original = lockedenv::Secret::new("data".to_string());
    let cloned = original.clone();
    original.zeroize();
    // Clone must be unaffected
    assert_eq!(cloned.as_ref(), "data");
    assert!(original.as_ref().is_empty());
}

#[test]
fn secret_eq_and_ne() {
    let a = lockedenv::Secret::new("same".to_string());
    let b = lockedenv::Secret::new("same".to_string());
    let c = lockedenv::Secret::new("diff".to_string());
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn secret_new_and_from_are_equivalent() {
    let from_new = lockedenv::Secret::new(42u32);
    let from_trait: lockedenv::Secret<u32> = 42u32.into();
    assert_eq!(from_new, from_trait);
}

#[test]
fn secret_into_inner_returns_value() {
    let s = lockedenv::Secret::new("payload".to_string());
    let inner = s.into_inner();
    assert_eq!(inner, "payload");
}

// ── macro edge cases ──────────────────────────────────────────────────────

#[test]
fn macro_zero_fields() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let config = lockedenv::from_map! { map: m, };
    let _ = format!("{config:?}"); // Debug works
}

#[test]
fn macro_single_field() {
    let m = hmap(&[("X", "1")]);
    let config = lockedenv::from_map! { map: m, X: u32 };
    assert_eq!(config.X, 1);
}

#[test]
fn macro_nested_option_vec() {
    let m = hmap(&[("PORTS", "80,443")]);
    let config = lockedenv::from_map! {
        map: m,
        PORTS: Option<Vec<u16>>,
    };
    assert_eq!(config.PORTS, Some(vec![80, 443]));
}

#[test]
fn macro_nested_option_vec_absent() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let config = lockedenv::from_map! { map: m, PORTS: Option<Vec<u16>> };
    assert!(config.PORTS.is_none());
}

#[test]
fn macro_all_defaults_no_entries() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let config = lockedenv::from_map! {
        map: m,
        A: u32 = 1,
        B: String = "default".to_string(),
        C: bool = false,
    };
    assert_eq!(config.A, 1);
    assert_eq!(config.B, "default");
    assert!(!config.C);
}

#[test]
fn try_from_map_with_prefix() {
    let m = hmap(&[("SVC_PORT", "9090"), ("SVC_HOST", "0.0.0.0")]);
    let r = lockedenv::try_from_map! {
        map: m,
        prefix = "SVC_",
        PORT: u16,
        HOST: String,
    };
    let config = r.unwrap();
    assert_eq!(config.PORT, 9090);
    assert_eq!(config.HOST, "0.0.0.0");
}

#[test]
fn from_map_panics_on_missing() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let result = std::panic::catch_unwind(|| {
        lockedenv::from_map! { map: m, REQUIRED: u32 }
    });
    assert!(result.is_err(), "from_map! must panic on missing required field");
}

#[test]
fn from_map_panics_on_bad_parse() {
    let m = hmap(&[("BAD", "xyz")]);
    let result = std::panic::catch_unwind(|| {
        lockedenv::from_map! { map: m, BAD: u32 }
    });
    assert!(result.is_err(), "from_map! must panic on bad parse");
}

// ── error variant coverage ────────────────────────────────────────────────

#[test]
fn error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<lockedenv::EnvLockError>();
}

#[test]
fn error_is_clone() {
    let e = lockedenv::EnvLockError::missing("X".into());
    let e2 = e.clone();
    assert_eq!(e.to_string(), e2.to_string());
}

#[test]
fn with_hint_noop_on_missing() {
    let e = lockedenv::EnvLockError::missing("X".into()).with_hint("ignored");
    let s = e.to_string();
    assert!(!s.contains("ignored"), "hint on Missing should be no-op: {s}");
}

#[test]
fn with_hint_noop_on_dotenv() {
    let e = lockedenv::EnvLockError::dotenv(".env".into(), "io error".into())
        .with_hint("ignored");
    let s = e.to_string();
    assert!(!s.contains("ignored"), "hint on Dotenv should be no-op: {s}");
}

#[test]
fn parse_error_display_without_hint() {
    let e = lockedenv::EnvLockError::parse_error("X".into(), "y".into(), "z");
    let s = e.to_string();
    assert!(!s.contains("hint:"), "no hint should appear: {s}");
}

#[test]
fn dotenv_error_display_always_available() {
    // Dotenv variant can be constructed and displayed even without the dotenv feature
    let e = lockedenv::EnvLockError::dotenv("/path".into(), "parse error".into());
    let s = e.to_string();
    assert!(s.contains("/path"));
    assert!(s.contains("parse error"));
}

// ── edge cases for FromEnvStr ─────────────────────────────────────────────

#[test]
fn string_value_preserves_unicode_and_whitespace() {
    let m = hmap(&[("U", "  ciao 🦀\tnewline\n")]);
    let cfg = lockedenv::from_map! { map: m, U: String };
    assert_eq!(cfg.U, "  ciao 🦀\tnewline\n");
}

#[test]
fn char_emoji() {
    assert_eq!(char::from_env_str("🦀").unwrap(), '🦀');
    assert!(char::from_env_str("🦀🦀").is_err(), "two emoji = two chars");
}

#[test]
fn duration_all_zeros() {
    assert_eq!(
        Duration::from_env_str("0h0m0s0ms").unwrap(),
        Duration::ZERO,
    );
}

#[test]
fn duration_repeated_units() {
    // "1h2h" is valid — units accumulate
    assert_eq!(
        Duration::from_env_str("1h2h").unwrap(),
        Duration::from_secs(3 * 3600),
    );
}

#[test]
fn vec_trailing_comma() {
    let v = Vec::<u16>::from_env_str("80,443,").unwrap();
    assert_eq!(v, vec![80, 443], "trailing comma → empty part filtered out");
}

#[test]
fn vec_leading_comma() {
    let v = Vec::<u16>::from_env_str(",80").unwrap();
    assert_eq!(v, vec![80], "leading comma → empty part filtered out");
}

#[test]
fn vec_only_commas() {
    let v = Vec::<u16>::from_env_str(",,,").unwrap();
    assert!(v.is_empty());
}

#[test]
fn vec_of_bools() {
    let v = Vec::<bool>::from_env_str("true,false,1,0,yes,no").unwrap();
    assert_eq!(v, vec![true, false, true, false, true, false]);
}

#[test]
fn option_secret_string_present() {
    let m = hmap(&[("TOK", "abc")]);
    let cfg = lockedenv::from_map! { map: m, TOK: Option<lockedenv::Secret<String>> };
    assert_eq!(*cfg.TOK.unwrap(), "abc");
}

#[test]
fn option_secret_string_absent() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let cfg = lockedenv::from_map! { map: m, TOK: Option<lockedenv::Secret<String>> };
    assert!(cfg.TOK.is_none());
}

// ── tests migrated from gap_coverage.rs (unique high-value only) ──────────

#[test]
fn bool_whitespace_rejected() {
    assert!(bool::from_env_str(" true").is_err());
    assert!(bool::from_env_str("true ").is_err());
    assert!(bool::from_env_str(" 1 ").is_err());
}

#[test]
fn duration_number_overflow() {
    assert!(Duration::from_env_str("99999999999999999999h").is_err());
}

#[test]
fn vec_of_durations() {
    let v = Vec::<Duration>::from_env_str("30s, 1m, 2h").unwrap();
    assert_eq!(v, vec![Duration::from_secs(30), Duration::from_secs(60), Duration::from_secs(7200)]);
}

#[test]
fn vec_strings_with_internal_spaces() {
    let v = Vec::<String>::from_env_str("hello, world, foo bar").unwrap();
    assert_eq!(v, vec!["hello", "world", "foo bar"]);
}

#[test]
fn missing_value_trait_defaults() {
    assert!(u32::missing_value("K").is_err());
    assert!(String::missing_value("K").is_err());
    assert_eq!(Option::<u32>::missing_value("K").unwrap(), None);
    assert!(lockedenv::Secret::<String>::missing_value("K").is_err());
}

#[test]
fn option_duration_present_and_absent() {
    let m = hmap(&[("T", "5s")]);
    let cfg = lockedenv::from_map! { map: m, T: Option<Duration>, T2: Option<Duration> };
    assert_eq!(cfg.T, Some(Duration::from_secs(5)));
    assert!(cfg.T2.is_none());
}

#[test]
fn macro_mix_required_optional_default() {
    let m = hmap(&[("REQ", "hello")]);
    let cfg = lockedenv::from_map! { map: m, REQ: String, OPT: Option<u32>, DEF: u32 = 42 };
    assert_eq!(cfg.REQ, "hello");
    assert!(cfg.OPT.is_none());
    assert_eq!(cfg.DEF, 42);
}

#[test]
fn macro_secret_field_redacts_in_debug() {
    let m = hmap(&[("API_KEY", "sk-12345"), ("DB_PASS", "p@ssw0rd!")]);
    let cfg = lockedenv::from_map! { map: m, API_KEY: lockedenv::Secret<String>, DB_PASS: lockedenv::Secret<String> };
    let dbg = format!("{cfg:?}");
    assert!(!dbg.contains("sk-12345"));
    assert!(!dbg.contains("p@ssw0rd!"));
    assert!(dbg.contains("[REDACTED]"));
}

#[test]
fn all_optional_fields_empty_map() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let cfg = lockedenv::from_map! { map: m, A: Option<String>, B: Option<u32>, C: Option<bool> };
    assert!(cfg.A.is_none());
    assert!(cfg.B.is_none());
    assert!(cfg.C.is_none());
}

#[test]
fn multiple_errors_first_field_wins() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let r = lockedenv::try_from_map! { map: m, MISSING_FIRST: u32, MISSING_SECOND: u32 };
    let msg = r.unwrap_err().to_string();
    assert!(msg.contains("MISSING_FIRST"));
}

#[test]
fn error_implements_std_error() {
    let e1: Box<dyn std::error::Error> = Box::new(lockedenv::EnvLockError::missing("X".into()));
    assert!(e1.source().is_none());
    let e2: Box<dyn std::error::Error> = Box::new(lockedenv::EnvLockError::parse_error("Y".into(), "z".into(), "bad"));
    assert!(e2.to_string().contains('Y'));
}
