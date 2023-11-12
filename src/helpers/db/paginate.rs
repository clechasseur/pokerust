//! Implementation of a pagination helper for [`diesel`].
//!
//! The code in this module has been adapted from an [example](https://github.com/diesel-rs/diesel/blob/2.1.x/examples/postgres/advanced-blog-cli/src/pagination.rs)
//! in the [`diesel` repository](https://github.com/diesel-rs/diesel).

use diesel::backend::Backend;
use diesel::query_builder::{AstPass, Query, QueryFragment};
use diesel::serialize::ToSql;
use diesel::sql_types::BigInt;
use diesel::QueryResult;
use diesel_async::methods::LoadQuery;
use diesel_async::{AsyncConnection, RunQueryDsl};
use diesel_derives::QueryId;

/// Helper trait used to add a `paginate` method on types.
///
/// This adds the method to [`diesel`]'s query DSL and allows callers to use
/// [`load_and_count_pages`](Paginated::load_and_count_pages).
pub trait Paginate: Sized {
    /// Paginates the current [`diesel` query](LoadQuery).
    ///
    /// Allows the later use of [`load_and_count_pages`](Paginated::load_and_count_pages) to load
    /// a page of results as well as the total number of pages available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use diesel::QueryDsl;
    /// # use pokedex_rs::db::get_pool;
    /// use pokedex_rs::helpers::db::paginate::Paginate;
    /// use pokedex_rs::models::pokemon::Pokemon;
    /// use pokedex_rs::schema::pokemons::all_columns;
    /// use pokedex_rs::schema::pokemons::dsl::*;
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// # let pool = get_pool()?;
    /// // let pool = ...;
    /// let mut connection = pool.get().await?;
    ///
    /// let page = 1i64;
    /// let page_size = 10i64;
    ///
    /// let (paged_pokemons, total_pages) = pokemons
    ///     .order(id)
    ///     .select(all_columns)
    ///     .paginate(page, page_size)
    ///     .load_and_count_pages::<Pokemon, _>(&mut connection)
    ///     .await?;
    /// #
    /// # Ok(())
    /// # }
    /// ```
    fn paginate(self, page: i64, page_size: i64) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: i64, page_size: i64) -> Paginated<Self> {
        Paginated { query: self, page_size, offset: (page - 1) * page_size }
    }
}

/// Helper that allows the use of [`load_and_count_pages`](Paginated::load_and_count_pages).
///
/// See [`paginate`](Paginate::paginate) for example usage.
#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    page_size: i64,
    offset: i64,
}

impl<T> Paginated<T> {
    /// Performs the equivalent of [`load`](RunQueryDsl::load) to load a page of results.
    ///
    /// Also returns the total number of pages available. See [`paginate`](Paginate::paginate)
    /// for example usage.
    pub async fn load_and_count_pages<'query, 'conn, U, Conn>(
        self,
        conn: &'conn mut Conn,
    ) -> QueryResult<(Vec<U>, i64)>
    where
        U: Send,
        Conn: AsyncConnection,
        Self: LoadQuery<'query, Conn, (U, i64)> + 'query,
    {
        let page_size = self.page_size;

        let results: Vec<(U, i64)> = self.load(conn).await?;

        let total_records = results.get(0).map(|x| x.1).unwrap_or(0);
        let records = results.into_iter().map(|x| x.0).collect();
        let total_pages = (total_records as f64 / page_size as f64).ceil() as i64;

        Ok((records, total_pages))
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt);
}

impl<T, DB: Backend> QueryFragment<DB> for Paginated<T>
where
    T: QueryFragment<DB>,
    i64: ToSql<BigInt, DB>,
{
    /// Generates the SQL query needed to paginate our inner query.
    ///
    /// The resulting query will return all rows from the inner query as well as the total number
    /// of rows (that would be returned if pagination was not used). This is apparently currently
    /// impossible to implement directly through [`diesel`]'s helper functions.
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("SELECT *, COUNT(*) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.page_size)?;
        out.push_sql(" OFFSET ");
        out.push_bind_param::<BigInt, _>(&self.offset)?;
        Ok(())
    }
}
