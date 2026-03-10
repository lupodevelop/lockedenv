#![allow(clippy::uninlined_format_args)]
// Tests for load! / try_load! macros (which interact with std::env).
// Each test uses unique variable names to avoid cross-test interference.

// ── load! / try_load! ──────────────────────────────────────────────────────

#[test]
fn load_basic_types() {
    std::env::set_var("BASIC_PORT", "8080");
    std::env::set_var("BASIC_DB_URL", "postgres://localhost/test");

    let config = env_lock::load! {
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
    let cfg = env_lock::load! { CLDBG_PORT: u16 };
    let cloned = cfg.clone();
    assert_eq!(cfg.CLDBG_PORT, cloned.CLDBG_PORT);
    let dbg = format!("{cfg:?}");
    assert!(dbg.contains("99"), "Debug output should contain field value");
}

#[test]
fn try_load_returns_ok() -> Result<(), env_lock::EnvLockError> {
    std::env::set_var("TRYLOAD_PORT", "1234");
    let config = env_lock::try_load! { TRYLOAD_PORT: u16 }?;
    assert_eq!(config.TRYLOAD_PORT, 1234);
    Ok(())
}

#[test]
fn try_load_returns_err_on_missing() {
    std::env::remove_var("TRYLOAD_ABSENT");
    let result = env_lock::try_load! { TRYLOAD_ABSENT: u32 };
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("TRYLOAD_ABSENT"), "error message should name the missing variable");
}

#[test]
fn load_panics_on_missing() {
    std::env::remove_var("LOAD_PANIC_VAR");
    let result = std::panic::catch_unwind(|| {
        let _ = env_lock::load! { LOAD_PANIC_VAR: u32 };
    });
    assert!(result.is_err(), "load! should panic when a required variable is absent");
}

#[test]
fn load_panics_on_bad_parse() {
    std::env::set_var("LOAD_BAD_INT", "not_a_number");
    let result = std::panic::catch_unwind(|| {
        let _ = env_lock::load! { LOAD_BAD_INT: u32 };
    });
    assert!(result.is_err(), "load! should panic when the value cannot be parsed");
}

#[test]
fn load_option_absent() {
    std::env::remove_var("LOAD_OPT_ABSENT");
    let config = env_lock::load! { LOAD_OPT_ABSENT: Option<String> };
    assert!(config.LOAD_OPT_ABSENT.is_none());
}

#[test]
fn load_option_present() {
    std::env::set_var("LOAD_OPT_PRESENT", "hello");
    let config = env_lock::load! { LOAD_OPT_PRESENT: Option<String> };
    assert_eq!(config.LOAD_OPT_PRESENT, Some("hello".to_string()));
}

#[test]
fn load_string_with_spaces() {
    std::env::set_var("LOAD_SPACE_VAL", "hello world  ");
    let config = env_lock::load! { LOAD_SPACE_VAL: String };
    assert_eq!(config.LOAD_SPACE_VAL, "hello world  ", "String values should not be trimmed");
}

#[test]
fn load_default_overridden_by_env() {
    std::env::set_var("LOAD_WITH_DEF", "999");
    let config = env_lock::load! { LOAD_WITH_DEF: u32 = 1 };
    assert_eq!(config.LOAD_WITH_DEF, 999, "env var should override the default");
}

#[test]
fn load_default_used_when_absent() {
    std::env::remove_var("LOAD_USES_DEF");
    let config = env_lock::load! { LOAD_USES_DEF: u32 = 42 };
    assert_eq!(config.LOAD_USES_DEF, 42, "default should be used when variable is absent");
}
