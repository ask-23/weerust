# ---- Build stage -------------------------------------------------------------
FROM rust:1.80-alpine AS builder
WORKDIR /app

RUN apk add --no-cache musl-dev openssl-dev pkgconfig
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release || true

COPY . .
RUN cargo build --release

# ---- Runtime stage ----------------------------------------------------------
FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=builder /app/target/release/weerust /app/weerust
ENV LISTEN_PORT=8080
USER nonroot:nonroot
EXPOSE 8080
ENTRYPOINT ["/app/weerust"]
