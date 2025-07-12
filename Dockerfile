FROM rust:alpine as builder

RUN apk add --no-cache musl-dev openssl-dev

RUN rustup default stable
RUN USER=root cargo new --bin lutgen

WORKDIR /lutgen

COPY . .

RUN RUSTFLAGS="-Ctarget-feature=-crt-static" cargo build --release

FROM alpine:latest

RUN apk add --no-cache gcc
RUN adduser -D -s /bin/sh lutgen
USER lutgen

COPY --from=builder /lutgen/target/release/lutgen /lutgen

ENTRYPOINT ["/lutgen"]
