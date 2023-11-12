//! Seeds the Pokedex database with initial pokemon data.
//!
//! See `README.md` for usage.

use std::env::current_exe;
use std::path::Path;
use std::time::Instant;

use anyhow::Context;
use cargo_metadata::camino::Utf8PathBuf;
use cargo_metadata::MetadataCommand;
use diesel::{delete, insert_into, Connection, RunQueryDsl};
use log::{info, trace};
use pokedex_rs::db::{get_db_url, SyncConnection};
use pokedex_rs::helpers::env::load_optional_dotenv;
use pokedex_rs::models::pokemon::ImportPokemon;
use simple_logger::SimpleLogger;
use validator::Validate;

/// Main program body.
///
/// Loads pokemon data from the CSV file located at `./seed/pokemon.csv` and inserts the pokemons
/// in the Pokedex database, overwriting any existing data.
fn main() -> anyhow::Result<()> {
    SimpleLogger::new()
        .init()
        .with_context(|| "failed to initialize logging facility")?;

    info!("Loading environment variables");
    load_optional_dotenv()
        .with_context(|| "failed to load `.env` file containing environment variables")?;

    info!("Starting Pokemon seeding program");
    let start_time = Instant::now();
    let seed_file_path = get_seed_file_path()?;

    info!("Loading pokemon data from {}", seed_file_path);
    let new_pokemons = load_pokemons_from_seed_file(seed_file_path)?;

    info!("Connecting to Postgres database");
    let mut connection = SyncConnection::establish(&get_db_url()?)
        .with_context(|| "failed to connect to Postgres database")?;

    info!("Dropping existing pokemons from database, if any");
    drop_existing_pokemons(&mut connection)?;

    info!("Inserting pokemons into database");
    insert_pokemons(&mut connection, &new_pokemons)?;

    let elapsed = start_time.elapsed();
    info!("Pokemon database seed done in {:.4?}s.", elapsed.as_secs_f64());

    Ok(())
}

/// Returns the path to the seed pokemon CSV file.
fn get_seed_file_path() -> anyhow::Result<Utf8PathBuf> {
    // First try looking in the directory of the current executable.
    let mut seed_file_path = current_exe()?;
    seed_file_path.pop();
    seed_file_path.push("seed");
    seed_file_path.push("pokemon.csv");
    if seed_file_path.is_file() {
        return seed_file_path
            .try_into()
            .with_context(|| "seed file path contains invalid UTF-8 characters");
    }

    // If we didn't find seed file yet, we must be in dev environment, so use cargo.
    let metadata = MetadataCommand::new()
        .exec()
        .with_context(|| "failed to get metadata to fetch workspace root")?;

    let mut seed_file_path = metadata.workspace_root;
    seed_file_path.push("seed");
    seed_file_path.push("pokemon.csv");

    Ok(seed_file_path)
}

/// Loads the pokemon data from the seed CSV file.
///
/// The data is returned as a list of [`ImportPokemon`] models.
fn load_pokemons_from_seed_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<ImportPokemon>> {
    let csv_reader = csv::Reader::from_path(path)
        .with_context(|| "failed to load CSV file containing pokemon data")?;

    let new_pokemons = csv_reader
        .into_deserialize()
        .collect::<Result<Vec<ImportPokemon>, _>>()
        .with_context(|| "failed to load pokemon data from CSV file")?
        .into_iter()
        .map(|new_pokemon| match new_pokemon.validate() {
            Ok(_) => Ok(new_pokemon),
            Err(errs) => Err(errs),
        })
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| "CSV file contained some invalid pokemon data")?;
    trace!("Found {} pokemons in the seed CSV file", new_pokemons.len());

    Ok(new_pokemons)
}

/// Clears the Pokedex database of any existing pokemons.
fn drop_existing_pokemons(connection: &mut SyncConnection) -> anyhow::Result<()> {
    use pokedex_rs::schema::pokemons::dsl::*;

    let deleted_count = delete(pokemons)
        .execute(connection)
        .with_context(|| "failed to delete existing pokemons from database")?;
    trace!("{} existing pokemons have been deleted", deleted_count);

    Ok(())
}

/// Inserts the given pokemons in the Pokedex database.
fn insert_pokemons(
    connection: &mut SyncConnection,
    new_pokemons: &Vec<ImportPokemon>,
) -> anyhow::Result<()> {
    use pokedex_rs::schema::pokemons::dsl::*;

    let inserted_count = insert_into(pokemons)
        .values(new_pokemons)
        .execute(connection)
        .with_context(|| "failed to insert pokemons into database")?;
    trace!("{} pokemons have been inserted into database", inserted_count);

    Ok(())
}
