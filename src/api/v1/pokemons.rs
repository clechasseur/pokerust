//! Implementation of the Pokedex REST API endpoints for pokemons.
//!
//! # Endpoints
//!
//! | HTTP method | Endpoint                | Usage                                                          | See                       |
//! |-------------|-------------------------|----------------------------------------------------------------|---------------------------|
//! | `GET`       | `/api/v1/pokemons`      | Lists pokemons in the DB, paginated                            | [`list`]                  |
//! | `GET`       | `/api/v1/pokemons/{id}` | Returns one pokemon stored in DB, using its ID                 | [`get`](struct@get)       |
//! | `POST`      | `/api/v1/pokemons`      | Adds a new pokemon in the DB                                   | [`create`]                |
//! | `PUT`       | `/api/v1/pokemons/{id}` | Updates the pokemon with the given ID in the DB                | [`update`]                |
//! | `PATCH`     | `/api/v1/pokemons/{id}` | Updates some fields of the pokemon with the given ID in the DB | [`patch`](struct@patch)   |
//! | `DELETE`    | `/api/v1/pokemons/{id}` | Deletes the pokemon with the given ID from the DB              | [`delete`](struct@delete) |

pub mod doc;

use std::ops::Deref;

use actix_web::web::{Data, ServiceConfig};
use actix_web::{delete, get, patch, post, put, HttpResponse};
use actix_web_validator::{Json, Path, Query};
use log::trace;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use validator::Validate;

use crate::api::v1::pokemons::doc::{
    IdNotFoundResponse, InvalidIdParamOrPokemonBodyResponse, InvalidIdParamResponse,
    InvalidPokemonBodyResponse, ServerErrorResponse,
};
use crate::db::Pool;
use crate::models::pokemon::{CreatePokemon, PatchPokemon, Pokemon, UpdatePokemon};
use crate::services::pokemon;
use crate::services::pokemon::PokemonsPage;

/// Allows registration of all pokemon REST API endpoints.
///
/// See [module documentation](self) for the entire list of supported endpoints.
/// Called automatically from [`api::v1::configure`](crate::api::v1::configure).
pub fn configure(pool: &Pool) -> impl FnOnce(&mut ServiceConfig) + '_ {
    |config| {
        trace!("Registering Pokemon service app data");
        config.app_data(Data::new(pokemon::Service::new(pool.clone())));

        trace!("Adding API CRUD endpoints for /api/v1/pokemons");
        config
            .service(list)
            .service(get)
            .service(create)
            .service(update)
            .service(patch)
            .service(delete);
    }
}

/// [`Result`](crate::Result) definition used to return [`HttpResponse`]s from API endpoints.
///
/// If an [`Error`](crate::Error) is returned, it is converted to an appropriate [`HttpResponse`]
/// by the error handling code (see [`ErrorResponse::from`](crate::api::errors::ErrorResponse::from) for details).
pub type HttpResult = crate::Result<HttpResponse>;

/// Default value of the [`page_size`](ListParams::page_size) query parameter used when [listing pokemons](list).
pub const DEFAULT_PAGE_SIZE: i64 = 10;

/// Provides default value of the [`page_size`](ListParams::page_size) query parameter used when [listing pokemons](list).
///
/// Provided because [`IntoParams`] needs a function to fetch a computed value; a constant does not work.
///
/// # See also
///
/// [`DEFAULT_PAGE_SIZE`]
pub fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}

/// Path parameter used for endpoints with a Pokemon id ([`get`](struct@get), [`update`], [`patch`](struct@patch) and [`delete`](struct@delete)).
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Validate, IntoParams)]
pub struct Id {
    /// id of Pokemon in database
    #[validate(range(min = 0))]
    #[param(minimum = 0)]
    pub id: i64,
}

/// Query parameters for [list endpoint](list). Includes optional paging information.
///
/// See [`ListParams::default`] for the default values.
///
/// # Notes
///
/// Setting [`page_size`](ListParams::page_size) to a value greater than the [maximum](crate::services::pokemon::Service::MAX_PAGE_SIZE)
/// will have no effect (the maximum value will be used instead).
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Validate, IntoParams)]
#[serde(default, deny_unknown_fields)]
pub struct ListParams {
    /// Index of the page to fetch (1-based)
    #[validate(range(min = 1))]
    #[param(minimum = 1, default = 1)]
    pub page: i64,

    /// Number of Pokemons to return in each page
    #[validate(range(min = 1))]
    #[param(minimum = 1, maximum = 100, default = default_page_size)]
    pub page_size: i64,
}

impl Deref for Id {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl Default for ListParams {
    /// Returns the default values of the query parameters passed to the API endpoint that [lists pokemons](list).
    ///
    /// | Query parameter | Default value         |
    /// |-----------------|-----------------------|
    /// | `page`          | 1                     |
    /// | `page_size`     | [`DEFAULT_PAGE_SIZE`] |
    fn default() -> Self {
        Self { page: 1, page_size: DEFAULT_PAGE_SIZE }
    }
}

#[cfg_attr(
    doc,
    doc = r"
        API endpoint to list pokemons in a paginated way.

        Registered as `GET /api/v1/pokemons`.

        # Input

        | Query parameter | Usage                                      |
        |-----------------|--------------------------------------------|
        | `page`          | Index of page to fetch (1-based)           |
        | `page_size`     | Number of pokemons to include in each page |

        See [`ListParams::default`] for default values.

        # Output

        The endpoint returns a [`PokemonsPage`], serialized as JSON. This struct includes the list of
        [`Pokemon`]s in the page, as well as a [`total_pages`](PokemonsPage::total_pages) field that
        contains the total number of pages that could theoretically be returned. Note that if pokemons
        are inserted in the DB while paginated list calls are performed, this may change between calls.
    "
)]
#[cfg_attr(not(doc), doc = "Lists Pokemons in the Pokedex in a paginated way")]
#[utoipa::path(
    context_path = "/api/v1/pokemons",
    params(ListParams),
    responses(
        (status = OK, response = PokemonsPage),
        ServerErrorResponse,
    ),
)]
#[get("", name = "/")]
pub async fn list(params: Query<ListParams>, service: Data<pokemon::Service>) -> HttpResult {
    let pokemons_page = service
        .get_ref()
        .get_pokemons(params.page, params.page_size)
        .await?;

    Ok(HttpResponse::Ok().json(pokemons_page))
}

#[cfg_attr(
    doc,
    doc = r"
        API endpoint to fetch one pokemon from the DB.

        Registered as `GET /api/v1/pokemons/{id}`.

        # Input

        - `{id}`: ID of pokemon to fetch.

        # Output

        A [`Pokemon`], serialized as JSON.
    "
)]
#[cfg_attr(not(doc), doc = "Returns information about a Pokemon")]
#[utoipa::path(
    context_path = "/api/v1/pokemons",
    params(Id),
    responses(
        (status = OK, response = Pokemon),
        InvalidIdParamResponse,
        IdNotFoundResponse,
        ServerErrorResponse,
    ),
)]
#[get("/{id}", name = "/{id}")]
pub async fn get(id: Path<Id>, service: Data<pokemon::Service>) -> HttpResult {
    let pokemon = service.get_ref().get_pokemon(*id.into_inner()).await?;

    Ok(HttpResponse::Ok().json(pokemon))
}

#[cfg_attr(
    doc,
    doc = r"
        API endpoint to add a new pokemon to the DB.

        Registered as `POST /api/v1/pokemons`.

        # Input

        - Request body: the pokemon data, as a JSON-serialized [`CreatePokemon`].

        # Output

        The newly-inserted [`Pokemon`], serialized as JSON.
    "
)]
#[cfg_attr(not(doc), doc = "Creates a new Pokemon")]
#[utoipa::path(
    context_path = "/api/v1/pokemons",
    request_body(
        content = inline(CreatePokemon),
        description = "New Pokemon information",
    ),
    responses(
        (status = CREATED, response = Pokemon),
        InvalidPokemonBodyResponse,
        ServerErrorResponse,
    ),
)]
#[post("", name = "/")]
pub async fn create(
    new_pokemon: Json<CreatePokemon>,
    service: Data<pokemon::Service>,
) -> HttpResult {
    let pokemon = service.get_ref().create_pokemon(&new_pokemon).await?;

    Ok(HttpResponse::Created().json(pokemon))
}

#[cfg_attr(
    doc,
    doc = r"
        API endpoint to update a pokemon in the DB.

        Updates all fields of the pokemon in one go. Registered as `PUT /api/v1/pokemons/{id}`.

        # Input

        - `{id}`: ID of pokemon to update.
        - Request body: the updated pokemon data, as a JSON-serialized [`UpdatePokemon`]. Must include
                        all fields or the request will be rejected.

        # Output

        The updated [`Pokemon`], serialized as JSON.
    "
)]
#[cfg_attr(not(doc), doc = "Updates a Pokemon")]
#[utoipa::path(
    context_path = "/api/v1/pokemons",
    params(Id),
    request_body(
        content = inline(UpdatePokemon),
        description = "Updated Pokemon information",
    ),
    responses(
        (status = OK, response = Pokemon),
        InvalidIdParamOrPokemonBodyResponse,
        IdNotFoundResponse,
        ServerErrorResponse,
    ),
)]
#[put("/{id}", name = "/{id}")]
pub async fn update(
    id: Path<Id>,
    updated_pokemon: Json<UpdatePokemon>,
    service: Data<pokemon::Service>,
) -> HttpResult {
    let pokemon = service
        .get_ref()
        .update_pokemon(*id.into_inner(), &updated_pokemon)
        .await?;

    Ok(HttpResponse::Ok().json(pokemon))
}

#[cfg_attr(
    doc,
    doc = r"
        API endpoint to update some fields of a pokemon in the DB.

        Any field not specified will not be updated. Registered as `PATCH /api/v1/pokemons/{id}`.

        # Input

        - `{id}`: ID of pokemon to update.
        - Request body: the fields to update in the pokemon, as a JSON-serialized [`PatchPokemon`][^1].

        # Output

        The updated [`Pokemon`], serialized as JSON.

        [^1]: Any nullable field in the pokemon (like for example `type_2`) can be set to `NULL` in the
              DB by specifying them in the input data as a JSON `null` value. If the field is omitted
              in the input data, its value will not be updated. (For more details, see for example
              [`PatchPokemon::type_2`].)
    "
)]
#[cfg_attr(not(doc), doc = "Updates specific fields of a Pokemon")]
#[utoipa::path(
    context_path = "/api/v1/pokemons",
    params(Id),
    request_body(
        content = inline(PatchPokemon),
        description = "Specific Pokemon fields to update",
    ),
    responses(
        (status = OK, response = Pokemon),
        InvalidIdParamOrPokemonBodyResponse,
        IdNotFoundResponse,
        ServerErrorResponse,
    ),
)]
#[patch("/{id}", name = "/{id}")]
pub async fn patch(
    id: Path<Id>,
    pokemon_patch: Json<PatchPokemon>,
    service: Data<pokemon::Service>,
) -> HttpResult {
    let pokemon = service
        .get_ref()
        .patch_pokemon(*id.into_inner(), &pokemon_patch)
        .await?;

    Ok(HttpResponse::Ok().json(pokemon))
}

#[cfg_attr(
    doc,
    doc = r"
        API endpoint to delete a pokemon from the DB.

        Registered as `DELETE /api/v1/pokemons/{id}`.

        # Input

        - `{id}`: ID of pokemon to delete.

        # Output

        This endpoint simply returns `HTTP 204 No Content` upon success.
    "
)]
#[cfg_attr(not(doc), doc = "Deletes a Pokemon")]
#[utoipa::path(
    context_path = "/api/v1/pokemons",
    params(Id),
    responses(
        (status = NO_CONTENT, description = "Pokemon deleted from Pokedex"),
        InvalidIdParamResponse,
        IdNotFoundResponse,
        ServerErrorResponse,
    ),
)]
#[delete("/{id}", name = "/{id}")]
pub async fn delete(id: Path<Id>, service: Data<pokemon::Service>) -> HttpResult {
    service.get_ref().delete_pokemon(*id.into_inner()).await?;

    Ok(HttpResponse::NoContent().finish())
}
