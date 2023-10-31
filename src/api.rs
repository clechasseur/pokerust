//! Types and functions used to implement the Pokedex REST API.

pub mod doc;
pub mod errors;
pub mod v1;

use actix_web::web;
use actix_web::web::ServiceConfig;
use log::trace;

use crate::db::Pool;

/// Allows registration of the current version of the Pokedex API under the `/v1` scope.
///
/// Called automatically from [`configure_api`](crate::configure_api).
pub fn configure(pool: &Pool) -> impl FnOnce(&mut ServiceConfig) + '_ {
    |config| {
        trace!("Adding API endpoints for /api");
        config.service(web::scope("/v1").configure(v1::configure(pool)));
    }
}
