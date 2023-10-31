//! Helpers pertaining to interacting with environment variables.

use std::env;
use std::num::ParseIntError;
use std::str::FromStr;

use dotenvy::dotenv;

use crate::error::EnvVarError;

/// Optionally loads `.env` file via [`dotenv`], skipping if not found.
///
/// # Return values
///
/// | `.env` file                | Return value |
/// |----------------------------|--------------|
/// | Exists, loads successfully | `Ok(true)`   |
/// | Does not exist             | `Ok(false)`  |
/// | Exists but fails to load   | `Err(_)`     |
pub fn load_optional_dotenv() -> dotenvy::Result<bool> {
    dotenv().map(|_| true).or_else(|err| match err {
        err if err.not_found() => Ok(false),
        err => Err(err),
    })
}

/// Reads the value of an environment variable, as an int value.
///
/// # Possible return values
///
/// | Environment variable     | Return value                    |
/// |--------------------------|---------------------------------|
/// | Contains value `42`      | `Ok(42)`                        |
/// | Does not exist           | `Err(EnvVarError::NotFound)`    |
/// | Contains invalid unicode | `Err(EnvVarError::NotUnicode)`  |
/// | Contains value `foo`     | `Err(EnvVarError::IntExpected)` |
pub fn int_env_var<T>(key: &str) -> Result<T, EnvVarError>
where
    T: FromStr<Err = ParseIntError>,
{
    env::var(key).map_err(Into::into).and_then(|value| {
        value
            .parse::<T>()
            .map_err(|parse_err| EnvVarError::IntExpected { value, source: parse_err })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod int_env_var {
        use std::num::IntErrorKind;

        use assert_matches::assert_matches;
        use serial_test::serial;

        use super::*;
        use crate::helpers::tests::get_invalid_os_string;

        #[test]
        #[serial(int_env_var_tests)]
        fn test_without_env_var() {
            env::remove_var("POKEDEX_TEST_INT_ENV_VAR");

            assert_matches!(
                int_env_var::<i32>("POKEDEX_TEST_INT_ENV_VAR"),
                Err(EnvVarError::NotFound)
            );
        }

        #[test]
        #[serial(int_env_var_tests)]
        fn test_with_int_value() {
            env::set_var("POKEDEX_TEST_INT_ENV_VAR", "42");

            assert_matches!(int_env_var::<i32>("POKEDEX_TEST_INT_ENV_VAR"), Ok(42));
        }

        #[test]
        #[serial(int_env_var_tests)]
        fn test_with_invalid_unicode() {
            env::set_var("POKEDEX_TEST_INT_ENV_VAR", get_invalid_os_string());

            assert_matches!(
                int_env_var::<i32>("POKEDEX_TEST_INT_ENV_VAR"),
                Err(EnvVarError::NotUnicode(_))
            );
        }

        #[test]
        #[serial(int_env_var_tests)]
        fn test_with_invalid_int_value() {
            env::set_var("POKEDEX_TEST_INT_ENV_VAR", "life");

            assert_matches!(int_env_var::<i32>("POKEDEX_TEST_INT_ENV_VAR"), Err(EnvVarError::IntExpected { value, source }) => {
                assert_eq!("life", value);
                assert_eq!(IntErrorKind::InvalidDigit, *source.kind());
            });
        }
    }
}
