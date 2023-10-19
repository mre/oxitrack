FROM docker.io/library/rust:alpine AS builder
RUN apk add --no-cache musl-dev
RUN SQLX_OFFLINE=true cargo install oxitraffic --locked

FROM scratch
EXPOSE 80
ENV OXITRAFFIC_DATA_DIR=/volumes/data
COPY --from=builder /usr/local/cargo/bin/oxitraffic /usr/local/bin/oxitraffic
CMD ["/usr/local/bin/oxitraffic"]
