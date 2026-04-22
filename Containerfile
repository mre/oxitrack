FROM docker.io/library/rust:slim AS builder
RUN cargo install oxytrack --locked

FROM docker.io/library/debian:stable-slim
EXPOSE 80
ENV OXYTRACK_CONFIG_FILE=/volumes/config.toml
COPY --from=builder /usr/local/cargo/bin/oxytrack /usr/local/bin/oxytrack
CMD ["/usr/local/bin/oxytrack"]
