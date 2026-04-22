use std::fmt;

/// Error produced when loading or parsing environment variables.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum EnvLockError {
    /// A required variable was absent from the environment (or the map).
    #[non_exhaustive]
    Missing {
        variable: String,
        /// Optional hint attached via [`EnvLockError::with_hint`].
        hint: Option<String>,
    },
    /// A variable was present but its value could not be parsed into the target type.
    #[non_exhaustive]
    Parse {
        variable: String,
        found: String,
        /// Description of what was expected (the `Display` output of the parse error).
        expected: String,
        /// Optional extra hint added via [`EnvLockError::with_hint`].
        hint: Option<String>,
    },
    /// A `.env` file could not be loaded (feature `dotenv`).
    #[non_exhaustive]
    Dotenv {
        path: String,
        cause: String,
        /// Optional hint attached via [`EnvLockError::with_hint`].
        hint: Option<String>,
    },
}

impl EnvLockError {
    /// Create a `Missing` error for the given variable name.
    #[must_use]
    pub fn missing(variable: String) -> Self {
        EnvLockError::Missing {
            variable,
            hint: None,
        }
    }

    /// Create a `Parse` error, converting the parse failure to a string automatically.
    pub fn parse_error<E: fmt::Display>(variable: String, found: String, err: E) -> Self {
        EnvLockError::Parse {
            variable,
            found,
            expected: err.to_string(),
            hint: None,
        }
    }

    /// Create a `Dotenv` error (feature `dotenv`).
    #[must_use]
    pub fn dotenv(path: String, cause: String) -> Self {
        EnvLockError::Dotenv {
            path,
            cause,
            hint: None,
        }
    }

    /// Attach a hint to any error variant.
    /// The hint is shown in `Display` output after the primary message.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        match &mut self {
            EnvLockError::Missing { hint: h, .. }
            | EnvLockError::Parse { hint: h, .. }
            | EnvLockError::Dotenv { hint: h, .. } => {
                *h = Some(hint.into());
            }
        }
        self
    }
}

impl fmt::Display for EnvLockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvLockError::Missing { variable, hint } => {
                write!(f, "EnvLockError: missing required variable\n  variable: {variable}\n  hint: add {variable} to your environment or .env file")?;
                if let Some(h) = hint {
                    write!(f, "\n  hint: {h}")?;
                }
                Ok(())
            }
            EnvLockError::Parse {
                variable,
                found,
                expected,
                hint,
            } => {
                write!(f, "EnvLockError: failed to parse environment variable\n  variable: {variable}\n  found: \"{found}\"\n  expected type: {expected}")?;
                if let Some(h) = hint {
                    write!(f, "\n  hint: {h}")?;
                }
                Ok(())
            }
            EnvLockError::Dotenv { path, cause, hint } => {
                write!(
                    f,
                    "EnvLockError: failed to load .env file\n  path: {path}\n  cause: {cause}"
                )?;
                if let Some(h) = hint {
                    write!(f, "\n  hint: {h}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for EnvLockError {}
