use std::env;
use std::sync::Once;

use diesel::{delete, Connection, RunQueryDsl};
use log::{debug, trace};
use pokedex::db::{get_db_url, get_pool, Pool, PooledConnection, SyncConnection};
use pokedex::helpers::env::load_optional_dotenv;

#[macro_export]
macro_rules! init_test_service {
    ($app_var:ident, $service_var:ident) => {
        let $app_var = $crate::integration_helpers::app::TestApp::new();
        let $service_var =
            actix_web::test::init_service(pokedex::pokedex_app!($app_var.get_pool())).await;
    };
}

pub struct TestApp {
    pool: Pool,
}

impl TestApp {
    pub fn new() -> Self {
        static INIT_TEST_DB_ENV_VAR: Once = Once::new();
        INIT_TEST_DB_ENV_VAR.call_once(|| {
            debug!("Loading environment variables");
            load_optional_dotenv().unwrap();

            debug!("Setting environment variable required to connect to test DB");
            let db_url = get_db_url()
                .unwrap()
                .replace("5432", "5433")
                .replace("/pokedex", "/pokedex-test");
            env::set_var("DATABASE_URL", db_url);
        });

        debug!("Creating test database connection pool");
        let pool = get_pool().unwrap();

        Self { pool }
    }

    pub fn get_pool(&self) -> Pool {
        self.pool.clone()
    }

    pub async fn get_pooled_connection(&self) -> PooledConnection {
        self.pool.get().await.unwrap()
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        use pokedex::schema::pokemons::dsl::*;

        debug!("Connecting to test DB to perform cleanup");
        let db_url = get_db_url().unwrap();
        let mut connection = SyncConnection::establish(&db_url).unwrap();

        debug!("Deleting all pokemons in test DB");
        let deleted_count = delete(pokemons).execute(&mut connection).unwrap();
        trace!("Cleaned up {} pokemons from test DB", deleted_count);
    }
}
