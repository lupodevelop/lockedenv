//! Support for loading a `.env` file before reading environment variables.
//! Enabled with feature `dotenv`.

use crate::EnvLockError;

/// Load a `.env` file using `dotenvy`, silently ignoring non‑existent files.
/// Errors are reported as `EnvLockError::Dotenv`.
#[allow(clippy::missing_errors_doc)]
pub fn load_file(path: impl AsRef<std::path::Path>) -> Result<(), EnvLockError> {
    let p = path.as_ref();
    match dotenvy::from_filename(p) {
        Ok(_) => Ok(()),
        Err(dotenvy::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(EnvLockError::dotenv(p.display().to_string(), e.to_string())),
    }
}
