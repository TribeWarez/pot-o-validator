FROM rust:bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY core/ core/
COPY ai3-lib/ ai3-lib/
COPY mining/ mining/
COPY extensions/ extensions/
COPY src/ src/

RUN cargo build --release --bin pot-o-validator

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/pot-o-validator /usr/local/bin/pot-o-validator

WORKDIR /app
COPY config/ /config/

EXPOSE 8900

CMD ["pot-o-validator"]
