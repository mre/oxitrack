FROM docker.io/library/rust:latest AS builder
ENV SQLX_OFFLINE=true
RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools
RUN cargo install oxitraffic --locked --target x86_64-unknown-linux-musl

FROM docker.io/library/alpine:latest
EXPOSE 80
ENV OXITRAFFIC_DATA_DIR=/volumes/data
COPY --from=builder /usr/local/cargo/bin/oxitraffic /usr/local/bin/oxitraffic
CMD ["oxitraffic"]
