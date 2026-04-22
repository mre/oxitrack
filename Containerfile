FROM docker.io/library/rust:slim AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --locked

FROM docker.io/library/debian:stable-slim

WORKDIR /app
EXPOSE 80
ENV OXITRACK_CONFIG_FILE=/volumes/config.toml

COPY --from=builder /app/target/release/oxitrack /usr/local/bin/oxitrack
COPY --from=builder /app/static /app/static

CMD ["/usr/local/bin/oxitrack"]