//! [`Error`] type definition for our app.

use std::env;
use std::ffi::OsString;
use std::num::ParseIntError;

use diesel_async::pooled_connection::deadpool::PoolError;

use crate::forward_from;

/// [`Result`](core::result::Result) type for our crate.
///
/// Uses our crate's [`Error`] type automatically.
pub type Result<T> = core::result::Result<T, Error>;

/// Error type used throughout this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error that occurred when loading data from an environment variable.
    #[error("error related to environment variable: {context}")]
    EnvVar {
        /// Environment variable error context.
        ///
        /// Used by the code (via [`EnvVarContext::with_env_var_context`]) to provide context for the error.
        context: String,

        /// Source of the environment error.
        source: EnvVarError,

        /// [`Backtrace`](std::backtrace::Backtrace) indicating where the error occurred.
        ///
        /// Will only contain useful information if backtrace is enabled (see
        /// [`Backtrace::capture`](std::backtrace::Backtrace::capture)).
        #[cfg(backtrace_support)]
        backtrace: std::backtrace::Backtrace,
    },

    /// Error caused by invalid user input.
    #[error("input parsing error")]
    Input {
        /// Source of the input error.
        #[from]
        source: actix_web_validator::error::Error,

        /// [`Backtrace`](std::backtrace::Backtrace) indicating where the error occurred.
        ///
        /// Will only contain useful information if backtrace is enabled (see
        /// [`Backtrace::capture`](std::backtrace::Backtrace::capture)).
        #[cfg(backtrace_support)]
        backtrace: std::backtrace::Backtrace,
    },

    /// Error related to the database connection pool.
    ///
    /// See [`PoolError`](deadpool::managed::PoolError) (and the inner [`diesel_async::pooled_connection::PoolError`])
    /// for more information.
    #[error("database connection error")]
    Pool {
        /// Source of the pool error.
        #[from]
        source: PoolError,

        /// [`Backtrace`](std::backtrace::Backtrace) indicating where the error occurred.
        ///
        /// Will only contain useful information if backtrace is enabled (see
        /// [`Backtrace::capture`](std::backtrace::Backtrace::capture)).
        #[cfg(backtrace_support)]
        backtrace: std::backtrace::Backtrace,
    },

    /// Error that occurred while performing a database query using [`diesel`].
    #[error("query error: {context}")]
    Query {
        /// Query error context.
        ///
        /// Used by the code (via [`QueryContext::with_query_context`]) to provide some context
        /// as to the type of query that caused the error.
        context: String,

        /// Source of the query error.
        source: diesel::result::Error,

        /// [`Backtrace`](std::backtrace::Backtrace) indicating where the error occurred.
        ///
        /// Will only contain useful information if backtrace is enabled (see
        /// [`Backtrace::capture`](std::backtrace::Backtrace::capture)).
        #[cfg(backtrace_support)]
        backtrace: std::backtrace::Backtrace,
    },
}

/// Error type used for errors related to environment variables.
///
/// This is our variant of [`VarError`], with additional variants for our specific use cases.
/// In particular, a [`From`] `impl` is provided to be able to convert a [`VarError`] to this type.
///
/// [`VarError`]: env::VarError
#[derive(Debug, thiserror::Error)]
pub enum EnvVarError {
    /// The environment variable did not exist.
    ///
    /// This is our equivalent for [`VarError::NotPresent`](env::VarError::NotPresent).
    #[error("variable not found in environment")]
    NotFound,

    /// The environment variable could not be parsed to a Rust string because it contains
    /// invalid Unicode characters.
    ///
    /// This is our equivalent for [`VarError::NotUnicode`](env::VarError::NotUnicode).
    #[error("variable contained invalid, non-Unicode characters")]
    NotUnicode(OsString),

    /// The environment variable was expected to contain an int value, but didn't.
    #[error("expected int value, found {value}")]
    IntExpected {
        /// The actual value found in the environment variable.
        value: String,

        /// The parsing error that occurred when we tried to parse the value as an int.
        source: ParseIntError,
    },
}

impl From<env::VarError> for EnvVarError {
    /// Converts an `std` [`VarError`] to our intermediate [`EnvVarError`] type.
    ///
    /// Each variant of [`VarError`] has a corresponding variant in our [`EnvVarError`] type,
    /// so the mapping is straightforward.
    ///
    /// [`VarError`]: env::VarError
    fn from(value: env::VarError) -> Self {
        match value {
            env::VarError::NotPresent => Self::NotFound,
            env::VarError::NotUnicode(os_str) => Self::NotUnicode(os_str),
        }
    }
}

/// Helper trait to provide context for [`EnvVar`](Error::EnvVar) errors.
pub trait EnvVarContext {
    /// Type of output returned by [`with_env_var_context`](EnvVarContext::with_env_var_context).
    type Output;

    /// Provides context about the error that occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::env;
    ///
    /// use pokedex::error::EnvVarContext;
    ///
    /// # fn example() -> pokedex::Result<()> {
    /// let db_url = env::var("DATABASE_URL")
    ///     .with_env_var_context(|| "DATABASE_URL environment variable should be set")?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    fn with_env_var_context<C, F>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<E> EnvVarContext for E
where
    E: Into<EnvVarError>,
{
    type Output = Error;

    fn with_env_var_context<C, F>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        Error::EnvVar {
            context: (context)().into(),
            source: self.into(),
            #[cfg(backtrace_support)]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

impl<T, E> EnvVarContext for core::result::Result<T, E>
where
    E: EnvVarContext<Output = Error>,
{
    type Output = Result<T>;

    fn with_env_var_context<C, F>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|err| err.with_env_var_context(context))
    }
}

forward_from!(diesel_async::pooled_connection::PoolError => PoolError => Error);

impl<E> From<deadpool::managed::BuildError<E>> for Error
where
    E: Into<Error>,
{
    /// Converts a [`BuildError`](deadpool::managed::BuildError) into our [`Error`] type.
    ///
    /// This makes it possible to use `?` when building a connection pool.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    /// use pokedex::db::{get_db_url, Pool};
    ///
    /// fn get_pool() -> pokedex::Result<Pool> {
    ///     let manager = AsyncDieselConnectionManager::new(get_db_url()?);
    ///     Ok(Pool::builder(manager).build()?)
    /// }
    /// ```
    fn from(value: deadpool::managed::BuildError<E>) -> Self {
        match value {
            deadpool::managed::BuildError::Backend(err) => err.into(),
            deadpool::managed::BuildError::NoRuntimeSpecified(msg) => {
                panic!("Runtime should be specified in Cargo.toml: {}", msg);
            },
        }
    }
}

/// Helper trait to provide context for [`Query`](Error::Query) errors.
pub trait QueryContext {
    /// Type of output returned by [`with_query_context`](QueryContext::with_query_context).
    type Output;

    /// Provides context about the query performed when the error occurred.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use diesel::QueryDsl;
    /// use diesel_async::RunQueryDsl;
    /// use pokedex::error::QueryContext;
    /// # use pokedex::db::{get_pool, PooledConnection};
    /// use pokedex::models::pokemon::Pokemon;
    /// use pokedex::schema::pokemons::dsl::*;
    ///
    /// # async fn example(pokemon_id: i64) -> pokedex::Result<()> {
    /// # let pool = get_pool()?;
    /// # let mut connection = pool.get().await?;
    /// #
    /// let pokemon: Pokemon = pokemons
    ///     .find(pokemon_id)
    ///     .first(&mut connection)
    ///     .await
    ///     .with_query_context(|| format!("Failed to fetch pokemon with id {}", pokemon_id))?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    fn with_query_context<C, F>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl QueryContext for diesel::result::Error {
    type Output = Error;

    fn with_query_context<C, F>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        Error::Query {
            context: (context)().into(),
            source: self,
            #[cfg(backtrace_support)]
            backtrace: std::backtrace::Backtrace::capture(),
        }
    }
}

impl<T, E> QueryContext for core::result::Result<T, E>
where
    E: QueryContext<Output = Error>,
{
    type Output = Result<T>;

    fn with_query_context<C, F>(self, context: F) -> Self::Output
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.map_err(|err| err.with_query_context(context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod from_var_error_for_env_var_error {
        use assert_matches::assert_matches;
        use serial_test::serial;

        use super::*;
        use crate::helpers::tests::get_invalid_os_string;

        #[test]
        #[serial(result_env_var_tests)]
        fn test_not_present() {
            env::remove_var("POKEDEX_RESULT_ENV_VAR_TEST");

            let var_error = env::var("POKEDEX_RESULT_ENV_VAR_TEST").unwrap_err();
            let env_var_error: EnvVarError = var_error.into();
            assert_matches!(env_var_error, EnvVarError::NotFound);
        }

        #[test]
        #[serial(result_env_var_tests)]
        fn test_not_unicode() {
            let invalid_os_string = get_invalid_os_string();
            env::set_var("POKEDEX_RESULT_ENV_VAR_TEST", &invalid_os_string);

            let var_error = env::var("POKEDEX_RESULT_ENV_VAR_TEST").unwrap_err();
            let env_var_error: EnvVarError = var_error.into();
            assert_matches!(env_var_error, EnvVarError::NotUnicode(value) if value == invalid_os_string);
        }
    }

    mod env_var_context {
        use super::*;

        mod for_e_where_e_into_error {
            use assert_matches::assert_matches;
            use serial_test::serial;

            use super::*;

            #[test]
            #[serial(result_env_var_tests)]
            fn test_all() {
                env::remove_var("POKEDEX_RESULT_ENV_VAR_TEST");

                let var_error = env::var("POKEDEX_RESULT_ENV_VAR_TEST").unwrap_err();
                let error: Error = var_error.with_env_var_context(|| "context");
                assert_matches!(error, Error::EnvVar { context, source: env_var_error, .. } => {
                    assert_eq!("context", context);
                    assert_matches!(env_var_error, EnvVarError::NotFound);
                });
            }
        }

        mod for_result_t_e_where_e_env_var_context {
            use assert_matches::assert_matches;
            use serial_test::serial;

            use super::*;

            #[test]
            #[serial(result_env_var_tests)]
            fn test_all() {
                env::remove_var("POKEDEX_RESULT_ENV_VAR_TEST");

                let result = env::var("POKEDEX_RESULT_ENV_VAR_TEST");
                let result = result.with_env_var_context(|| "context");
                assert_matches!(result, Err(Error::EnvVar { context, source: env_var_error, .. }) => {
                    assert_eq!("context", context);
                    assert_matches!(env_var_error, EnvVarError::NotFound);
                });
            }
        }
    }

    mod from_deadpool_managed_build_error_for_error {
        use assert_matches::assert_matches;

        use super::*;

        #[test]
        fn test_backend() {
            let backend_error = diesel_async::pooled_connection::PoolError::QueryError(
                diesel::result::Error::BrokenTransactionManager,
            );
            let build_error = deadpool::managed::BuildError::Backend(backend_error);
            let error: Error = build_error.into();
            assert_matches!(error, Error::Pool { source: pool_error, .. } => {
                assert_matches!(pool_error, PoolError::Backend(diesel_async::pooled_connection::PoolError::QueryError(diesel::result::Error::BrokenTransactionManager)));
            });
        }

        #[test]
        #[should_panic]
        fn test_no_runtime_specified() {
            let build_error = deadpool::managed::BuildError::<
                diesel_async::pooled_connection::PoolError,
            >::NoRuntimeSpecified("no runtime specified".to_string());
            let _ = Into::<Error>::into(build_error);
        }
    }

    mod query_context {
        use super::*;

        mod for_diesel_result_error {
            use assert_matches::assert_matches;

            use super::*;

            #[test]
            fn test_all() {
                let diesel_error = diesel::result::Error::NotFound;
                let error = diesel_error.with_query_context(|| "context");
                assert_matches!(error, Error::Query { context, source: query_error, .. } => {
                    assert_eq!("context", context);
                    assert_matches!(query_error, diesel::result::Error::NotFound);
                });
            }
        }

        mod for_result_t_e_where_e_query_context {
            use assert_matches::assert_matches;

            use super::*;

            fn try_something() -> core::result::Result<(), diesel::result::Error> {
                Err(diesel::result::Error::NotFound)
            }

            #[test]
            fn test_all() {
                let result = try_something();
                let result = result.with_query_context(|| "context");
                assert_matches!(result, Err(Error::Query { context, source: query_error, .. }) => {
                    assert_eq!("context", context);
                    assert_matches!(query_error, diesel::result::Error::NotFound);
                });
            }
        }
    }
}
