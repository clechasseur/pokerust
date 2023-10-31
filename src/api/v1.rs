//! Current version (`v1`) of the Pokedex REST API.

pub mod pokemons;

use actix_web::web;
use actix_web::web::ServiceConfig;
use log::trace;

use crate::db::Pool;

/// Allows registration of the Pokedex API routes under the `/pokemons` scope.
///
/// This includes all endpoints to create, update, etc. pokemons. Called automatically from [`api::configure`](crate::api::configure).
pub fn configure(pool: &Pool) -> impl FnOnce(&mut ServiceConfig) + '_ {
    |config| {
        trace!("Adding API endpoints for /api/v1");
        config.service(web::scope("/pokemons").configure(pokemons::configure(pool)));
    }
}
