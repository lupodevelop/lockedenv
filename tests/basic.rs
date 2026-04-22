#![allow(clippy::uninlined_format_args)]
// Tests for load! / try_load! macros (which interact with std::env).
// Each test uses unique variable names to avoid cross-test interference.

// ── load! / try_load! ──────────────────────────────────────────────────────

#[test]
fn load_basic_types() {
    std::env::set_var("BASIC_PORT", "8080");
    std::env::set_var("BASIC_DB_URL", "postgres://localhost/test");

    let config = lockedenv::load! {
        BASIC_PORT:   u16,
        BASIC_DB_URL: String,
        BASIC_DEBUG:  bool = false,
    };
    assert_eq!(config.BASIC_PORT, 8080);
    assert_eq!(config.BASIC_DB_URL, "postgres://localhost/test");
    assert!(!config.BASIC_DEBUG);
}

#[test]
fn load_implements_clone_and_debug() {
    std::env::set_var("CLDBG_PORT", "99");
    let cfg = lockedenv::load! { CLDBG_PORT: u16 };
    let cloned = cfg.clone();
    assert_eq!(cfg.CLDBG_PORT, cloned.CLDBG_PORT);
    let dbg = format!("{cfg:?}");
    assert!(
        dbg.contains("99"),
        "Debug output should contain field value"
    );
}

#[test]
fn try_load_returns_ok() -> Result<(), lockedenv::EnvLockError> {
    std::env::set_var("TRYLOAD_PORT", "1234");
    let config = lockedenv::try_load! { TRYLOAD_PORT: u16 }?;
    assert_eq!(config.TRYLOAD_PORT, 1234);
    Ok(())
}

#[test]
fn try_load_returns_err_on_missing() {
    std::env::remove_var("TRYLOAD_ABSENT");
    let result = lockedenv::try_load! { TRYLOAD_ABSENT: u32 };
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("TRYLOAD_ABSENT"),
        "error message should name the missing variable"
    );
}

#[test]
fn load_panics_on_missing() {
    std::env::remove_var("LOAD_PANIC_VAR");
    let result = std::panic::catch_unwind(|| {
        let _ = lockedenv::load! { LOAD_PANIC_VAR: u32 };
    });
    assert!(
        result.is_err(),
        "load! should panic when a required variable is absent"
    );
}

#[test]
fn load_panics_on_bad_parse() {
    std::env::set_var("LOAD_BAD_INT", "not_a_number");
    let result = std::panic::catch_unwind(|| {
        let _ = lockedenv::load! { LOAD_BAD_INT: u32 };
    });
    assert!(
        result.is_err(),
        "load! should panic when the value cannot be parsed"
    );
}

#[test]
fn load_option_absent() {
    std::env::remove_var("LOAD_OPT_ABSENT");
    let config = lockedenv::load! { LOAD_OPT_ABSENT: Option<String> };
    assert!(config.LOAD_OPT_ABSENT.is_none());
}

#[test]
fn load_option_present() {
    std::env::set_var("LOAD_OPT_PRESENT", "hello");
    let config = lockedenv::load! { LOAD_OPT_PRESENT: Option<String> };
    assert_eq!(config.LOAD_OPT_PRESENT, Some("hello".to_string()));
}

#[test]
fn load_string_with_spaces() {
    std::env::set_var("LOAD_SPACE_VAL", "hello world  ");
    let config = lockedenv::load! { LOAD_SPACE_VAL: String };
    assert_eq!(
        config.LOAD_SPACE_VAL, "hello world  ",
        "String values should not be trimmed"
    );
}

#[test]
fn load_default_overridden_by_env() {
    std::env::set_var("LOAD_WITH_DEF", "999");
    let config = lockedenv::load! { LOAD_WITH_DEF: u32 = 1 };
    assert_eq!(
        config.LOAD_WITH_DEF, 999,
        "env var should override the default"
    );
}

#[test]
fn load_default_used_when_absent() {
    std::env::remove_var("LOAD_USES_DEF");
    let config = lockedenv::load! { LOAD_USES_DEF: u32 = 42 };
    assert_eq!(
        config.LOAD_USES_DEF, 42,
        "default should be used when variable is absent"
    );
}

// ── try_check! / check! ────────────────────────────────────────────────────

#[test]
fn try_check_returns_ok_when_all_present() {
    std::env::set_var("CHK_HOST", "localhost");
    std::env::set_var("CHK_PORT", "9000");
    let result = lockedenv::try_check! { CHK_HOST: String, CHK_PORT: u16 };
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert_eq!(cfg.CHK_HOST, "localhost");
    assert_eq!(cfg.CHK_PORT, 9000u16);
}

#[test]
fn try_check_with_defaults_ok_when_absent() {
    std::env::remove_var("CHK_DEF_A");
    std::env::remove_var("CHK_DEF_B");
    let result = lockedenv::try_check! { CHK_DEF_A: u32 = 1, CHK_DEF_B: String = "x".to_string() };
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert_eq!(cfg.CHK_DEF_A, 1);
    assert_eq!(cfg.CHK_DEF_B, "x");
}

#[test]
fn try_check_collects_all_errors() {
    std::env::remove_var("CHK_MISS1");
    std::env::remove_var("CHK_MISS2");
    std::env::remove_var("CHK_MISS3");
    let errors = lockedenv::try_check! {
        CHK_MISS1: String,
        CHK_MISS2: u16,
        CHK_MISS3: bool,
    }
    .unwrap_err();
    assert_eq!(
        errors.len(),
        3,
        "must collect all 3 missing errors, got {}",
        errors.len()
    );
}

#[test]
fn try_check_collects_mixed_errors() {
    std::env::set_var("CHK_MIX_OK", "42");
    std::env::remove_var("CHK_MIX_MISS");
    std::env::set_var("CHK_MIX_BAD", "not-a-number");
    let errors = lockedenv::try_check! {
        CHK_MIX_OK:   u32,
        CHK_MIX_MISS: String,
        CHK_MIX_BAD:  u16,
    }
    .unwrap_err();
    assert_eq!(errors.len(), 2, "one ok, two errors; got {}", errors.len());
}

#[test]
fn try_check_option_absent_is_not_error() {
    std::env::remove_var("CHK_OPT_MISS");
    let result = lockedenv::try_check! { CHK_OPT_MISS: Option<String> };
    assert!(result.is_ok());
    assert!(result.unwrap().CHK_OPT_MISS.is_none());
}

#[test]
fn try_check_map_returns_ok() {
    let m: std::collections::HashMap<String, String> =
        [("CM_PORT".into(), "1234".into())].into_iter().collect();
    let cfg = lockedenv::try_check! { map: m, CM_PORT: u16 }.unwrap();
    assert_eq!(cfg.CM_PORT, 1234u16);
}

#[test]
fn try_check_map_collects_all_errors() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let errors = lockedenv::try_check! { map: m, A: String, B: u16 }.unwrap_err();
    assert_eq!(
        errors.len(),
        2,
        "both missing → 2 errors, got {}",
        errors.len()
    );
}

#[test]
fn check_panics_with_all_errors_in_message() {
    std::env::remove_var("CHKP_A");
    std::env::remove_var("CHKP_B");
    let result = std::panic::catch_unwind(|| {
        lockedenv::check! { CHKP_A: String, CHKP_B: u16 }
    });
    let err = result.unwrap_err();
    let msg = err
        .downcast_ref::<String>()
        .map(|s: &String| s.as_str())
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap_or("");
    // Both variable names must appear in the panic message
    assert!(msg.contains("CHKP_A"), "panic msg: {msg}");
    assert!(msg.contains("CHKP_B"), "panic msg: {msg}");
}

#[test]
fn check_map_panics_on_missing() {
    let m: std::collections::HashMap<String, String> = std::collections::HashMap::default();
    let result = std::panic::catch_unwind(|| {
        lockedenv::check! { map: m, CHKM_REQ: u32 }
    });
    assert!(
        result.is_err(),
        "check! with map must panic on missing field"
    );
}

#[test]
fn try_check_map_with_prefix() {
    let m: std::collections::HashMap<String, String> = [
        ("SVC_HOST".into(), "0.0.0.0".into()),
        ("SVC_PORT".into(), "80".into()),
    ]
    .into_iter()
    .collect();
    let cfg = lockedenv::try_check! { map: m, prefix = "SVC_", HOST: String, PORT: u16 }.unwrap();
    assert_eq!(cfg.HOST, "0.0.0.0");
    assert_eq!(cfg.PORT, 80u16);
}
