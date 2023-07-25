FROM docker.io/library/rust:latest AS chef
RUN cargo install cargo-chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin oxitraffic

FROM docker.io/library/debian:stable-slim AS runtime
EXPOSE 80
WORKDIR app
ENV OXITRAFFIC_DATA_DIR=/volumes/data
COPY --from=builder /app/target/release/oxitraffic /usr/local/bin/oxitraffic
CMD ["oxitraffic"]
