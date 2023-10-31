//! OpenAPI documentation support.

use actix_web::web::ServiceConfig;
use log::trace;
use utoipa::OpenApi;
use utoipa_rapidoc::RapiDoc;
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

use crate::api;
use crate::api::errors::ErrorResponse;
use crate::models::pokemon::Pokemon;
use crate::services::pokemon::PokemonsPage;

/// Registers the various OpenAPI-related endpoints, like swagger UI.
///
/// Called automatically from [`configure_api`](crate::configure_api).
pub fn configure(config: &mut ServiceConfig) {
    trace!("Adding OpenAPI doc endpoints");

    let openapi = ApiDoc::openapi();
    config
        .service(
            SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
        )
        .service(Redoc::with_url("/redoc", openapi.clone()))
        .service(RapiDoc::new("/api-docs/openapi.json").path("/rapidoc"));
}

/// API documentation in OpenAPI format.
///
/// Generated automatically by the [`utoipa`] crate. To use, simply call [`ApiDoc::openapi`]
/// to create an instance, then pass it to the various helpers that allow the API doc to be
/// published, like [`SwaggerUi`].
#[derive(OpenApi)]
#[openapi(
    paths(
        api::v1::pokemons::list,
        api::v1::pokemons::get,
        api::v1::pokemons::create,
        api::v1::pokemons::update,
        api::v1::pokemons::patch,
        api::v1::pokemons::delete,
    ),
    components(schemas(Pokemon), responses(PokemonsPage, Pokemon, ErrorResponse))
)]
pub struct ApiDoc;
