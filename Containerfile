FROM docker.io/library/rust:latest AS builder
ENV SQLX_OFFLINE=true
RUN cargo install oxitraffic

FROM docker.io/library/debian:stable-slim AS runtime
EXPOSE 80
WORKDIR app
ENV DATA_DIR_OXITRAFFIC=/volumes/data
COPY --from=builder /usr/local/cargo/bin/oxitraffic /usr/local/bin/oxitraffic
CMD ["oxitraffic"]
