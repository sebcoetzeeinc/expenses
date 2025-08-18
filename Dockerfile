FROM expenses-builder

COPY ./src ./src
COPY Cargo.toml Cargo.lock ./

RUN cargo build --release --target-dir /app/build

FROM debian:12-slim AS runner

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/build/release/expenses ./expenses

ENTRYPOINT ["./expenses"]

CMD []

