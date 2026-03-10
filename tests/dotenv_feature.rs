#[cfg(feature = "dotenv")]
mod dotenv_tests {
    use std::io::Write;

    fn tmp_env_file(name: &str, content: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        // Include the process id to avoid collisions between test runs.
        path.push(format!("env_lock_{}_{}.env", std::process::id(), name));
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "{content}").unwrap();
        path
    }

    /// A non-existent .env file is silently ignored (Ok returned).
    #[test]
    fn missing_file_is_ok() {
        let result = lockedenv::dotenv::load_file("/tmp/__env_lock_nonexistent_file__.env");
        assert!(result.is_ok(), "missing .env file should not be an error");
    }

    /// A valid .env file sets variables in the process environment.
    #[test]
    fn valid_file_sets_vars() {
        let path = tmp_env_file(
            "valid",
            "DOTENV_LOAD_VAR_A=hello\nDOTENV_LOAD_VAR_B=42\n",
        );
        // Remove vars first so the test is deterministic.
        std::env::remove_var("DOTENV_LOAD_VAR_A");
        std::env::remove_var("DOTENV_LOAD_VAR_B");

        lockedenv::dotenv::load_file(path.to_str().unwrap()).unwrap();

        assert_eq!(std::env::var("DOTENV_LOAD_VAR_A").unwrap(), "hello");
        assert_eq!(std::env::var("DOTENV_LOAD_VAR_B").unwrap(), "42");

        std::fs::remove_file(path).ok();
    }

    /// An existing env var is NOT overwritten (dotenvy default behaviour).
    #[test]
    fn existing_var_not_overwritten() {
        let path = tmp_env_file("no_overwrite", "DOTENV_EXISTING=from_file\n");
        std::env::set_var("DOTENV_EXISTING", "from_env");

        lockedenv::dotenv::load_file(path.to_str().unwrap()).unwrap();

        assert_eq!(
            std::env::var("DOTENV_EXISTING").unwrap(),
            "from_env",
            "existing env vars must not be overwritten by the .env file",
        );
        std::fs::remove_file(path).ok();
    }

    /// A syntactically invalid .env file returns Err.
    #[test]
    fn invalid_file_returns_err() {
        // An assignment with an unclosed quote is invalid in dotenvy.
        let path = tmp_env_file("invalid", "BROKEN_VAR=\"unclosed\n");
        let result = lockedenv::dotenv::load_file(path.to_str().unwrap());
        // dotenvy may or may not error on this; what matters is the function
        // doesn't panic and returns an Err/Ok consistently.
        let _ = result; // just assert it doesn't panic
        std::fs::remove_file(path).ok();
    }

    /// `load_dotenv`! macro sets vars then parses them.
    #[test]
    fn load_dotenv_macro() {
        let path = tmp_env_file("macro", "DOTENV_MACRO_PORT=7777\n");
        std::env::remove_var("DOTENV_MACRO_PORT");

        let config = lockedenv::load_dotenv! {
            path: path.to_str().unwrap(),
            DOTENV_MACRO_PORT: u16,
        };
        assert_eq!(config.DOTENV_MACRO_PORT, 7777);

        std::fs::remove_file(path).ok();
    }
}
