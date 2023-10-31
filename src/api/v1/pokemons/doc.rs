//! [`IntoResponses`] wrappers for Pokedex REST API endpoints.
//!
//! These helper types are used to document the possible API responses using [`utoipa::path`].

use utoipa::IntoResponses;

use crate::api::errors::ErrorResponse;

/// [`IntoResponses`] wrapper for bad `id` path parameter errors.
///
/// Can be used to document 400 API error responses using [`utoipa::path`].
#[derive(Debug, IntoResponses)]
#[response(status = BAD_REQUEST, description = "Invalid value for id path parameter")]
pub struct InvalidIdParamResponse;

/// [`IntoResponses`] wrapper for bad Pokemon request body error.
///
/// Can be used to document 400 API error responses using [`utoipa::path`].
#[derive(Debug, IntoResponses)]
#[response(status = BAD_REQUEST, description = "Invalid Pokemon information in request body")]
pub struct InvalidPokemonBodyResponse;

/// [`IntoResponses`] wrapper for bad `id` path parameter OR bad Pokemon request body error.
///
/// Can be used to document 400 API error responses using [`utoipa::path`].
#[derive(Debug, IntoResponses)]
#[response(
    status = BAD_REQUEST,
    description = "Invalid value for id path parameter OR invalid Pokemon information in request body",
)]
pub struct InvalidIdParamOrPokemonBodyResponse;

/// [`IntoResponses`] wrapper for `Pokemon not found` errors.
///
/// Can be used to document 404 API error responses using [`utoipa::path`].
#[derive(Debug, IntoResponses)]
#[response(status = NOT_FOUND, description = "Requested Pokemon not found in database")]
pub struct IdNotFoundResponse;

/// [`IntoResponses`] wrapper for internal server errors.
///
/// Can be used to document 5XX API error responses using [`utoipa::path`].
#[derive(Debug, IntoResponses)]
#[response(status = "5XX")]
pub struct ServerErrorResponse(#[to_response] ErrorResponse);
