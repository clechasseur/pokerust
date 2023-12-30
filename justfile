set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

toolchain := ''
trimmed_toolchain := trim(toolchain)

cargo := if trimmed_toolchain != "" {
    "cargo +" + trimmed_toolchain
} else {
    "cargo"
}

docker-compose-build := if trimmed_toolchain == "nightly" {
    "RUST_TAG=nightly docker compose build --build-arg RUST_TOOLCHAIN=nightly"
} else if trimmed_toolchain != "" {
    "RUST_TAG=" + trimmed_toolchain + " docker compose build --build-arg RUST_VERSION=" + trimmed_toolchain
} else {
    "docker compose build"
}
docker-compose-run := if trimmed_toolchain == "nightly" {
  "RUST_TAG=nightly docker compose run --env RUST_TOOLCHAIN=nightly"
} else if trimmed_toolchain != "" {
  "RUST_TAG=" + trimmed_toolchain + " docker compose run --env RUST_VERSION=" + trimmed_toolchain
} else {
  "docker compose run"
}

default:
    @just --list

tidy: clippy fmt

clippy:
    {{cargo}} clippy --workspace --all-targets --all-features -- -D warnings

fmt:
    cargo +nightly fmt --all

check:
    {{cargo}} check --workspace --all-targets --all-features

build *extra_args:
    {{cargo}} build --workspace --all-targets --all-features {{extra_args}}

test *extra_args:
    {{cargo}} test --workspace --all-features {{extra_args}}

tarpaulin *extra_args:
    {{cargo}} tarpaulin --target-dir target-tarpaulin {{extra_args}}

pre-msrv:
    mv Cargo.toml Cargo.toml.bak
    mv Cargo.lock Cargo.lock.bak
    mv Cargo.toml.msrv Cargo.toml
    mv Cargo.lock.msrv Cargo.lock

post-msrv:
    mv Cargo.toml Cargo.toml.msrv
    mv Cargo.lock Cargo.lock.msrv
    mv Cargo.toml.bak Cargo.toml
    mv Cargo.lock.bak Cargo.lock

msrv:
    {{ if path_exists("Cargo.lock.msrv") == "true" { `just pre-msrv` } else { ` ` } }}
    cargo msrv -- cargo check --workspace --lib --bins --all-features
    {{ if path_exists("Cargo.lock.bak") == "true" { `just post-msrv` } else { ` ` } }}

doc $RUSTDOCFLAGS="-D warnings":
    {{cargo}} doc {{ if env('CI', '') != '' { '--no-deps' } else { '--open' } }} --workspace --all-features

doc-coverage $RUSTDOCFLAGS="-Z unstable-options --show-coverage":
    cargo +nightly doc --no-deps --workspace --all-features

test-package:
    {{cargo}} publish --dry-run

migrate:
    cargo run --bin run_migrations

seed:
    cargo run --bin seed_db

serve:
    {{cargo}} run

db command:
    docker compose {{ if command == "up" { 'up -d' } else { command } }}

docker-build *extra_args:
    {{docker-compose-build}} {{extra_args}} pokedex
    -docker rmi $(docker images -f "dangling=true" -q)

docker-migrate *extra_args:
    {{docker-compose-run}} {{extra_args}} --rm pokedex-migrate

docker-seed *extra_args:
    {{docker-compose-run}} {{extra_args}} --rm pokedex-seed

docker-serve *extra_args:
    {{docker-compose-run}} {{extra_args}} --service-ports --rm pokedex
