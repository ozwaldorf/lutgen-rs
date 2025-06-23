FROM rust:alpine as builder

# Dependencies
RUN apk add --no-cache musl-dev mold openssl-dev

RUN rustup default stable
RUN USER=root cargo new --bin lutgen

WORKDIR /lutgen

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY palettes ./palettes
COPY benches ./benches

# Build
RUN RUSTFLAGS="-Ctarget-feature=-crt-static" cargo build --release

# STAGE binary
FROM alpine:latest

RUN apk add --no-cache gcc

COPY --from=builder /lutgen/target/release/lutgen /lutgen

ENTRYPOINT ["/lutgen"]
