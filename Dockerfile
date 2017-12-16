FROM rustlang/rust:nightly as builder
WORKDIR /app/src
RUN USER=root cargo new --bin ht
COPY Cargo.toml Cargo.lock ./ht/

WORKDIR /app/src/ht
RUN cargo build --release

COPY ./ /app/src/ht/
RUN cargo build --release

FROM debian:stable-slim
WORKDIR /app
RUN apt update \
    && apt install -y openssl ca-certificates \
    && apt clean \
    && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

EXPOSE 80 443 10080 10443

COPY --from=builder /app/src/ht/target/release/ht /app/src/ht/gateway.tests.com.pfx ./

CMD ["/app/ht"]
