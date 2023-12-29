//! Implementation of a pagination helper for [`diesel`].
//!
//! The code in this module has been adapted from an [example](https://github.com/diesel-rs/diesel/blob/2.1.x/examples/postgres/advanced-blog-cli/src/pagination.rs)
//! in the [`diesel` repository](https://github.com/diesel-rs/diesel).

use diesel::QueryResult;
use diesel_async::methods::LoadQuery;
use diesel_async::AsyncConnection;

use crate::helpers::db::paginate::detail::InnerPaginated;

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
    /// let page = 1;
    /// let page_size = 10;
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
        Paginated::new(self, page_size, (page - 1) * page_size)
    }
}

/// Helper that allows the use of [`load_and_count_pages`](Paginated::load_and_count_pages).
///
/// See [`paginate`](Paginate::paginate) for example usage.
#[derive(Debug, Clone, Copy)]
pub struct Paginated<T>(InnerPaginated<T>);

impl<T> Paginated<T> {
    fn new(query: T, page_size: i64, offset: i64) -> Self {
        Self(InnerPaginated::new(query, page_size, offset))
    }

    /// Performs the equivalent of [`load`](diesel_async::RunQueryDsl::load) to load a page of results.
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
        detail::RealPaginated<T>: LoadQuery<'query, Conn, (U, i64)> + 'query,
        detail::mock::MockablePaginated<T>: LoadQuery<'query, Conn, (U, i64)> + 'query,
    {
        self.0.load_and_count_pages(conn).await
    }
}

/// Sets a global "error producer" for [`Paginated`] mocks.
///
/// When an error producer is set, [`load_and_count_pages`](Paginated::load_and_count_pages) will use that
/// error producer to fetch an error and return it instead of performing the actual load operation.
///
/// This function should be called in tests **only**.
///
/// # Examples
///
/// ```no_run
/// use actix_http::StatusCode;
/// use actix_web::test;
/// use diesel::result::Error as DieselError;
/// # use pokedex_rs::db::get_pool;
/// use pokedex_rs::helpers::db::paginate::{reset_mock_error_producer, set_mock_error_producer};
///
/// # async fn example() -> pokedex_rs::Result<()> {
/// # let service = test::init_service(pokedex_rs::pokedex_app!(get_pool()?)).await;
/// let result = {
///     set_mock_error_producer(Box::new(|| Some(DieselError::BrokenTransactionManager)));
///
///     let req = test::TestRequest::with_uri("/api/v1/pokemons").to_request();
///     let result = test::call_service(&service, req).await;
///
///     reset_mock_error_producer();
///
///     result
/// };
///
/// assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, result.status());
/// #
/// # Ok(())
/// # }
/// ```
pub fn set_mock_error_producer(producer: detail::mock::BoxedErrorProducer) {
    detail::mock::set_error_producer(producer);
}

/// Resets any global "error producer" set via [`set_mock_error_producer`].
///
/// This function should be called in tests **only**.
pub fn reset_mock_error_producer() {
    detail::mock::reset_error_producer();
}

mod detail {
    use diesel::backend::Backend;
    use diesel::query_builder::{AstPass, Query, QueryFragment};
    use diesel::serialize::ToSql;
    use diesel::sql_types::BigInt;
    use diesel::QueryResult;
    use diesel_async::methods::LoadQuery;
    use diesel_async::{AsyncConnection, RunQueryDsl};
    use diesel_derives::QueryId;

    use crate::helpers::db::paginate::detail::mock::MockablePaginated;

    // This is the inner implementation of `Paginated`. Unless there is a mock error producer set,
    // the real implementation will be used; otherwise, `MockablePaginated` is used instead (see below).
    #[derive(Debug, Clone, Copy)]
    pub enum InnerPaginated<T> {
        Real(RealPaginated<T>),
        Mockable(MockablePaginated<T>),
    }

    impl<T> InnerPaginated<T> {
        pub fn new(query: T, page_size: i64, offset: i64) -> Self {
            if mock::has_error_producer() {
                Self::Mockable(MockablePaginated::new(query, page_size, offset))
            } else {
                Self::Real(RealPaginated::new(query, page_size, offset))
            }
        }

        pub async fn load_and_count_pages<'query, 'conn, U, Conn>(
            self,
            conn: &'conn mut Conn,
        ) -> QueryResult<(Vec<U>, i64)>
        where
            U: Send,
            Conn: AsyncConnection,
            RealPaginated<T>: LoadQuery<'query, Conn, (U, i64)> + 'query,
            MockablePaginated<T>: LoadQuery<'query, Conn, (U, i64)> + 'query,
        {
            match self {
                Self::Real(real_paginated) => real_paginated.load_and_count_pages(conn).await,
                Self::Mockable(mockable_paginated) => {
                    mockable_paginated.load_and_count_pages(conn).await
                },
            }
        }
    }

    // This is the "real" implementation of `Paginated`, implementing the actual logic.
    //
    // This type is used everywhere except in tests where it might be overridden (see `MockablePaginated`, below).
    #[derive(Debug, Clone, Copy, QueryId)]
    pub struct RealPaginated<T> {
        query: T,
        page_size: i64,
        offset: i64,
    }

    impl<T> RealPaginated<T> {
        pub fn new(query: T, page_size: i64, offset: i64) -> Self {
            Self { query, page_size, offset }
        }

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

            let total_records = results.as_slice().first().map(|x| x.1).unwrap_or(0);
            let records = results.into_iter().map(|x| x.0).collect();
            let total_pages = (total_records as f64 / page_size as f64).ceil() as i64;

            Ok((records, total_pages))
        }
    }

    impl<T> Query for RealPaginated<T>
    where
        T: Query,
    {
        type SqlType = (T::SqlType, BigInt);
    }

    impl<T, DB> QueryFragment<DB> for RealPaginated<T>
    where
        T: QueryFragment<DB>,
        DB: Backend,
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

    pub mod mock {
        use std::sync::Mutex;

        use diesel::backend::Backend;
        use diesel::query_builder::{AstPass, Query, QueryFragment, QueryId};
        use diesel::result::Error as DieselError;
        use diesel::QueryResult;
        use diesel_async::methods::LoadQuery;
        use diesel_async::AsyncConnection;

        use crate::helpers::db::paginate::detail::RealPaginated;

        // This is a mockable implementation of `Paginated`. It wraps a real one, but can be "mocked" by
        // setting a global `PAGINATED_ERROR_PRODUCER`.
        #[derive(Debug, Clone, Copy)]
        pub struct MockablePaginated<T>(RealPaginated<T>);

        impl<T> MockablePaginated<T> {
            pub fn new(query: T, page_size: i64, offset: i64) -> Self {
                Self(RealPaginated::new(query, page_size, offset))
            }

            pub async fn load_and_count_pages<'query, 'conn, U, Conn>(
                self,
                conn: &'conn mut Conn,
            ) -> QueryResult<(Vec<U>, i64)>
            where
                U: Send,
                Conn: AsyncConnection,
                RealPaginated<T>: LoadQuery<'query, Conn, (U, i64)> + 'query,
            {
                match mocked_error() {
                    Some(mocked_error) => Err(mocked_error),
                    None => self.0.load_and_count_pages(conn).await,
                }
            }
        }

        impl<T> QueryId for MockablePaginated<T>
        where
            RealPaginated<T>: QueryId,
        {
            type QueryId = <RealPaginated<T> as QueryId>::QueryId;
        }

        impl<T> Query for MockablePaginated<T>
        where
            RealPaginated<T>: Query,
        {
            type SqlType = <RealPaginated<T> as Query>::SqlType;
        }

        impl<T, DB> QueryFragment<DB> for MockablePaginated<T>
        where
            RealPaginated<T>: QueryFragment<DB>,
            DB: Backend,
        {
            #[cfg(not(tarpaulin_include))]
            fn walk_ast<'b>(&'b self, out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
                self.0.walk_ast(out)
            }
        }

        pub type BoxedErrorProducer = Box<dyn FnMut() -> Option<DieselError> + Send + Sync>;

        static PAGINATED_ERROR_PRODUCER: Mutex<Option<BoxedErrorProducer>> = Mutex::new(None);

        pub(super) fn has_error_producer() -> bool {
            PAGINATED_ERROR_PRODUCER.lock().unwrap().is_some()
        }

        pub fn set_error_producer(producer: BoxedErrorProducer) {
            PAGINATED_ERROR_PRODUCER.lock().unwrap().replace(producer);
        }

        pub fn reset_error_producer() {
            *PAGINATED_ERROR_PRODUCER.lock().unwrap() = None;
        }

        fn mocked_error() -> Option<DieselError> {
            PAGINATED_ERROR_PRODUCER
                .lock()
                .unwrap()
                .as_mut()
                .and_then(|producer_fn| producer_fn())
        }
    }
}
