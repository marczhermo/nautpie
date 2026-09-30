//! Environment-variable helpers.
//!
//! - [`load_dotenv`] mirrors PHP `vlucas/phpdotenv::safeLoad` — load `.env` from
//!   the current working directory, silently ignoring missing files.
//! - [`check_envs`] returns the values of every requested variable or
//!   [`Error::MissingEnv`] if any are unset.
//! - [`is_production`] case-insensitively matches `"prod"` / `"production"`.

use crate::error::Error;

/// Load `.env` from the current directory. Missing files are ignored, matching
/// PHP `safeLoad`. Returns `true` if a `.env` was loaded, `false` otherwise.
pub fn load_dotenv() -> bool {
    dotenvy::dotenv().is_ok()
}

/// Read each named environment variable, returning their values in order. If
/// any variable is unset or empty, returns [`Error::MissingEnv`].
pub fn check_envs(names: &[&str]) -> Result<Vec<String>, Error> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let value = std::env::var(name).map_err(|_| Error::MissingEnv((*name).into()))?;
        if value.is_empty() {
            return Err(Error::MissingEnv((*name).into()));
        }
        out.push(value);
    }
    Ok(out)
}

/// Case-insensitive check against the production environment names.
pub fn is_production(env: &str) -> bool {
    matches!(env.to_ascii_lowercase().as_str(), "prod" | "production")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_env_is_error() {
        let key = "NAUTPIE_TEST_DEFINITELY_NOT_SET_X9K";
        std::env::remove_var(key);
        let err = check_envs(&[key]).unwrap_err();
        assert!(matches!(err, Error::MissingEnv(ref n) if n == key));
    }

    #[test]
    fn present_env_returns_value() {
        let key = "NAUTPIE_TEST_PRESENT_KEY";
        std::env::set_var(key, "abc123");
        let v = check_envs(&[key]).unwrap();
        assert_eq!(v, vec!["abc123".to_string()]);
        std::env::remove_var(key);
    }

    #[test]
    fn empty_env_is_missing() {
        let key = "NAUTPIE_TEST_EMPTY_KEY";
        std::env::set_var(key, "");
        assert!(check_envs(&[key]).is_err());
        std::env::remove_var(key);
    }

    #[test]
    fn is_production_matches() {
        assert!(is_production("prod"));
        assert!(is_production("PROD"));
        assert!(is_production("Production"));
        assert!(!is_production("uat"));
        assert!(!is_production("test"));
    }
}
