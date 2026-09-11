FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --release -p sdkwork-api-invoice-standalone-gateway

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /src/target/release/sdkwork-api-invoice-standalone-gateway /usr/local/bin/sdkwork-api-invoice-standalone-gateway
ENV INVOICE_API_BIND=0.0.0.0:18098
EXPOSE 18098
CMD ["sdkwork-api-invoice-standalone-gateway"]
