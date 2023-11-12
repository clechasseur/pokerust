//! Types and functions to implement proper error handling in the Pokedex API.

use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use diesel::result::DatabaseErrorKind;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TryFromInto};
use utoipa::{ToResponse, ToSchema};

use crate::helpers::error::recursive_error_message;
use crate::service_env::ServiceEnv;
use crate::Error;

impl ResponseError for Error {
    /// Returns the [`StatusCode`] to use for this [`Error`].
    ///
    /// This function does the actual mapping between our internal errors and the resulting
    /// external HTTP [`StatusCode`].
    fn status_code(&self) -> StatusCode {
        let status_code = match self {
            Error::Input { .. } => Some(StatusCode::BAD_REQUEST),
            Error::Query { source, .. } => status_code_for_query_error(source),
            _ => None,
        };

        status_code.unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    /// Returns an appropriate [`HttpResponse`] to return when a REST API error occurs.
    ///
    /// Uses the context of this [`Error`] to craft the response (see [`ErrorResponse::from`]).
    fn error_response(&self) -> HttpResponse<BoxBody> {
        let error_response: ErrorResponse = self.into();
        HttpResponse::build(error_response.status_code).json(error_response)
    }
}

/// Helper function to get a [`StatusCode`] for a [query error](diesel::result::Error).
///
/// If the error is due to faulty user input (like [`NotFound`](diesel::result::Error::NotFound)),
/// this method will return `Some` with an appropriate HTTP status code (like [`NOT_FOUND`](StatusCode::NOT_FOUND)).
/// Otherwise, it will return `None` and the caller can decide what status code to use.
pub fn status_code_for_query_error(error: &diesel::result::Error) -> Option<StatusCode> {
    match error {
        diesel::result::Error::NotFound => Some(StatusCode::NOT_FOUND),
        diesel::result::Error::DatabaseError(kind, ..) => match kind {
            DatabaseErrorKind::UniqueViolation | DatabaseErrorKind::CheckViolation => {
                Some(StatusCode::BAD_REQUEST)
            },
            _ => None,
        },
        _ => None,
    }
}

#[cfg_attr(
    doc,
    doc = r"
        Struct used to return error information as JSON in [`HttpResponse`]s.

        # Notes

        The [`internal_error`](ErrorResponse::internal_error) field will only
        be populated when running in a [`Development`] environment (see [`ErrorResponse::from`]).

        [`Development`]: ServiceEnv::Development
    "
)]
#[cfg_attr(not(doc), doc = "Pokedex API error information")]
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, ToResponse)]
#[response(
    description = "Server error",
    example = json!({
        "status_code": 500,
        "error": "Internal Server Error"
    }),
)]
pub struct ErrorResponse {
    /// HTTP status code
    #[serde_as(as = "TryFromInto<u16>")]
    #[schema(
        value_type = u16,
        minimum = 100,
        maximum = 999,
    )]
    pub status_code: StatusCode,

    /// Error message
    pub error: String,

    /// More details, when appropriate (like for deserialization or validation errors)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    #[cfg_attr(
        doc,
        doc = r"
            Description of the internal error

            Only present when server is running in a [`Development`] environment.

            [`Development`]: ServiceEnv::Development
        "
    )]
    #[cfg_attr(
        not(doc),
        doc = "Description of the internal error (when server is running in development)"
    )]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_error: Option<String>,
}

impl From<&Error> for ErrorResponse {
    /// Creates an [`ErrorResponse`] for an internal [`Error`].
    ///
    /// This will be used to create the body of the [`HttpResponse`] returned when an error occurs
    /// during a REST API call.
    ///
    /// # Service environment
    ///
    /// Unless we run in a [`Development`] environment, this function will not include any actual
    /// information about the cause of the internal error, to maintain security. In [`Development`],
    /// the [`internal_error`](ErrorResponse#structfield.internal_error) field will include more information.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::env;
    ///
    /// use actix_web::http::StatusCode;
    /// use actix_web::{HttpResponse, ResponseError};
    /// use pokedex_rs::api::errors::ErrorResponse;
    /// use pokedex_rs::error::EnvVarContext;
    /// use pokedex_rs::Error;
    ///
    /// let error = env::var("NONEXISTENT_POKEDEX_ENV_VAR")
    ///     .unwrap_err()
    ///     .with_env_var_context(|| "NONEXISTENT_POKEDEX_ENV_VAR should be set");
    ///
    /// let error_response: ErrorResponse = (&error).into();
    /// let http_response = HttpResponse::build(error_response.status_code).json(error_response);
    ///
    /// assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, http_response.status());
    /// ```
    ///
    /// [`Development`]: ServiceEnv::Development
    fn from(value: &Error) -> Self {
        let status_code = value.status_code();

        Self {
            status_code,
            error: status_code
                .canonical_reason()
                .unwrap_or("Unknown Error")
                .into(),
            details: Self::generate_details(value),
            internal_error: Self::generate_internal_error(value),
        }
    }
}

impl ErrorResponse {
    /// Returns the value to use for the [`details`](ErrorResponse::details) field.
    ///
    /// This will return a value for some types of errors, like deserialization or validation
    /// errors, so that user can have more information.
    fn generate_details(error: &Error) -> Option<String> {
        match error {
            Error::Input { source, .. } => Some(format!("{}", source)),
            _ => None,
        }
    }

    /// Returns the value to use for the [`internal_error`](ErrorResponse::internal_error) field.
    ///
    /// This will return `None` except when running in [`Development`](ServiceEnv::Development)
    /// environment (see [`from`](ErrorResponse::from)).
    fn generate_internal_error(error: &Error) -> Option<String> {
        if ServiceEnv::current().is_development() {
            Some(recursive_error_message(error))
        } else {
            None
        }
    }
}

/// Generic error handler for `actix_web`'s various configs.
///
/// This handler accepts any type of error that can be turned into our [`Error`] type, then turns
/// that into an [`actix_web::error::Error`]. This is possible because we implemented [`ResponseError`]
/// for our [`Error`] type - thus, `actix_web` will use our implementation to generate an appropriate
/// HTTP response for the type of error encountered. This makes it possible to handle pre-request handler
/// errors (like for example [`DeserializeErrors`]s) using the same error handling code as in-request
/// handler errors.
///
/// # Examples
///
/// ```no_run
/// use actix_web_validator::{JsonConfig, PathConfig};
/// use pokedex_rs::api::errors::actix_error_handler;
///
/// let json_config = JsonConfig::default().error_handler(actix_error_handler);
/// let path_config = PathConfig::default().error_handler(actix_error_handler);
/// ```
///
/// [`DeserializeErrors`]: actix_web_validator::error::DeserializeErrors
pub fn actix_error_handler<E, R>(err: E, _req: &R) -> actix_web::error::Error
where
    E: Into<Error>,
{
    Into::<Error>::into(err).into()
}

#[cfg(test)]
mod tests {
    use actix_web::body::MessageBody;
    use actix_web::http::header;
    use actix_web::http::header::HeaderValue;
    use assert_matches::assert_matches;
    use serde::de::DeserializeOwned;

    use super::*;

    fn http_response_json_content<T>(http_response: HttpResponse) -> T
    where
        T: DeserializeOwned,
    {
        let actual_content_type_header = http_response.head().headers().get(header::CONTENT_TYPE);
        let expected_content_type_header =
            HeaderValue::from_str(mime::APPLICATION_JSON.as_ref()).unwrap();
        assert_matches!(actual_content_type_header, Some(value) if value == expected_content_type_header);

        let response_body = http_response.into_body().try_into_bytes().unwrap();
        serde_json::from_reader(response_body.as_ref()).unwrap()
    }

    mod response_error_for_error {
        use std::env;

        use actix_web::error::JsonPayloadError;
        use diesel_async::pooled_connection::deadpool::PoolError;
        use serial_test::file_parallel;

        use super::*;
        use crate::error::{EnvVarContext, QueryContext};

        fn assert_response_error_impl<E>(error: E, expected_status_code: StatusCode)
        where
            E: Into<Error>,
        {
            let error: Error = error.into();

            let actual_status_code = error.status_code();
            assert_eq!(expected_status_code, actual_status_code);

            let response = error.error_response();
            assert_eq!(expected_status_code, response.status());

            let actual_error_response: ErrorResponse = http_response_json_content(response);
            let expected_error_response: ErrorResponse = (&error).into();
            assert_eq!(expected_error_response, actual_error_response);
        }

        fn assert_response_error_impl_for_query<E>(error: E, expected_status_code: StatusCode)
        where
            E: QueryContext<Output = Error>,
        {
            assert_response_error_impl(
                error.with_query_context(|| "query error"),
                expected_status_code,
            );
        }

        #[test]
        #[file_parallel(pokedex_env)]
        fn test_env() {
            assert_response_error_impl(
                env::VarError::NotPresent.with_env_var_context(|| "SOME_ENV_VAR not defined"),
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }

        #[test]
        #[file_parallel(pokedex_env)]
        fn test_input() {
            assert_response_error_impl(
                actix_web_validator::Error::JsonPayloadError(JsonPayloadError::ContentType),
                StatusCode::BAD_REQUEST,
            );
        }

        #[test]
        #[file_parallel(pokedex_env)]
        fn test_pool() {
            assert_response_error_impl(PoolError::Closed, StatusCode::INTERNAL_SERVER_ERROR);
        }

        #[test]
        #[file_parallel(pokedex_env)]
        fn test_query() {
            assert_response_error_impl_for_query(
                diesel::result::Error::NotFound,
                StatusCode::NOT_FOUND,
            );
            assert_response_error_impl_for_query(
                diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    Box::new(String::from("unique violation")),
                ),
                StatusCode::BAD_REQUEST,
            );
            assert_response_error_impl_for_query(
                diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::CheckViolation,
                    Box::new(String::from("check violation")),
                ),
                StatusCode::BAD_REQUEST,
            );
            assert_response_error_impl_for_query(
                diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::ForeignKeyViolation,
                    Box::new(String::from("foreign key violation")),
                ),
                StatusCode::INTERNAL_SERVER_ERROR,
            );
            assert_response_error_impl_for_query(
                diesel::result::Error::BrokenTransactionManager,
                StatusCode::INTERNAL_SERVER_ERROR,
            );
        }
    }

    mod status_code_for_query_errors {
        use actix_web::http::StatusCode;
        use diesel::result::DatabaseErrorKind;

        use super::*;

        fn assert_query_error_status_code<E>(error: E, expected_status_code: Option<StatusCode>)
        where
            E: Into<diesel::result::Error>,
        {
            assert_eq!(expected_status_code, status_code_for_query_error(&error.into()));
        }

        #[test]
        fn test_not_found() {
            assert_query_error_status_code(
                diesel::result::Error::NotFound,
                Some(StatusCode::NOT_FOUND),
            );
        }

        #[test]
        fn test_db_errors() {
            assert_query_error_status_code(
                diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    Box::new(String::from("unique violation")),
                ),
                Some(StatusCode::BAD_REQUEST),
            );

            assert_query_error_status_code(
                diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::CheckViolation,
                    Box::new(String::from("check violation")),
                ),
                Some(StatusCode::BAD_REQUEST),
            );

            assert_query_error_status_code(
                diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::ForeignKeyViolation,
                    Box::new(String::from("foreign key violation")),
                ),
                None,
            );
        }

        #[test]
        fn test_other() {
            assert_query_error_status_code(diesel::result::Error::BrokenTransactionManager, None);
        }
    }

    mod error_response {
        use super::*;

        mod from {
            use super::*;

            mod error_ref {
                use super::*;
                use crate::error::QueryContext;

                async fn test_impl<F>(env: ServiceEnv, internal_error_test: F)
                where
                    F: FnOnce(&Option<String>),
                {
                    ServiceEnv::test(env, async {
                        let error = diesel::result::Error::NotFound
                            .with_query_context(|| "entity not found");
                        let error_response: ErrorResponse = (&error).into();

                        assert_eq!(StatusCode::NOT_FOUND, error_response.status_code);
                        assert_eq!(
                            StatusCode::NOT_FOUND.canonical_reason().unwrap(),
                            error_response.error
                        );

                        internal_error_test(&error_response.internal_error);
                    })
                    .await;
                }

                mod development {
                    use assert_matches::assert_matches;
                    use serial_test::file_serial;

                    use super::*;

                    #[actix_web::test]
                    #[file_serial(pokedex_env)]
                    async fn test_all() {
                        test_impl(ServiceEnv::Development, |internal_error| {
                            assert_matches!(*internal_error, Some(ref internal_error_msg) => {
                                assert!(internal_error_msg.contains("query error: entity not found"));

                                #[cfg(backtrace_support)]
                                assert!(internal_error_msg.contains("Backtrace: "));
                            });
                        }).await;
                    }
                }

                mod production {
                    use serial_test::file_serial;

                    use super::*;

                    #[actix_web::test]
                    #[file_serial(pokedex_env)]
                    async fn test_all() {
                        test_impl(ServiceEnv::Production, |internal_error| {
                            assert_eq!(None, *internal_error);
                        })
                        .await;
                    }
                }
            }
        }
    }

    mod actix_error_handler {
        use serial_test::file_parallel;

        use super::*;
        use crate::error::{EnvVarContext, EnvVarError};

        #[test]
        #[file_parallel(pokedex_env)]
        fn test_handler() {
            let error = EnvVarError::NotFound.with_env_var_context(|| "env var not found");

            let expected_http_response = error.error_response();
            let actual_http_response = actix_error_handler(error, &()).error_response();

            let expected_error_response: ErrorResponse =
                http_response_json_content(expected_http_response);
            let actual_error_response: ErrorResponse =
                http_response_json_content(actual_http_response);
            assert_eq!(expected_error_response, actual_error_response);
        }
    }
}
