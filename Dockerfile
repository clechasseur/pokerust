# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.73.0
ARG RUST_TOOLCHAIN=stable
ARG APP_NAME=pokedex_rs
ARG SEED_APP_NAME=seed_db
ARG MIGRATE_APP_NAME=run_migrations

FROM rust:${RUST_VERSION}-bookworm AS build_stable
RUN echo "Building on Rust stable toolchain (${RUST_VERSION})"

FROM rustlang/rust:nightly AS build_nightly
RUN echo "Building on Rust nightly toolchain"

FROM build_${RUST_TOOLCHAIN} AS build
ARG APP_NAME
ARG SEED_APP_NAME
ARG MIGRATE_APP_NAME
WORKDIR /app

RUN --mount=type=bind,source=migrations,target=migrations \
    --mount=type=bind,source=seed,target=seed \
    --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=build.rs,target=build.rs \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=bind,source=diesel.toml,target=diesel.toml \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --bins --locked --release
cp ./target/release/$APP_NAME /bin/server
cp ./target/release/$SEED_APP_NAME /bin/seed_db
cp ./target/release/$MIGRATE_APP_NAME /bin/run_migrations
cp -r ./seed /bin/seed
EOF

FROM debian:bookworm-slim AS final
LABEL org.opencontainers.image.authors="Charles Lechasseur <shiftingbeard@gmx.com>"

RUN apt-get update && \
    apt-get install -y --no-install-recommends libpq5 && \
    rm -rf /var/lib/apt/lists/*

ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser
USER appuser

COPY --from=build /bin/server /bin/
COPY --from=build /bin/seed_db /bin/
COPY --from=build /bin/run_migrations /bin/
COPY --from=build /bin/seed/* /bin/seed/

EXPOSE 8080

CMD ["/bin/server"]
