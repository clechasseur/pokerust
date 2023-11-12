# Rust Pokédex API 🦀

[![CI](https://github.com/clechasseur/pokerust/actions/workflows/ci.yml/badge.svg?branch=main&event=push)](https://github.com/clechasseur/pokerust/actions/workflows/ci.yml) [![codecov](https://codecov.io/gh/clechasseur/pokerust/graph/badge.svg?token=fR0lBpOqdp)](https://codecov.io/gh/clechasseur/pokerust) [![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

This project implements a simple web application that contains a CRUD API for a Pokédex - a database of [Pokémons](https://en.wikipedia.org/wiki/Pok%C3%A9mon).
It is written in the [Rust programming language](https://www.rust-lang.org/) and is meant as an experiment in building fully-working
web applications in that language.

Rust is a systems programming language offering low resource footprint and excellent performance, but contrarily to other
systems language like C, it also includes memory safety features that makes it an attractive alternative to higher-level
languages sometimes used to build web applications, like Java, Ruby or JavaScript.

This project includes several components usually found in modern web applications, including:

- A high-performance HTTP server to handle incoming requests
- A REST API with CRUD endpoints for Pokémon entities
- Automatic serialization/deserialization of Pokémon entities as JSON
- Automatic OpenAPI documentation including Swagger UI support (and others)
- An ORM-like interface to persist Pokémons in a Postgres database
- Support for managing and applying database migrations
- Validation of incoming data at the endpoint level
- Database connection pooling to improve performance
- Configurable logging using a simple logging facade
- Error handling with separation between service errors and their HTTP response counterparts
- Support for development and production environments

## Building and running

### Supported platforms

In theory, the web application should work on all platforms supported by Rust. However, Windows does not support natively running
Linux Docker containers. The easiest way to run the app on Windows is through [WSL](https://learn.microsoft.com/en-us/windows/wsl/install).
Otherwise, the Docker-based commands will not work out-of-the-box. This includes running the local Postgres database, which
will need to be installed manually.

### Prerequisites

In order to build and run the web application and related utilities, you will need at the minimum:

- A [Docker Engine](https://docs.docker.com/engine/) installation, including Docker Compose. If you do not already have this, the easiest way to get it is to install [Docker Desktop](https://www.docker.com/products/docker-desktop/). (As mentioned, on Windows, the native Docker Desktop will not work; you can however use Docker on WSL.)
- The [just command runner](https://github.com/casey/just). This `make`-like tool is used to run project-specific commands. It can be installed in [a variety of ways](https://github.com/casey/just#installation).

### With Docker

Follow these steps to build, setup and run the service using Docker. Installing Rust locally is not required in such a case.

#### Build the image

```shell
just docker-build
```

This will create a local Docker image named `clechasseur/pokerust` that contains the web application binary and related tools.
It runs on Debian Linux. The first build can take a while as you need to download the builder Docker image and compile all
the code. (If you're not used to a compiled language, this will seem to take forever, but hang in there 😉)

#### Start the local database

```shell
just db up
```

This will launch two local containers running Postgres. One will serve as the database server when running the web
application locally; the other is for running integration tests (in case you want to do so later).

When you're done with the local database, you can stop it:

```shell
just db down
```

#### Run database migrations

```shell
just docker-migrate
```

This will execute a small tool named `run_migrations` (compiled in the above Docker image) that runs the database migrations
on the locally-running Postgres databases. It will set up the database so it is ready to be used by the web application.

#### Seed the database (optional)

```shell
just docker-seed
```

This will execute a small tool named `seed_db` (compiled in the above Docker image) that will read a [CSV file](./seed/pokemon.csv)
containing the data of 800 Pokémons and insert them in the local database. Any existing data in the DB will be wiped first.
This step is optional, but can be useful to showcase the possibilities of the REST API without having to insert many Pokémons
by hand.

#### Start the Pokédex server

```shell
just docker-serve
```

This will launch the web application server, listening locally on port 8080. You should see log entries on the console,
including one indicating that the server has been successfully started:

```
[2023-10-29T03:38:50Z INFO  pokedex_rs] Pokedex server started in Production! Listening on 0.0.0.0:8080.
```

Afterwards, the API can be accessed at [`/api/v1/pokemons`](http://localhost:8080/api/v1/pokemons). It is also possible
to see what endpoints are supported by accessing the [application's Swagger UI](http://localhost:8080/swagger-ui/).

### Locally

In order to build and run the application locally, you need the following additional components:

- A recent stable Rust toolchain (**1.68.2** is required at the minimum). If you do not have Rust installed, the easiest way to do so is via [rustup](https://www.rust-lang.org/tools/install).
- If you wish to work with the database schema, you will need the [Diesel CLI](https://github.com/diesel-rs/diesel/tree/master/diesel_cli). It is not strictly required to run database migrations however, since the locally-built `run_migrations` tool works for this.

By default, the Diesel CLI requires some local libraries for Postgres, MySQL and SQLite; however, only the Postgres support is required for Pokédex.
To install the CLI with only Postgres support, you can run:

```shell
cargo install diesel_cli --no-default-features --features "postgres"
```

You will still need the Postgres [`libpq` library](https://www.postgresql.org/download/), however.

- If you wish to run [`rustfmt`](https://github.com/rust-lang/rustfmt) or build the docs, you will need a Nightly Rust toolchain. If you do not have one, you can install one by running:

```shell
rustup toolchain install nightly
```

- If you wish to run tests locally with code coverage, you will need to install [`cargo-tarpaulin`](https://github.com/xd009642/tarpaulin). If you do not already have it, you can install it in [a variety of ways](https://github.com/xd009642/tarpaulin#installation).
- If you wish to locally determine the Minimum Supported Rust Version (MSRV) of the project, you will need to install [`cargo-msrv`](https://github.com/foresterre/cargo-msrv). If you do not already have it, you can install it in [a variety of ways](https://github.com/foresterre/cargo-msrv#install).

Please be aware that running `cargo-msrv` will install a lot of Rust toolchains locally.

#### Build the binaries

```shell
just build
```

This will build the web application and related binaries in the `target/` folder. It can take a while the first time around.

#### Start and set up the local database

Even when building the application locally, you still need a Postgres database to store the Pokédex data. The easiest way
to do so is via Docker:

```shell
just db up
```

When running on Windows (natively), this will need to be performed by hand. See the [`.env` file](./.env) and the
[test `app.rs` file](./tests/integration_helpers/app.rs) for details on what the application expects for the local
databases.

Then, just like above, you need to run migrations and (optionally) seed the database:

```shell
just migrate seed
```

As before, when you're done with the local DB, you can stop it:

```shell
just db down
```

#### Start the local server

```shell
just serve
```

This will launch the application server locally. As before, it is then accessible via [`/api/v1/pokemons`](http://localhost:8080/api/v1/pokemons).

If you check the console log, you might notice that running the server locally starts it in `Development` mode:

```
[2023-10-29T04:16:21Z INFO  pokedex_rs] Pokedex server started in Development! Listening on 127.0.0.1:8080.
```

This affects the content of error messages returned by the API (see below).

#### Run the tests

```shell
just test
```

This will run all the tests included in the project: unit tests, integration tests (which require the local test database
to be up) and [documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html) (a cool
Rust feature that allows you to embed tests in your code's documentation).

To run the tests with code coverage, use:

```shell
just tarpaulin
```

This will run all the tests and generate an HTML report named `tarpaulin-report.html` at the project root. Please be
aware that this takes much longer, as code coverage requires a special build with instrumentation (and because of an
apparent bug, the tests need to be rebuilt on every run 😔).

#### Generate the docs

```shell
just doc
```

This will generate documentation for the types and functions used in the code via [`rustdoc`](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html).
The resulting HTML will then be launched in your local web browser.

`rustdoc` generates quite nice extensive documentation. For an example output, see [the `actix-web` documentation on docs.rs](https://docs.rs/actix-web/latest/actix_web/).

#### Linting and code formatting

Rust comes with two tools to help you check your code:

- [`clippy`](https://github.com/rust-lang/rust-clippy): a Rust linter. Checks your code for common mistakes that, while not technically bugs, could be improved.
- [`rustfmt`](https://github.com/rust-lang/rustfmt): a Rust code formatter. Formats your code automatically according to predefined rules which can be [configured](./rustfmt.toml).

You can run both tools on the codebase:

```shell
just tidy
```

## Features

This section explores some of the interesting features found in the project's code.

### OpenAPI support

The application includes support for generating an OpenAPI 3.0 documentation of the API via the [`utoipa` crate](https://crates.io/crates/utoipa)
(note: the name is not a typo). When the app is running, the documentation can be accessed at [`/api-docs/openapi.json`](http://localhost:8080/api-docs/openapi.json).
The documentation can also be viewed via built-in frontends:

- [Swagger UI](https://swagger.io/tools/swagger-ui/) (via [`/swagger-ui/`](http://localhost:8080/swagger-ui/))
- [Redocly](https://redocly.com/) (via [`/redoc`](http://localhost:8080/redoc))
- [RapiDoc](https://rapidocweb.com/) (via [`/rapidoc`](http://localhost:8080/rapidoc))

### Internal errors when running in development

The application can run in two modes: `Development` or `Production`. It runs in the latter mode by default, but the mode
can be set via the `POKEDEX_ENV` environment variable. In the local repo, this is set to `Development` in the [`.env` file](./.env).

When the application runs in `Development`, any error returned by the API will contain an `internal_error` field containing
the recursive error messages that caused the error to be returned.

For example, if you were to run a query on the API while the database was down:

```shell
% just db down
docker compose down
[+] Running 3/3
 ✔ Container pokerust-pokedex-db-test-1  Removed 
 ✔ Container pokerust-pokedex-db-1       Removed 
 ✔ Network pokerust_pokedex-net          Removed

% curl http://localhost:8080/api/v1/pokemons | jq
{
  "status_code": 500,
  "error": "Internal Server Error",
  "internal_error": "database connection error\ncaused by: Error occurred while creating a new object: error connecting to server: Connection refused (os error 61)\ncaused by: error connecting to server: Connection refused (os error 61)"
}
```

For security reasons, the `internal_error` field is not returned when running in `Production`, because it might expose
security details. However, some errors (like validation errors) still include a `details` field that include more information:

```shell
% curl http://localhost:8080/api/v1/pokemons/-1 | jq
{
  "status_code": 400,
  "error": "Bad Request",
  "details": "Validation error: id: Validation error: range [{\"value\": Number(-1), \"min\": Number(0.0)}]"
}
```

### Backtrace support

Rust includes support for generating a "backtrace" (e.g. a callstack) when an error occurs. However, although the
[`Backtrace` struct](https://doc.rust-lang.org/std/backtrace/struct.Backtrace.html) is available in stable Rust, storing
one when an error occurs is only supported in Nightly Rust.

The application supports including backtraces with errors when running in `Development`. This requires two things:

- Building the application with the Nightly toolchain
- Setting the `RUST_BACKTRACE` environment variable (to `1`) to enable backtrace capture (otherwise, backtraces will be empty)

#### Testing backtrace support locally

```shell
RUST_BACKTRACE=1 just toolchain=nightly serve
```

This will build and run the app using the Nightly Rust toolchain and also enable backtrace generation. Backtrace support
can be verified in the server logs:

```
[2023-10-29T04:56:08Z INFO  pokedex_rs] Rust version used: 1.75.0-nightly
[2023-10-29T04:56:08Z INFO  pokedex_rs] Backtrace support: supported
```

The inclusion of a backtrace with errors can be tested by sending an invalid query to the API:

```shell
curl http://localhost:8080/api/v1/pokemons/-1
```

#### Testing backtrace support via Docker

Testing via Docker is a bit trickier since it requires building another Docker image for the application
using the Nightly Rust toolchain:

```shell
just toolchain=nightly docker-build
```

This should build another version of the app's Docker image (`clechasseur/pokerust:nightly`); again, this will probably
take a while the first time. To then run the application using that image and enable backtrace support, use:

```shell
just toolchain=nightly docker-serve --env POKEDEX_ENV=development --env RUST_BACKTRACE=1
```

Then, like before, you can test that backtrace support works:

```shell
curl http://localhost:8080/api/v1/pokemons/-1
```

You might notice that the returned backtrace is different from one generated locally. This is because backtrace generation
is highly platform-specific (it is even completely unsupported on some platforms).

### Logging level

The Pokédex application includes logging of various operations. As with other popular frameworks, log entries have different
_levels_: `error`, `warning`, `info`, `debug` or `trace` (see [`log::Level`](https://docs.rs/log/latest/log/enum.Level.html)).

By default, the application only displays log entries of level `info` or above. This can be configured via the `RUST_LOG`
environment variable, however. For example, to enable `trace` logging when running locally:

```shell
RUST_LOG=trace just serve
```

Or via Docker:

```shell
just docker-serve --env RUST_LOG=trace
```

Lots of other options exist to control logging output, including filtering certain entries and only enable logging for
specific modules. For more information, see the [`env_logger` crate documentation](https://docs.rs/env_logger/latest/env_logger/).

### Pagination support

The [`GET /api/v1/pokemons` endpoint](http://localhost:8080/api/v1/pokemons) supports listing Pokémons in the Pokédex
in _pages_. By default, the endpoint returns a maximum of 10 Pokémons at a time. Pagination is controlled via the `page`
and `page_size` query parameters. For example:

```shell
curl "http://localhost:8080/api/v1/pokemons?page=2&page_size=5"
```

The returned JSON will include the Pokémons, as well as information about the total number of pages available for the
specified `page_size`:

```json
{
  "pokemons": [
    ...
  ],
  "page": 2,
  "page_size": 5,
  "total_pages": 160
}
```

For performance reasons, the `page_size` is limited (currently to 100). This is currently hardcoded in the service code
(see `MAX_PAGE_SIZE` in [`service/pokemon.rs`](./src/services/pokemon.rs)).

### Documentation

Although the Pokédex application is a [bin crate](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#binaries),
the main program's code only includes what is necessary to actually start the HTTP server and listen to connections.
The body of the code is in the project's [lib crate](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#library)
(see [`lib.rs`](./src/lib.rs)) and is all documented. As mentioned before, the documentation can be generated and viewed
locally via:

```shell
just doc
```

Note that all the types and functions in the library are currently public. This would not normally be the case; it was
done this way here so that it's easier to explore the app's code via the doc.

### Integration testing

The project includes integration tests that launch a test service using `actix-web`'s testing helpers, connecting it
to a test DB hosted on a separate Postgres server. The integration tests perform requests on the actual API endpoints
and parse the data to validate the result.

In order for the tests to be able to perform validations on entity counts, etc. every test creates a new test service
and when the test concludes, the test DB is cleared of its content. This works well, but has one drawback: because
changes are actually persisted to the database by running tests, they interfere with each other so have to be _serialized_
(e.g. they cannot run in parallel).

In this sample project, the small number of tests makes this manageable. In a large project however, it would be quite a
problem. Many frameworks go around this problem by creating a database connection and starting a _transaction_ before
handing the connection to the test code, then rolls back the transaction when the test is done. Since no actual data is
ever committed to the database, tests can easily run in parallel.

I have not been able to find an easy way to implement this in the project, though. DB connections used by the API endpoints
come from a connection pool. Furthermore, some tests perform multiple requests, so they'd have to reuse the same connection
throughout so that they are in the same transaction. I'm not entirely sure what's the best way to hook into the API code
in order to achieve this. I have a feeling that using [`mockall_double`](https://crates.io/crates/mockall_double) could help,
but I haven't spent enough time thinking about it.

## Interesting crates

In Rust, external libraries are stored in units called [_crates_](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html).
Many open-source libraries are available and can be viewed and downloaded from the [crates.io registry](https://crates.io/).
It is not necessary to download them manually though; [`cargo`](https://doc.rust-lang.org/cargo/), the Rust package manager,
does so automatically when building by looking up dependencies in [`Cargo.toml`](./Cargo.toml).

Contrarily to many ecosystems, the Rust ecosystem does not include an everything-but-the-kitchen-sink framework to develop
web applications (such as Ruby's `Rails` framework, or Elixir's `Phoenix`). Instead, Rust libraries tend to be broken down
into small, reusable components that offer one or more related features. Because of this, building a web application requires
the use of several crates (much of the time spent building this experimental project was spent looking for and testing various
libraries for the different parts of the app).

The following list includes some of the more interesting crates used in the application's project. They are certainly
useful to know to build similar Rust projects (or even other Rust projects that are unrelated, since some of those are
quite ubiquitous in the Rust ecosystem).

### Web frameworks

- [`actix-web`](https://actix.rs/) : A [high-performance](https://www.techempower.com/benchmarks/#section=data-r21) web application framework
- [`tokio`](https://tokio.rs/) : A powerful asynchronous runtime for Rust

`actix-web` is probably the most popular web framework for Rust. It offers great performance while still being relatively
easy to set up and use. For other options in terms of web development, check out the [Are we web yet?](https://www.arewewebyet.org/) website.

`actix-web` uses Rust's [asynchronous programming](https://rust-lang.github.io/async-book/) support to handle requests in an
efficient manner. This requires an asynchronous runtime. Enter `tokio`, an asynchronous runtime that is designed for building
network applications. Although other asynchronous runtime implementations exist in the Rust ecosystem, `tokio` is by far the
most widely used.

### Database and persistence

- [`diesel`](https://diesel.rs/) : A flexible ORM framework
- [`deadpool`](https://crates.io/crates/deadpool) : A library for asynchronous pooling of database connections (or any type of object, really)
- [`diesel-async`](https://crates.io/crates/diesel-async) : Asynchronous wrapper around `diesel` that includes connection pooling support

`diesel` is probably the most popular ORM in the Rust ecosystem. One potential issue with `diesel`, however,  is that
it does not provide an asynchronous interface; this means that when you perform a database operation inside an async function,
the thread in the runtime thread pool is hung until the DB call returns. However, whether this is a real "issue" is debated;
asynchronous code is not in and out of itself necessarily faster.

This project uses the `diesel-async` crate to wrap calls to the database made with `diesel` so that they appear async. This
is mostly "for show", however, since `diesel` remains synchronous. Rather, the DB calls are offloaded to other threads that
are not part of the runtime thread pool. Whether this improves performance significantly would need to be benchmarked.

Alternatives to `diesel` include:

- [`sqlx`](https://github.com/launchbadge/sqlx) : A library to perform SQL queries using an asynchronous interface, albeit without a DSL
- [`ormx`](https://github.com/NyxCode/ormx) : A small library adding ORM-like features to `sqlx`, albeit in a limited way
- [`sea-orm`](https://www.sea-ql.org/SeaORM/) : A truly asynchronous ORM that uses `sqlx` under the hood

`sea-orm` looks promising and seems to require less boilerplate than `diesel`. It also supports writing database migrations
as Rust code instead of pure SQL (whether this is better is a matter of opinion, I guess).

### Validators

- [`validator`](https://github.com/Keats/validator) : Simple library to add validation support for Rust structs
- [`actix-web-validator`](https://crates.io/crates/actix-web-validator) : Adds `validator` support to `actix-web` projects, allowing automatic validation of API input

The combination of both of these crates allow code to add validations at the struct level, which will then be enforced at
the API level. Validation errors can then be converted to proper HTTP responses (e.g. `400 Bad Request`) via Actix's built-in
error handling facilities.

### OpenAPI support

- [`utoipa`](https://crates.io/crates/utoipa) : OpenAPI 3.0 documentation generator for your API (with a weird name to boot)
- [`utoipa-swagger-ui`](https://crates.io/crates/utoipa-swagger-ui) : Automatic Swagger UI support (via `utoipa`)
- [`utoipa-redoc`](https://crates.io/crates/utoipa-redoc) : Automatic Redocly support (via `utoipa`)
- [`utoipa-rapidoc`](https://crates.io/crates/utoipa-rapidoc) : Automatic RapiDoc support (via `utoipa`)

Generating OpenAPI documentation for your API endpoints is easy with `utoipa`. It allows you to use derive macros on
endpoints as well as schema and response structs to document them, then bind everything together to generate one OpenAPI
JSON documentation. This documentation can then be used to host viewers like Swagger UI via the other crates.

I did find a few quirks when using `utoipa` (namely, many derive macros use the `rustdoc` documentation to generate the
OpenAPI documentation, but sometimes you want it to be different); however, it is definitely quite the time saver.

### Serialization

- [`serde`](https://serde.rs/) : The _de facto_ **ser**eliazation / **de**serialization library of the Rust ecosystem
- [`serde_json`](https://crates.io/crates/serde_json) : JSON parser and validator that uses `serde` to allow (de)serialization of Rust types
- [`csv`](https://crates.io/crates/csv) : CSV parser that supports (de)serialization via `serde`
- [`serde_with`](https://crates.io/crates/serde_with) / [`serde-this-or-that`](https://crates.io/crates/serde-this-or-that) : Helpers for implementing `serde` support

Supporting serialization of data structures in formats like JSON is easier in languages that support some kind of reflection API.
Compiled languages can have that kind of support (see: Java), but alas this is not the case for Rust. This is however often
replaced with generic traits combined with clever proc macros.

Enter `serde`, a compile-time serialization library that showcases the power of Rust's [trait system](https://doc.rust-lang.org/book/ch10-02-traits.html)
by implementing a _format-agnostic_ serialization framework. Wait, what?

Basically, `serde` separates the _serialization_ of a type into primitive instructions from the actual implementation of
a serializer that persists the data in a specific format. To do this, `serde` offers the generic `Serialize` trait that
can be  implemented. This trait's only method, `serialize`, is passed a `Serializer` (another generic trait) and must use
the serializer to save the type's data. For instance, a struct would persist itself by serializing a struct, then each of
its named (or unnamed) fields.

Then, other crates like `serde_json` provide actual implementations of the `Serializer` trait for their specific data format.
These serializers will take the provided serialization instructions and create an appropriate output. Magic!

But the fun doesn't end there. Because `serde` comes with support for serializing most basic Rust types out-of-the-box,
the `Serialize` trait can be [_derived_](https://doc.rust-lang.org/reference/attributes/derive.html) automatically for
almost all types. For example, structs and enums support deriving `Serialize` as long as they contain fields that can
all already be serialized themselves (e.g. their types already implement `Serialize`). In practice, this means that
structs can be tagged with a simple `#[derive(Serialize)]` directly and boom, they can automatically be serialized in
all data formats for which there exists a `serde`-based library (and there are [many](https://serde.rs/#data-formats)).

(Deserialization is similarly supported via the `Deserialize` and `Deserializer` traits.)

### Logging

- [`log`](https://crates.io/crates/log) : Simple logging facade for Rust
- [`env_logger`](https://crates.io/crates/env_logger) : Console logger that can be configured via an environment variable
- [`simple_logger`](https://crates.io/crates/simple_logger) : Dead-simple console logger for simple cases

`log` is a logging facade that is heavily used in the Rust ecosystem. It includes easy macros to log data, like `info!`,
`error!`, etc. Then, to perform actual logging, you can initialize a logger implementation (like `env_logger`) at the
start of your program.

There exists multiple logger implementations; in particular, some can be used to log to files. They weren't explored in
this project, but some can be found in the [`log` crate documentation](https://docs.rs/log/latest/log/#available-logging-implementations).

### Error handling

- [`thiserror`](https://crates.io/crates/thiserror) : Useful derive macro to ease implementation of error types
- [`anyhow`](https://crates.io/crates/anyhow) : A type-erased error type for easy error handling in applications

For those used to handle errors through exceptions, Rust's error handling capabilities might feel weird at first (they
are more akin to Go, for example).

In Rust, the basic way of handling errors is by using a type called [`Result`](https://doc.rust-lang.org/std/result/enum.Result.html).
`Result` is an [_enum_](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html) - a Rust concept that is similar
in aspect to enums in other languages like Java or C++, but are actually more powerful: in Rust, each enum variant can
optionally contain additional data and the enum "knows" which variant it is storing at any moment. Data in each enum
variant is not shared, so you can only have one variant's data members at a time (a little like C's unions).

The `Result` enum has only two variants:

- `Ok(T)` : represents a success and contains the resulting data of type `T`
- `Err(E)` : represents an error and contains error information of type `E`

When a function is fallible, it usually returns a `Result` that can be used to determine if the call succeeded. If the
function returns `Err`, then an error occurred, and it must be handled. Because `Result` is generic but strongly-typed,
errors can be bubbled up, but their type will be clearly identified.

Rust also includes a trait called [`Error`](https://doc.rust-lang.org/std/error/trait.Error.html) that is usually used
for error types (although it is not required). The goal of this trait is to be able to fetch the "source" of the error -
the underlying error that is the root cause. In many languages, this is actually called `cause`.

For external libraries, it is common to define a custom error type that implements `Error` and can be used to represent
the different types of errors that can be returned by the library (often through an enum). This is where the `thiserror`
crate comes in: it offers a derive macro to automatically implement the `Error` trait (and some related traits like
`Display`) for your error type.

For applications, it is sometimes desirable to be able to handle any kind of error, because we might call many different
libraries, so creating a custom type could be unwieldy. The `anyhow` crate can be used for this: its [`anyhow::Error`](https://docs.rs/anyhow/latest/anyhow/struct.Error.html)
type can be used to store any source error, as long as it implements the standard library's `Error` trait. This makes
it easier to add proper error handling at the application level.

Rust's error handling design means you need to think carefully about how you handle errors in your code: when an error
occurs in a deeper layer, should you bubble it up as-is? Should you wrap it in a more friendly error type to add context?
Maybe you can simply compensate via other means? Although this is something that should be present in all applications,
Rust's reliance on an actual `Result` type that is returned explicitly instead of via exceptions that can easily propagate
through layers unchecked means you are forced to think it through. This can feel a little daunting at first, but it could
be argued that the resulting API for your library will be more solid.

## Contributing

Although this project is meant as an example only, if you want to discuss it, feel free to open a Discussion (or an
Issue if you find a bug somewhere). Also see [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md).
