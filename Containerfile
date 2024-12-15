FROM docker.io/library/rust:slim AS builder
RUN cargo install oxitraffic --locked

FROM docker.io/library/debian:stable-slim
EXPOSE 80
ENV OXITRAFFIC_CONFIG_FILE=/volumes/config.toml
COPY --from=builder /usr/local/cargo/bin/oxitraffic /usr/local/bin/oxitraffic
CMD ["/usr/local/bin/oxitraffic"]
