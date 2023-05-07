FROM rust:latest as build
WORKDIR /usr/src/qa-rs
COPY . .
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y ca-certificates tzdata && rm -rf /var/lib/apt/lists/*
COPY --from=build /usr/src/qa-rs/target/release/qa-rs /usr/local/bin/qa-rs

WORKDIR /usr/local/bin
CMD ["qa-rs"]
