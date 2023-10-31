//! Private lib crate implementing the types and functions required to implement a CRUD REST API
//! for the Pokedex.
//!
//! Includes everything needed to connect to the database, perform queries and publish the REST
//! endpoints required. Used mainly through the [`pokedex_app!`] macro to create a Pokedex [`App`](actix_web::App).
//!
//! # Notes
//!
//! Pretty much everything in this crate is `pub`lic. This would not normally be the case, but
//! it was done for demo purposes, so that it's easier to document the various pieces.

#![cfg_attr(backtrace_support, feature(error_generic_member_access))]
#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::private_intra_doc_links)]

pub mod api;
pub mod db;
pub mod error;
pub mod helpers;
pub mod models;
#[doc(hidden)]
#[cfg(not(tarpaulin_include))]
pub mod schema;
pub mod service_env;
pub mod services;

use actix_web::web;
use actix_web::web::ServiceConfig;
use actix_web_validator::{JsonConfig, PathConfig};
use api::errors::actix_error_handler;
pub use error::Error;
pub use error::Result;
use log::trace;

use crate::db::Pool;

/// Macro that expands to an [`App`] instance, initialized for our web application.
///
/// For the main binary crate, pass the [`App`] as factory to [`HttpServer::new`] to
/// serve the app's endpoints over HTTP. For tests, pass the [`App`] to [`test::init_service`]
/// to initialize a test service.
///
/// # Notes
///
/// It is possible to further modify the [`App`] generated by this macro. For example:
///
/// ```no_run
/// # use actix_web::{HttpResponse, web};
/// # use pokedex::db::get_pool;
/// # use pokedex::pokedex_app;
/// #
/// # let pool = get_pool().unwrap();
/// // let pool = ...;
/// let app = pokedex_app!(pool).route("/", web::get().to(|| HttpResponse::Ok()));
/// ```
///
/// [`App`]: actix_web::App
/// [`HttpServer::new`]: actix_web::HttpServer::new
/// [`test::init_service`]: actix_web::test::init_service
#[macro_export]
macro_rules! pokedex_app {
    ($pool:expr) => {
        actix_web::App::new()
            .wrap(actix_web::middleware::Logger::default())
            .app_data($crate::get_json_config())
            .app_data($crate::get_path_config())
            .configure($crate::configure_api(&($pool)))
    };
}

/// Allows registration of the entire Pokedex API under the `/api` scope.
///
/// Do not use this function directly; instead, use the [`pokedex_app!`] macro to initialize an
/// [`App`](actix_web::App) instance.
pub fn configure_api(pool: &Pool) -> impl FnOnce(&mut ServiceConfig) + '_ {
    |config| {
        trace!("Configuring Pokedex API");

        trace!("Adding API endpoints for /");
        config
            .service(web::scope("/api").configure(api::configure(pool)))
            .configure(api::doc::configure);
    }
}

/// Returns the [`JsonConfig`] to use for our service.
///
/// This config will register a custom error handler that will handle deserialization errors
/// using our [`ResponseError` impl](Error#impl-ResponseError-for-Error).
///
/// # Notes
///
/// This function cannot be generic over the config type, because unfortunately `actix_web`'s
/// various config types do not share a common trait that has the `error_handler` method.
pub fn get_json_config() -> JsonConfig {
    JsonConfig::default().error_handler(actix_error_handler)
}

/// Returns the [`PathConfig`] to use for our service.
///
/// This config will register a custom error handler that will handle deserialization errors
/// using our [`ResponseError` impl](Error#impl-ResponseError-for-Error).
///
/// # Notes
///
/// This function cannot be generic over the config type, because unfortunately `actix_web`'s
/// various config types do not share a common trait that has the `error_handler` method.
pub fn get_path_config() -> PathConfig {
    PathConfig::default().error_handler(actix_error_handler)
}
