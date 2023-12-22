//! Service used to load and save pokemons. Used by the Pokedex REST API.

use std::cmp::min;

use diesel::{delete, insert_into, update, NotFound, QueryDsl};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::RunQueryDsl;
use serde::{Deserialize, Serialize};
use utoipa::ToResponse;

use crate::db::{Pool, PooledConnection};
use crate::error::QueryContext;
use crate::helpers::db::paginate::Paginate;
use crate::models::pokemon::{CreatePokemon, PatchPokemon, Pokemon, UpdatePokemon};
use crate::schema::pokemons::all_columns;

/// Service implementation for [`Pokemon`] entities.
///
/// This type contains the actual business logic to fetch/save pokemons from the database.
/// It will be used by the [pokemons REST API endpoint implementations](crate::api::v1::pokemons)
/// to handle operations regarding [`Pokemon`] entities.
#[derive(Clone)]
pub struct Service {
    pool: Pool,
}

impl Service {
    /// Max number of pokemons that can be fetched per page when [listing](Service::get_pokemons).
    pub const MAX_PAGE_SIZE: i64 = 100;

    /// Creates a new pokemon service using the provided database connection [`Pool`].
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Fetches [`Pokemon`]s from the database in a paginated way.
    ///
    /// See [`PokemonsPage`] for details on the returned data.
    pub async fn get_pokemons(&self, page: i64, page_size: i64) -> crate::Result<PokemonsPage> {
        use crate::schema::pokemons::dsl::*;

        let mut connection = self.get_pooled_connection().await?;

        // Performing a paginated query has an issue: if the query returns no results (perhaps
        // because caller asked for a page that is farther than those that exist), we can't get
        // a total_pages count, so the reported total_pages will be 0. To go around this, if
        // we get 0 results from our query, we'll perform a COUNT(*) query to get the total
        // number of entries and then calculate the total_pages manually. To have an accurate
        // result, we'll do this in a transaction with REPEATABLE READ isolation level so that
        // both queries see the same data.
        let (paged_pokemons, total_pages) = connection
            .build_transaction()
            .read_only()
            .repeatable_read()
            .run(|connection| {
                async move {
                    let paged_query_result = pokemons
                        .order(id)
                        .select(all_columns)
                        .paginate(page, min(page_size, Self::MAX_PAGE_SIZE))
                        .load_and_count_pages::<Pokemon, _>(connection)
                        .await;

                    match paged_query_result {
                        Ok((_, 0)) => {
                            let pokemon_count: i64 =
                                pokemons.count().get_result(connection).await?;
                            let total_pages =
                                (pokemon_count as f64 / page_size as f64).ceil() as i64;
                            Ok((vec![], total_pages))
                        },
                        paged_query_result => paged_query_result,
                    }
                }
                .scope_boxed()
            })
            .await
            .with_query_context(|| {
                format!("failed to load pokemons at page {} (page_size: {})", page, page_size)
            })?;

        Ok(PokemonsPage { pokemons: paged_pokemons, page, page_size, total_pages })
    }

    /// Returns the [`Pokemon`] with the given ID from the database.
    pub async fn get_pokemon(&self, pokemon_id: i64) -> crate::Result<Pokemon> {
        use crate::schema::pokemons::dsl::*;

        let mut connection = self.get_pooled_connection().await?;

        pokemons
            .find(pokemon_id)
            .first(&mut connection)
            .await
            .with_query_context(|| format!("failed to fetch pokemon with id {}", pokemon_id))
    }

    /// Creates a new [`Pokemon`] and adds it to the database.
    pub async fn create_pokemon(&self, new_pokemon: &CreatePokemon) -> crate::Result<Pokemon> {
        use crate::schema::pokemons::dsl::*;

        let mut connection = self.get_pooled_connection().await?;

        insert_into(pokemons)
            .values(new_pokemon)
            .get_result(&mut connection)
            .await
            .with_query_context(|| "failed to insert new pokemon")
    }

    /// Updates the [`Pokemon`] in the database with the given ID.
    ///
    /// This method overwrites the given pokemon completely; to update certain fields only,
    /// use [`patch_pokemon`](Service::patch_pokemon) instead.
    pub async fn update_pokemon(
        &self,
        pokemon_id: i64,
        pokemon_update: &UpdatePokemon,
    ) -> crate::Result<Pokemon> {
        use crate::schema::pokemons::dsl::*;

        let mut connection = self.get_pooled_connection().await?;

        update(pokemons.find(pokemon_id))
            .set(pokemon_update)
            .get_result(&mut connection)
            .await
            .with_query_context(|| format!("failed to update pokemon {}", pokemon_id))
    }

    /// Updates the [`Pokemon`] in the database with the given ID.
    ///
    /// This method only overwrites the fields that are specified (e.g. not set to `None`); to
    /// overwrite all fields, use [`update_pokemon`](Service::update_pokemon) instead.
    pub async fn patch_pokemon(
        &self,
        pokemon_id: i64,
        pokemon_patch: &PatchPokemon,
    ) -> crate::Result<Pokemon> {
        use crate::schema::pokemons::dsl::*;

        let mut connection = self.get_pooled_connection().await?;

        update(pokemons.find(pokemon_id))
            .set(pokemon_patch)
            .get_result(&mut connection)
            .await
            .with_query_context(|| format!("failed to patch pokemon {}", pokemon_id))
    }

    /// Deletes the pokemon with the given ID from the database.
    pub async fn delete_pokemon(&self, pokemon_id: i64) -> crate::Result<()> {
        use crate::schema::pokemons::dsl::*;

        let mut connection = self.get_pooled_connection().await?;

        delete(pokemons.find(pokemon_id))
            .execute(&mut connection)
            .await
            .and_then(|deleted_count| if deleted_count > 0 { Ok(()) } else { Err(NotFound) })
            .with_query_context(|| format!("failed to delete pokemon {}", pokemon_id))
    }

    /// Returns a [`PooledConnection`] from our internal database connection pool.
    ///
    /// The connection can then be used to perform DB queries.
    async fn get_pooled_connection(&self) -> crate::Result<PooledConnection> {
        Ok(self.pool.get().await?)
    }
}

#[cfg_attr(
    doc,
    doc = r"
        A page of [`Pokemon`]s, as returned by [`Service::get_pokemons`].

        Contains the list of [`Pokemon`]s in the page as well as paging information.
    "
)]
#[cfg_attr(not(doc), doc = "A page of Pokemons")]
#[derive(Debug, Serialize, Deserialize, ToResponse)]
#[response(example = json!({
    "pokemons": [
        {
            "id": 0,
            "number": 1,
            "name": "Bulbasaur",
            "type_1": "Grass",
            "type_2": "Poison",
            "total": 318,
            "hp": 45,
            "attack": 49,
            "defense": 49,
            "sp_atk": 65,
            "sp_def": 65,
            "speed": 45,
            "generation": 1,
            "legendary": false
        }
    ],
    "page": 1,
    "page_size": 10,
    "total_pages": 1
}))]
pub struct PokemonsPage {
    /// The Pokemons in the page
    pub pokemons: Vec<Pokemon>,

    /// Current page number (1-based)
    pub page: i64,

    /// Page size used when query was performed
    pub page_size: i64,

    /// Total number of pages available
    pub total_pages: i64,
}
