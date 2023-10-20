FROM docker.io/library/rust:slim AS builder
RUN SQLX_OFFLINE=true cargo install oxitraffic --locked

FROM docker.io/library/debian:12-slim
EXPOSE 80
ENV OXITRAFFIC_DATA_DIR=/volumes/data
COPY --from=builder /usr/local/cargo/bin/oxitraffic /usr/local/bin/oxitraffic
CMD ["/usr/local/bin/oxitraffic"]
