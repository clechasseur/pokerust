//! Main Pokedex web application crate.
//!
//! This crate creates the Pokedex web application, registers the API endpoints and starts handling
//! HTTP connections. All the actual implementation is in the [private lib crate](pokedex).
//!
//! For more information, see `README.md`.

use std::env;
use std::env::VarError;

use actix_web::{web, HttpResponse, HttpServer, Responder};
use anyhow::Context;
use env_logger::Env;
use log::info;
use pokedex_rs::db::get_pool;
use pokedex_rs::helpers::env::load_optional_dotenv;
use pokedex_rs::pokedex_app;
use pokedex_rs::service_env::ServiceEnv;
use rustc_version_runtime::version;
use serde::Serialize;

/// Default HTTP port used for the Pokedex app (see [`get_http_port`]).
const DEFAULT_HTTP_PORT: u16 = 8080;

/// Main program body.
///
/// Takes care of setting up the Pokedex app, then serves its endpoints over HTTP.
#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let env_file_loaded = load_optional_dotenv()?;

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    if !env_file_loaded {
        info!(".env file not found; skipped");
    }

    info!("Creating DB connection pool");
    let pool = get_pool().with_context(|| "failed to create DB connection pool")?;

    let server_address = get_server_address()?;
    let http_port = get_http_port()?;

    info!("Starting Pokedex HTTP server");
    let server = HttpServer::new(move || pokedex_app!(pool).route("/", web::get().to(hello)))
        .bind((server_address.as_str(), http_port))
        .with_context(|| format!("failed to bind to port {}", http_port))?
        .run();

    info!(
        "Pokedex server started in {}! Listening on {}:{}.",
        ServiceEnv::current(),
        server_address,
        http_port
    );
    info!("Rust version used: {}", version());
    if ServiceEnv::current().is_development() {
        info!("Backtrace support: {}", get_backtrace_support());
    }
    Ok(server.await?)
}

/// Returns the address to bind to for the Pokedex app.
///
/// By default, the server binds to `127.0.0.1`, which works locally. When deploying in production
/// (or in a Docker container), set the `HTTP_ADDR` environment variable to `0.0.0.0`.
fn get_server_address() -> anyhow::Result<String> {
    env::var("HTTP_ADDR")
        .or_else(|err| match err {
            VarError::NotPresent => Ok("127.0.0.1".into()),
            err => Err(err),
        })
        .with_context(|| "failed to parse content of HTTP_ADDR environment variable")
}

/// Returns the HTTP port to use for the Pokedex app.
///
/// By default, the server will listen on port 8080. To override this, set the `HTTP_PORT`
/// environment variable to a different value.
fn get_http_port() -> anyhow::Result<u16> {
    env::var("HTTP_PORT")
        .map(|port| port.parse::<u16>())
        .unwrap_or(Ok(DEFAULT_HTTP_PORT))
        .with_context(|| "failed to parse content of HTTP_PORT environment variable")
}

/// Returns a string representing the status of [`Backtrace`](std::backtrace::Backtrace) support on this platform.
fn get_backtrace_support() -> &'static str {
    #[cfg(backtrace_support)]
    match std::backtrace::Backtrace::capture().status() {
        std::backtrace::BacktraceStatus::Captured => "supported",
        std::backtrace::BacktraceStatus::Disabled => "disabled",
        std::backtrace::BacktraceStatus::Unsupported => "unsupported",
        _ => "unknown (unrecognized enum value)",
    }

    #[cfg(not(backtrace_support))]
    "unsupported (not Nightly toolchain)"
}

/// Handler for the `/` endpoint. Simply returns a hello message.
///
/// Could be used as a healthcheck of sorts.
async fn hello() -> impl Responder {
    HttpResponse::Ok().json(Hello::default())
}

/// Data returned by the `/` endpoint (see [`Hello::default`]).
#[derive(Debug, Serialize)]
struct Hello {
    message: &'static str,
}

impl Default for Hello {
    /// Returns the data that will be returned by the `/` endpoint.
    ///
    /// This will simply contain a hello message.
    fn default() -> Self {
        Self { message: "Hello from Pokedex!" }
    }
}
