//! Helpers to connect to the Pokedex database. Currently supports Postgres as backend only.

use std::env;

use diesel::pg::Pg;
use diesel::PgConnection;
use diesel_async::pooled_connection::deadpool::Object;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;

use crate::error::{EnvVarContext, EnvVarError};
use crate::helpers::env::int_env_var;

/// Type of DB backend supported by our crate; current Postgres.
pub type Backend = Pg;

/// A synchronous connection to the Pokedex database.
///
/// This is not used in the REST API implementations because they are `async`, but is used by the
/// bin crates to perform initial DB seeding / applying migrations.
pub type SyncConnection = PgConnection;

/// An asynchronous connection to the Pokedex database.
///
/// This is provided by the [`diesel_async`] crate.
pub type Connection = AsyncPgConnection;

/// A pool of [`Connection`]s to the database.
///
/// This can be shared among worker threads in the web application to reuse database connections
/// efficiently. The pooling is handled by the [`deadpool`] crate.
pub type Pool = diesel_async::pooled_connection::deadpool::Pool<Connection>;

/// A [`Connection`] stored in the connection [`Pool`].
///
/// This type is what is actually returned when [`Pool::get`] is called; its [`Deref`](std::ops::Deref)
/// implementation then allows it to be used as a standard asynchronous connection in calls like
/// [`execute`](diesel_async::RunQueryDsl::execute) through [`Deref` coercion](std::ops::Deref#more-on-deref-coercion).
pub type PooledConnection = Object<Connection>;

/// Returns the Pokedex database connection URL.
///
/// The URL should be specified through the `DATABASE_URL` environment variable.
pub fn get_db_url() -> crate::Result<String> {
    env::var("DATABASE_URL")
        .with_env_var_context(|| "DATABASE_URL environment variable must be set")
}

/// Returns the maximum number of connections to store in the database connection [`Pool`].
///
/// This can be specified through the `MAX_POOL_SIZE` environment variable, but is optional.
/// If not specified, the default value depends on the number of physical CPUs on the machine
/// (see [`PoolConfig::default`](deadpool::managed::PoolConfig::default)).
pub fn get_max_pool_size() -> crate::Result<Option<usize>> {
    match int_env_var("MAX_POOL_SIZE") {
        Ok(value) => Ok(Some(value)),
        Err(EnvVarError::NotFound) => Ok(None),
        Err(err @ EnvVarError::NotUnicode(_) | err @ EnvVarError::IntExpected { .. }) => {
            Err(err.with_env_var_context(|| "failed to parse environment variable MAX_POOL_SIZE"))
        },
    }
}

/// Creates and returns a Pokedex database connection [`Pool`].
///
/// The pool can be used to fetch database connections in worker threads in a safe way; when the
/// connection is no longer needed, it is recycled and returned to the pool to be reused later.
/// This is all implemented by the [`deadpool`] crate.
pub fn get_pool() -> crate::Result<Pool> {
    let manager = AsyncDieselConnectionManager::new(get_db_url()?);
    let mut pool_builder = Pool::builder(manager);

    if let Some(max_size) = get_max_pool_size()? {
        pool_builder = pool_builder.max_size(max_size);
    }

    Ok(pool_builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_db_url {
        use assert_matches::assert_matches;
        use serial_test::file_serial;

        use super::*;
        use crate::helpers::tests::get_invalid_os_string;
        use crate::Error;

        #[test]
        #[file_serial(db_url_env)]
        fn test_with_env_var() {
            env::set_var("DATABASE_URL", "some_url");

            assert_matches!(get_db_url(), Ok(url) if url == "some_url");
        }

        #[test]
        #[file_serial(db_url_env)]
        fn test_without_env_var() {
            env::remove_var("DATABASE_URL");

            assert_matches!(get_db_url(), Err(Error::EnvVar { source, .. }) => {
                assert_matches!(source, EnvVarError::NotFound);
            });
        }

        #[test]
        #[file_serial(db_url_env)]
        fn test_with_invalid_unicode() {
            env::set_var("DATABASE_URL", get_invalid_os_string());

            assert_matches!(get_db_url(), Err(Error::EnvVar { source, .. }) => {
                assert_matches!(source, EnvVarError::NotUnicode(_));
            })
        }
    }

    mod get_max_pool_size {
        use std::num::IntErrorKind;

        use assert_matches::assert_matches;
        use serial_test::file_serial;

        use super::*;
        use crate::helpers::tests::get_invalid_os_string;
        use crate::Error;

        #[test]
        #[file_serial(max_pool_size_env)]
        fn test_without_env_var() {
            env::remove_var("MAX_POOL_SIZE");

            assert_matches!(get_max_pool_size(), Ok(None));
        }

        #[test]
        #[file_serial(max_pool_size_env)]
        fn test_with_int_value() {
            env::set_var("MAX_POOL_SIZE", "42");

            assert_matches!(get_max_pool_size(), Ok(Some(42)));
        }

        #[test]
        #[file_serial(max_pool_size_env)]
        fn test_with_invalid_unicode() {
            env::set_var("MAX_POOL_SIZE", get_invalid_os_string());

            assert_matches!(get_max_pool_size(), Err(Error::EnvVar { source, .. }) => {
                assert_matches!(source, EnvVarError::NotUnicode(_));
            });
        }

        #[test]
        #[file_serial(max_pool_size_env)]
        fn test_with_invalid_int_value() {
            env::set_var("MAX_POOL_SIZE", "life");

            assert_matches!(get_max_pool_size(), Err(Error::EnvVar { source: env_var_err, .. }) => {
                assert_matches!(env_var_err, EnvVarError::IntExpected { value, source: parse_err } => {
                    assert_eq!("life", value);
                    assert_eq!(IntErrorKind::InvalidDigit, *parse_err.kind());
                });
            });
        }
    }
}
