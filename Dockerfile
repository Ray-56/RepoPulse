# ---- build stage ----
FROM rust:1.88.0 AS builder
WORKDIR /app

# 先拷贝清单以利用缓存
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests

RUN cargo build --release

# ---- runtime stage ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/repopulse /app/repopulse

# 数据卷: sqlite 存在这里
VOLUME ["/data"]

ENV RUST_LOG=info
CMD ["/app/repopulse"]