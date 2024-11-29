FROM rust:latest AS builder

WORKDIR /app

COPY . .

RUN cargo build

FROM registry.access.redhat.com/ubi9/ubi-micro:9.4-15
# FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/debug/prostor /app/prostor
COPY --from=builder /app/config.yml /app/config.yml

EXPOSE 8000

CMD ["./prostor"]
