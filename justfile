set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default_toolchain := ''

default: test

tidy: clippy fmt

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

fmt:
    cargo +nightly fmt --all

check toolchain=default_toolchain:
    cargo {{toolchain}} check --workspace --all-targets --all-features

build toolchain=default_toolchain:
    cargo {{toolchain}} build --workspace --all-targets --all-features

test toolchain=default_toolchain:
    cargo {{toolchain}} test --workspace --all-features

tarpaulin toolchain=default_toolchain:
    cargo {{toolchain}} tarpaulin --target-dir target-tarpaulin

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

doc:
    cargo +nightly doc --workspace --all-features --open
