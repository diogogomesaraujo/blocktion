FROM rust:latest

WORKDIR /app

COPY src ./src
COPY Cargo.toml .
COPY Cargo.lock .
COPY config ./config

ARG MODE
ARG ARGS

RUN cargo build --release
ENTRYPOINT cargo run --bin $MODE --release -- $ARGS
