//! Runs the pokedex DB migrations.
//!
//! Similar to running `diesel migration run`, but without the need to install the diesel CLI.

use std::env;
use std::sync::OnceLock;
use std::time::Instant;

use anyhow::{anyhow, Context};
use diesel::migration::MigrationSource;
use diesel::{Connection, ConnectionError};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use log::{info, trace};
use pokedex_rs::db::{get_db_url, Backend, SyncConnection};
use pokedex_rs::helpers::env::load_optional_dotenv;
use regex::Regex;
use simple_logger::SimpleLogger;

/// Container of migrations to apply, embedded in our executable.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Main program body.
///
/// Reads migrations and applies them to the pokedex DB as required.
fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .init()
        .with_context(|| "failed to initialize logging facility")?;

    info!("Loading environment variables");
    load_optional_dotenv()
        .with_context(|| "failed to load `.env` file containing environment variables")?;

    info!("Starting Pokedex migration run");
    let start_time = Instant::now();

    info!("Preparing migration targets");
    let db_url = get_db_url().with_context(|| "failed to get DB URL")?;
    let test_db_url = get_test_db_url(&db_url);
    let migration_targets = vec![db_url, test_db_url];

    migration_targets
        .into_iter()
        .map(|db_url| {
            info!("Applying migrations to database `{}`", filter_db_url(&db_url)?);
            apply_migrations(&db_url, MIGRATIONS)
        })
        .find(|result| result.is_err())
        .unwrap_or(Ok(()))?;

    let elapsed = start_time.elapsed();
    info!("Migrations applied in {:.4?}s.", elapsed.as_secs_f64());

    Ok(())
}

/// Filters the user/password from a DB URL so we can log it.
fn filter_db_url(db_url: &str) -> anyhow::Result<String> {
    static FILTER: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

    let filter = FILTER
        .get_or_init(|| {
            Regex::new(
                "postgres://(?:[^:@]+(?::[^@]+)?@)?(?<host>[^:]+)(?<port>:\\d+)?/(?<db>\\w+)",
            )
        })
        .as_ref()
        .map_err(|err| err.clone())
        .with_context(|| "failed to compile DB URL filter")?;
    Ok(filter
        .replace_all(db_url, "postgres://$host$port/$db")
        .into_owned())
}

/// Returns the URL of the test DB.
fn get_test_db_url(db_url: &str) -> String {
    // Try to replace DB host with test DB host. If this succeeds, it means we're running in Docker.
    // If not, it means we're running locally, so the test DB is on the same host but on a different port.
    let mut test_db_url = db_url.replace("pokedex-db:", "pokedex-db-test:");
    if test_db_url == db_url {
        test_db_url = test_db_url.replace("5432", "5433");
    }
    test_db_url.replace("/pokedex", "/pokedex-test")
}

/// Applies DB migrations to the given database.
fn apply_migrations<S>(db_url: &str, migrations: S) -> anyhow::Result<()>
where
    S: MigrationSource<Backend>,
{
    info!("Setting environment variable to connect to DB `{}`", filter_db_url(db_url)?);
    env::set_var("DATABASE_URL", db_url);

    info!("Connecting to Postgres database");
    match SyncConnection::establish(db_url) {
        Err(ConnectionError::BadConnection(_)) => {
            info!("Could not connect to Postgres database `{}`; skipping", filter_db_url(db_url)?);

            Ok(())
        },
        Ok(mut connection) => {
            info!("Applying migrations");
            let applied_migrations = connection
                .run_pending_migrations(migrations)
                .map_err(|err| anyhow!("{}", err))
                .with_context(|| "failed to apply migrations")?;
            trace!("{} migrations applied", applied_migrations.len());

            Ok(())
        },
        Err(err) => Err(err).with_context(|| "failed to connect to Postgres database"),
    }
}
