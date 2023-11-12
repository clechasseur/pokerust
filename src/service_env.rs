//! Information about the service's runtime environment.

// Unfortunately, strum's `EnumIs` generates undocumented methods
#![allow(missing_docs)]

use std::env;
use std::sync::RwLock;

use once_cell::sync::Lazy;
use strum_macros::{AsRefStr, Display, EnumIs, EnumString};

/// Environment in which service is running.
///
/// This is controlled by the `POKEDEX_ENV` environment variable (see [`current`](ServiceEnv::current)).
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, AsRefStr, Display, EnumIs, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum ServiceEnv {
    /// Service is running in a development environment.
    ///
    /// When running in `Development`, additional information is included in error responses
    /// sent by API endpoints.
    Development,

    /// Service is running in a production environment.
    ///
    /// When running in `Production`, error information returned by API endpoints is kept
    /// to a minumum for security reasons.
    ///
    /// To avoid compromising security, this is the default environment value unless specified otherwise.
    #[default]
    Production,
}

impl ServiceEnv {
    /// Returns the current service runtime environment.
    ///
    /// By default, this will return [`Production`](ServiceEnv::Production) to avoid any security
    /// issue. To override this, set the `POKEDEX_ENV` environment variable to a value that
    /// corresponds to an enum variant (it is case-insensitive).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use log::info;
    /// use pokedex_rs::service_env::ServiceEnv;
    ///
    /// // Can be converted to a string for logging
    /// info!("Service is running in {}", ServiceEnv::current());
    ///
    /// // Can be compared to enable some features
    /// if ServiceEnv::current() == ServiceEnv::Production {
    ///     info!("Enabling production-only features");
    ///     // ...
    /// }
    ///
    /// // Also has helper methods to detect specific environments
    /// if ServiceEnv::current().is_development() {
    ///     info!("Enabling development-only features");
    ///     // ...
    /// }
    /// ```
    pub fn current() -> Self {
        static CURRENT_ENV: Lazy<ServiceEnv> = Lazy::new(ServiceEnv::reload);

        if cfg!(test) {
            let test_env = TEST_ENV.read().unwrap();
            if let Some(test_env) = *test_env {
                return test_env;
            }
        }

        *CURRENT_ENV
    }

    /// Returns the current runtime environment, reloading it.
    ///
    /// This method should only be used in tests, when the value needs to be fresh; regular
    /// code should instead rely on [`current`](ServiceEnv::current).
    pub fn reload() -> Self {
        env::var("POKEDEX_ENV")
            .ok()
            .and_then(|env_var| env_var.as_str().try_into().ok())
            .unwrap_or_default()
    }

    /// Calls a test function while simulating a given [`ServiceEnv`].
    ///
    /// This method is only available to tests. It will call `f` and make sure that during
    /// its execution, [`ServiceEnv::current`] will return the provided service env value.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use pokedex_rs::service_env::ServiceEnv;
    ///
    /// # async fn example() {
    /// ServiceEnv::test(ServiceEnv::Development, async {
    ///     assert_eq!(ServiceEnv::Development, ServiceEnv::current());
    /// })
    /// .await;
    /// # }
    /// ```
    #[cfg(test)]
    pub async fn test<F>(env: ServiceEnv, f: F)
    where
        F: std::future::Future<Output = ()>,
    {
        let prev_test_env;
        {
            let mut test_env = TEST_ENV.write().unwrap();
            prev_test_env = *test_env;
            *test_env = Some(env);
        }

        f.await;

        let mut test_env = TEST_ENV.write().unwrap();
        *test_env = prev_test_env;
    }
}

/// Static test environment. Used for testing values of [`ServiceEnv::current`].
///
/// See [`ServiceEnv::test`] for details.
static TEST_ENV: RwLock<Option<ServiceEnv>> = RwLock::new(None);

#[cfg(test)]
mod tests {
    mod service_env_enum {
        use std::env;

        use serial_test::file_serial;

        use crate::service_env::ServiceEnv;

        #[test]
        #[file_serial(pokedex_env)]
        fn test_current_is_cached() {
            let current = ServiceEnv::current();

            if current.is_development() {
                env::remove_var("POKEDEX_ENV");
            } else {
                env::set_var("POKEDEX_ENV", ServiceEnv::Development.to_string());
            }

            assert_eq!(current, ServiceEnv::current());
        }

        #[test]
        #[file_serial(pokedex_env)]
        fn test_default_value() {
            env::remove_var("POKEDEX_ENV");

            assert_eq!(ServiceEnv::Production, ServiceEnv::reload());
        }

        #[test]
        #[file_serial(pokedex_env)]
        fn test_from_env() {
            env::set_var("POKEDEX_ENV", ServiceEnv::Development.to_string());

            assert_eq!(ServiceEnv::Development, ServiceEnv::reload());
        }

        #[test]
        #[file_serial(pokedex_env)]
        fn test_case_insensitive() {
            env::set_var("POKEDEX_ENV", ServiceEnv::Development.to_string().to_ascii_uppercase());

            assert_eq!(ServiceEnv::Development, ServiceEnv::reload());
        }

        #[test]
        #[file_serial(pokedex_env)]
        fn test_invalid_value_uses_default() {
            env::set_var("POKEDEX_ENV", "SomeEnvironmentThatDoesNotExist");

            assert_eq!(ServiceEnv::Production, ServiceEnv::reload());
        }

        #[actix_web::test]
        #[file_serial(pokedex_env)]
        async fn test_test_wrapper() {
            let new_env = match ServiceEnv::current() {
                ServiceEnv::Development => ServiceEnv::Production,
                ServiceEnv::Production => ServiceEnv::Development,
            };

            ServiceEnv::test(new_env, async {
                assert_eq!(new_env, ServiceEnv::current());
            })
            .await;
        }
    }
}
