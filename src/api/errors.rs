//! Types and functions to implement proper error handling in the Pokedex API.

use actix_web::body::BoxBody;
use actix_web::error::JsonPayloadError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use actix_web_validator::error::DeserializeErrors;
use actix_web_validator::Error as ValidationError;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TryFromInto};
use utoipa::{ToResponse, ToSchema};

use crate::error::{InputContext, InputErrorContext};
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
            Error::Input { context, source, .. } => status_code_for_input_error(*context, source),
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

/// Helper function to get a [`StatusCode`] for an [input error](ValidationError).
///
/// If the error is due to validation failures that occur while parsing an entity in the POST data
/// of a request, this function will return [`Some(UNPROCESSABLE_ENTITY)`](StatusCode::UNPROCESSABLE_ENTITY).
/// If the error is due to other invalid data issues, this function will return [`Some(BAD_REQUEST)`](StatusCode::BAD_REQUEST).
/// Otherwise, it will return `None` and the caller can decide what status code to use.
pub fn status_code_for_input_error(
    context: InputErrorContext,
    error: &ValidationError,
) -> Option<StatusCode> {
    match error {
        // Validation errors should return 422 Unprocessable Entity _only_ when a JSON validation occurs.
        // Otherwise, it's not an entity, so we return 400 Bad Request.
        ValidationError::Validate(_) if context.is_json() => Some(StatusCode::UNPROCESSABLE_ENTITY),
        ValidationError::Validate(_) => Some(StatusCode::BAD_REQUEST),

        // Deserialization errors are caused by faulty input, for which we return 400 Bad Request.
        ValidationError::Deserialize(DeserializeErrors::DeserializeQuery(_))
            if context.is_query() =>
        {
            Some(StatusCode::BAD_REQUEST)
        },
        ValidationError::Deserialize(DeserializeErrors::DeserializeJson(_))
            if context.is_json() =>
        {
            Some(StatusCode::BAD_REQUEST)
        },
        ValidationError::Deserialize(DeserializeErrors::DeserializePath(_))
            if context.is_path() =>
        {
            Some(StatusCode::BAD_REQUEST)
        },

        // Note: I believe that JSON deserialization errors should not result in the error below, but rather
        // in the ValidationError::Deserialize variant with a DeserializeErrors::DeserializeJson inside.
        // I've entered an issue for this: https://github.com/rambler-digital-solutions/actix-web-validator/issues/47
        ValidationError::JsonPayloadError(JsonPayloadError::Deserialize(_))
            if context.is_json() =>
        {
            Some(StatusCode::BAD_REQUEST)
        },

        // Other JSON payload errors are caused by faulty input or broken pipe on the client side, for which we return 400 Bad Request.
        ValidationError::JsonPayloadError(_) if context.is_json() => Some(StatusCode::BAD_REQUEST),

        // Url-encoded errors are caused by faulty input, for which we return 400 Bad Request.
        ValidationError::UrlEncodedError(_) if context.is_query() => Some(StatusCode::BAD_REQUEST),

        // Any other combination is a programmer error (possibly on the part of the programmer of a dependency).
        _ => None,
    }
}

/// Helper function to get a [`StatusCode`] for a [query error](DieselError).
///
/// If the error is due to faulty user input (like [`NotFound`](DieselError::NotFound)),
/// this function will return `Some` with an appropriate HTTP status code (like [`NOT_FOUND`](StatusCode::NOT_FOUND)).
/// Otherwise, it will return `None` and the caller can decide what status code to use.
pub fn status_code_for_query_error(error: &DieselError) -> Option<StatusCode> {
    match error {
        DieselError::NotFound => Some(StatusCode::NOT_FOUND),
        DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation | DatabaseErrorKind::CheckViolation,
            ..,
        ) => Some(StatusCode::UNPROCESSABLE_ENTITY),
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

/// Generic error handler for input validation errors.
///
/// This handler accepts any type of error that implements [`InputContext`] (e.g. can be turned into an
/// [`Input`](Error::Input) error), then turns that into an [`actix_web::error::Error`]. This is possible
/// because we implemented [`ResponseError`] for our [`Error`] type - thus, `actix_web` will use our implementation
/// to generate an appropriate HTTP response for the type of error encountered. This makes it possible to handle
/// pre-request handler errors (like for example [`DeserializeErrors`]s) using the same error handling code as
/// in-request handler errors.
///
/// # Examples
///
/// ```no_run
/// use actix_web_validator::{JsonConfig, PathConfig, QueryConfig};
/// use pokedex_rs::api::errors::input_error_handler;
/// use pokedex_rs::error::InputErrorContext;
///
/// let json_config =
///     JsonConfig::default().error_handler(input_error_handler(InputErrorContext::Json));
/// let path_config =
///     PathConfig::default().error_handler(input_error_handler(InputErrorContext::Path));
/// let query_config =
///     QueryConfig::default().error_handler(input_error_handler(InputErrorContext::Query));
/// ```
///
/// [`DeserializeErrors`]: actix_web_validator::error::DeserializeErrors
pub fn input_error_handler<E, R>(
    context: InputErrorContext,
) -> impl Fn(E, &R) -> actix_web::error::Error + Send + Sync + 'static
where
    E: InputContext<Output = Error>,
{
    move |err, _req| err.with_input_context(context).into()
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

        use serial_test::file_parallel;

        use super::*;
        use crate::error::QueryContext;

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

        mod env_var {
            use super::*;
            use crate::error::EnvVarContext;

            #[test]
            #[file_parallel(pokedex_env)]
            fn test_all() {
                assert_response_error_impl(
                    env::VarError::NotPresent.with_env_var_context(|| "SOME_ENV_VAR not defined"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        }

        mod input {
            use actix_web::error::UrlencodedError;
            use serde::de;
            use serde::de::Error as _;
            use validator::ValidationErrors;

            use super::*;

            fn assert_response_error_impl_for_input<E>(
                error: E,
                context: InputErrorContext,
                expected_status_code: StatusCode,
            ) where
                E: InputContext<Output = Error>,
            {
                assert_response_error_impl(error.with_input_context(context), expected_status_code);
            }

            mod context {
                use super::*;

                mod query {
                    use super::*;

                    fn assert_error_impl_for_query<E>(error: E, expected_status_code: StatusCode)
                    where
                        E: InputContext<Output = Error>,
                    {
                        assert_response_error_impl_for_input(
                            error,
                            InputErrorContext::Query,
                            expected_status_code,
                        );
                    }

                    #[test]
                    #[file_parallel(pokedex_env)]
                    fn test_all() {
                        assert_error_impl_for_query(
                            ValidationError::Validate(ValidationErrors::new()),
                            StatusCode::BAD_REQUEST,
                        );

                        assert_error_impl_for_query(
                            ValidationError::Deserialize(DeserializeErrors::DeserializeQuery(
                                serde_urlencoded::de::Error::custom("query error"),
                            )),
                            StatusCode::BAD_REQUEST,
                        );
                        assert_error_impl_for_query(
                            ValidationError::Deserialize(DeserializeErrors::DeserializeJson(
                                serde_json::Error::custom("json error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                        assert_error_impl_for_query(
                            ValidationError::Deserialize(DeserializeErrors::DeserializePath(
                                de::value::Error::custom("path error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );

                        assert_error_impl_for_query(
                            ValidationError::JsonPayloadError(JsonPayloadError::ContentType),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                        assert_error_impl_for_query(
                            ValidationError::JsonPayloadError(JsonPayloadError::Deserialize(
                                serde_json::Error::custom("json error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );

                        assert_error_impl_for_query(
                            ValidationError::UrlEncodedError(UrlencodedError::ContentType),
                            StatusCode::BAD_REQUEST,
                        );
                    }
                }

                mod json {
                    use super::*;

                    fn assert_error_impl_for_json<E>(error: E, expected_status_code: StatusCode)
                    where
                        E: InputContext<Output = Error>,
                    {
                        assert_response_error_impl_for_input(
                            error,
                            InputErrorContext::Json,
                            expected_status_code,
                        );
                    }

                    #[test]
                    #[file_parallel(pokedex_env)]
                    fn test_all() {
                        assert_error_impl_for_json(
                            ValidationError::Validate(ValidationErrors::new()),
                            StatusCode::UNPROCESSABLE_ENTITY,
                        );

                        assert_error_impl_for_json(
                            ValidationError::Deserialize(DeserializeErrors::DeserializeQuery(
                                serde_urlencoded::de::Error::custom("query error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                        assert_error_impl_for_json(
                            ValidationError::Deserialize(DeserializeErrors::DeserializeJson(
                                serde_json::Error::custom("json error"),
                            )),
                            StatusCode::BAD_REQUEST,
                        );
                        assert_error_impl_for_json(
                            ValidationError::Deserialize(DeserializeErrors::DeserializePath(
                                de::value::Error::custom("path error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );

                        assert_error_impl_for_json(
                            ValidationError::JsonPayloadError(JsonPayloadError::ContentType),
                            StatusCode::BAD_REQUEST,
                        );
                        assert_error_impl_for_json(
                            ValidationError::JsonPayloadError(JsonPayloadError::Deserialize(
                                serde_json::Error::custom("json error"),
                            )),
                            StatusCode::BAD_REQUEST,
                        );

                        assert_error_impl_for_json(
                            ValidationError::UrlEncodedError(UrlencodedError::ContentType),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                    }
                }

                mod path {
                    use super::*;

                    fn assert_error_impl_for_path<E>(error: E, expected_status_code: StatusCode)
                    where
                        E: InputContext<Output = Error>,
                    {
                        assert_response_error_impl_for_input(
                            error,
                            InputErrorContext::Path,
                            expected_status_code,
                        );
                    }

                    #[test]
                    #[file_parallel(pokedex_env)]
                    fn test_all() {
                        assert_error_impl_for_path(
                            ValidationError::Validate(ValidationErrors::new()),
                            StatusCode::BAD_REQUEST,
                        );

                        assert_error_impl_for_path(
                            ValidationError::Deserialize(DeserializeErrors::DeserializeQuery(
                                serde_urlencoded::de::Error::custom("query error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                        assert_error_impl_for_path(
                            ValidationError::Deserialize(DeserializeErrors::DeserializeJson(
                                serde_json::Error::custom("json error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                        assert_error_impl_for_path(
                            ValidationError::Deserialize(DeserializeErrors::DeserializePath(
                                de::value::Error::custom("path error"),
                            )),
                            StatusCode::BAD_REQUEST,
                        );

                        assert_error_impl_for_path(
                            ValidationError::JsonPayloadError(JsonPayloadError::ContentType),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                        assert_error_impl_for_path(
                            ValidationError::JsonPayloadError(JsonPayloadError::Deserialize(
                                serde_json::Error::custom("json error"),
                            )),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );

                        assert_error_impl_for_path(
                            ValidationError::UrlEncodedError(UrlencodedError::ContentType),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        );
                    }
                }
            }
        }

        mod pool {
            use diesel_async::pooled_connection::deadpool::PoolError;

            use super::*;

            #[test]
            #[file_parallel(pokedex_env)]
            fn test_all() {
                assert_response_error_impl(PoolError::Closed, StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        mod query {
            use super::*;

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
            fn test_all() {
                assert_response_error_impl_for_query(DieselError::NotFound, StatusCode::NOT_FOUND);
                assert_response_error_impl_for_query(
                    DieselError::DatabaseError(
                        DatabaseErrorKind::UniqueViolation,
                        Box::new(String::from("unique violation")),
                    ),
                    StatusCode::UNPROCESSABLE_ENTITY,
                );
                assert_response_error_impl_for_query(
                    DieselError::DatabaseError(
                        DatabaseErrorKind::CheckViolation,
                        Box::new(String::from("check violation")),
                    ),
                    StatusCode::UNPROCESSABLE_ENTITY,
                );
                assert_response_error_impl_for_query(
                    DieselError::DatabaseError(
                        DatabaseErrorKind::ForeignKeyViolation,
                        Box::new(String::from("foreign key violation")),
                    ),
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
                assert_response_error_impl_for_query(
                    DieselError::BrokenTransactionManager,
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        }
    }

    mod status_code_for_input_errors {
        use actix_web::error::UrlencodedError;
        use serde::de;
        use serde::de::Error as _;
        use validator::ValidationErrors;

        use super::*;

        fn assert_input_error_status_code<E>(
            context: InputErrorContext,
            error: E,
            expected_status_code: Option<StatusCode>,
        ) where
            E: Into<ValidationError>,
        {
            assert_eq!(expected_status_code, status_code_for_input_error(context, &error.into()));
        }

        #[test]
        fn test_unprocessable_entity() {
            assert_input_error_status_code(
                InputErrorContext::Json,
                ValidationError::Validate(ValidationErrors::new()),
                Some(StatusCode::UNPROCESSABLE_ENTITY),
            );
        }

        #[test]
        fn test_bad_request() {
            assert_input_error_status_code(
                InputErrorContext::Path,
                ValidationError::Validate(ValidationErrors::new()),
                Some(StatusCode::BAD_REQUEST),
            );
            assert_input_error_status_code(
                InputErrorContext::Query,
                ValidationError::Validate(ValidationErrors::new()),
                Some(StatusCode::BAD_REQUEST),
            );

            assert_input_error_status_code(
                InputErrorContext::Query,
                ValidationError::Deserialize(DeserializeErrors::DeserializeQuery(
                    serde_urlencoded::de::Error::custom("query error"),
                )),
                Some(StatusCode::BAD_REQUEST),
            );
            assert_input_error_status_code(
                InputErrorContext::Json,
                ValidationError::Deserialize(DeserializeErrors::DeserializeJson(
                    serde_json::Error::custom("json error"),
                )),
                Some(StatusCode::BAD_REQUEST),
            );
            assert_input_error_status_code(
                InputErrorContext::Path,
                ValidationError::Deserialize(DeserializeErrors::DeserializePath(
                    de::value::Error::custom("path error"),
                )),
                Some(StatusCode::BAD_REQUEST),
            );

            assert_input_error_status_code(
                InputErrorContext::Json,
                ValidationError::JsonPayloadError(JsonPayloadError::ContentType),
                Some(StatusCode::BAD_REQUEST),
            );
            assert_input_error_status_code(
                InputErrorContext::Json,
                ValidationError::JsonPayloadError(JsonPayloadError::Deserialize(
                    serde_json::Error::custom("json error"),
                )),
                Some(StatusCode::BAD_REQUEST),
            );

            assert_input_error_status_code(
                InputErrorContext::Query,
                ValidationError::UrlEncodedError(UrlencodedError::ContentType),
                Some(StatusCode::BAD_REQUEST),
            );
        }

        mod other {
            use super::*;

            fn assert_input_error_status_code_none<E>(context: InputErrorContext, error: E)
            where
                E: Into<ValidationError>,
            {
                assert_input_error_status_code(context, error, None);
            }

            mod context {
                use super::*;

                mod query {
                    use super::*;

                    fn assert_query_error_status_code_none<E>(error: E)
                    where
                        E: Into<ValidationError>,
                    {
                        assert_input_error_status_code_none(InputErrorContext::Query, error);
                    }

                    #[test]
                    fn test_other() {
                        assert_query_error_status_code_none(ValidationError::Deserialize(
                            DeserializeErrors::DeserializeJson(serde_json::Error::custom(
                                "json error",
                            )),
                        ));
                        assert_query_error_status_code_none(ValidationError::Deserialize(
                            DeserializeErrors::DeserializePath(de::value::Error::custom(
                                "path error",
                            )),
                        ));

                        assert_query_error_status_code_none(ValidationError::JsonPayloadError(
                            JsonPayloadError::ContentType,
                        ));
                        assert_query_error_status_code_none(ValidationError::JsonPayloadError(
                            JsonPayloadError::Deserialize(serde_json::Error::custom("json error")),
                        ));
                    }
                }

                mod json {
                    use super::*;

                    fn assert_json_error_status_code_none<E>(error: E)
                    where
                        E: Into<ValidationError>,
                    {
                        assert_input_error_status_code_none(InputErrorContext::Json, error);
                    }

                    #[test]
                    fn test_other() {
                        assert_json_error_status_code_none(ValidationError::Deserialize(
                            DeserializeErrors::DeserializeQuery(
                                serde_urlencoded::de::Error::custom("query error"),
                            ),
                        ));
                        assert_json_error_status_code_none(ValidationError::Deserialize(
                            DeserializeErrors::DeserializePath(de::value::Error::custom(
                                "path error",
                            )),
                        ));

                        assert_json_error_status_code_none(ValidationError::UrlEncodedError(
                            UrlencodedError::ContentType,
                        ));
                    }
                }

                mod path {
                    use super::*;

                    fn assert_path_error_status_code_none<E>(error: E)
                    where
                        E: Into<ValidationError>,
                    {
                        assert_input_error_status_code_none(InputErrorContext::Path, error);
                    }

                    #[test]
                    fn test_other() {
                        assert_path_error_status_code_none(ValidationError::Deserialize(
                            DeserializeErrors::DeserializeQuery(
                                serde_urlencoded::de::Error::custom("query error"),
                            ),
                        ));
                        assert_path_error_status_code_none(ValidationError::Deserialize(
                            DeserializeErrors::DeserializeJson(serde_json::Error::custom(
                                "json error",
                            )),
                        ));

                        assert_path_error_status_code_none(ValidationError::JsonPayloadError(
                            JsonPayloadError::ContentType,
                        ));
                        assert_path_error_status_code_none(ValidationError::JsonPayloadError(
                            JsonPayloadError::Deserialize(serde_json::Error::custom("json error")),
                        ));

                        assert_path_error_status_code_none(ValidationError::UrlEncodedError(
                            UrlencodedError::ContentType,
                        ));
                    }
                }
            }
        }
    }

    mod status_code_for_query_errors {
        use super::*;

        fn assert_query_error_status_code<E>(error: E, expected_status_code: Option<StatusCode>)
        where
            E: Into<DieselError>,
        {
            assert_eq!(expected_status_code, status_code_for_query_error(&error.into()));
        }

        #[test]
        fn test_not_found() {
            assert_query_error_status_code(DieselError::NotFound, Some(StatusCode::NOT_FOUND));
        }

        #[test]
        fn test_db_errors() {
            assert_query_error_status_code(
                DieselError::DatabaseError(
                    DatabaseErrorKind::UniqueViolation,
                    Box::new(String::from("unique violation")),
                ),
                Some(StatusCode::UNPROCESSABLE_ENTITY),
            );

            assert_query_error_status_code(
                DieselError::DatabaseError(
                    DatabaseErrorKind::CheckViolation,
                    Box::new(String::from("check violation")),
                ),
                Some(StatusCode::UNPROCESSABLE_ENTITY),
            );

            assert_query_error_status_code(
                DieselError::DatabaseError(
                    DatabaseErrorKind::ForeignKeyViolation,
                    Box::new(String::from("foreign key violation")),
                ),
                None,
            );
        }

        #[test]
        fn test_other() {
            assert_query_error_status_code(DieselError::BrokenTransactionManager, None);
        }
    }

    mod error_response {
        use super::*;

        mod from {
            use super::*;

            mod error_ref {
                use serial_test::file_serial;

                use super::*;
                use crate::error::QueryContext;

                async fn test_impl<F>(env: ServiceEnv, internal_error_test: F)
                where
                    F: FnOnce(&Option<String>),
                {
                    ServiceEnv::test(env, async {
                        let error = DieselError::NotFound.with_query_context(|| "entity not found");
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

    mod input_error_handler {
        use actix_web::error::UrlencodedError;
        use serde::de;
        use serde::de::Error as _;
        use serial_test::file_parallel;
        use validator::ValidationErrors;

        use super::*;

        fn test_handler<I, E>(context: InputErrorContext, input_context: I, error: E)
        where
            I: InputContext<Output = Error>,
            E: Into<Error>,
        {
            let error = error.into();

            let expected_http_response = error.error_response();
            let actual_http_response =
                input_error_handler(context)(input_context, &()).error_response();

            let expected_error_response: ErrorResponse =
                http_response_json_content(expected_http_response);
            let actual_error_response: ErrorResponse =
                http_response_json_content(actual_http_response);
            assert_eq!(expected_error_response, actual_error_response);
        }

        macro_rules! test_handler {
            ($context:expr, $error:expr) => {{
                let input_context = $error;
                let error: $crate::Error = $error.with_input_context($context);
                test_handler($context, input_context, error);
            }};
        }

        fn test_handler_for_context(context: InputErrorContext) {
            test_handler!(context, ValidationError::Validate(ValidationErrors::new()));

            test_handler!(
                context,
                ValidationError::Deserialize(DeserializeErrors::DeserializeQuery(
                    serde_urlencoded::de::Error::custom("query error")
                ))
            );
            test_handler!(
                context,
                ValidationError::Deserialize(DeserializeErrors::DeserializeJson(
                    serde_json::Error::custom("json error")
                ))
            );
            test_handler!(
                context,
                ValidationError::Deserialize(DeserializeErrors::DeserializePath(
                    de::value::Error::custom("path error")
                ))
            );

            test_handler!(
                context,
                ValidationError::JsonPayloadError(JsonPayloadError::Deserialize(
                    serde_json::Error::custom("json error")
                ))
            );
            test_handler!(
                context,
                ValidationError::JsonPayloadError(JsonPayloadError::ContentType)
            );

            test_handler!(context, ValidationError::UrlEncodedError(UrlencodedError::ContentType));
        }

        mod query {
            use super::*;

            #[test]
            #[file_parallel(pokedex_env)]
            fn test_all() {
                test_handler_for_context(InputErrorContext::Query);
            }
        }

        mod json {
            use super::*;

            #[test]
            #[file_parallel(pokedex_env)]
            fn test_all() {
                test_handler_for_context(InputErrorContext::Json);
            }
        }

        mod path {
            use super::*;

            #[test]
            #[file_parallel(pokedex_env)]
            fn test_all() {
                test_handler_for_context(InputErrorContext::Path);
            }
        }
    }
}
