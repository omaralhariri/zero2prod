FROM lukemathwalker/cargo-chef:latest-rust-1.68.2 as chef
WORKDIR /app
RUN apt update && apt install lld clang -y
FROM chef as planner
COPY . .

# compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json
FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project deps, not application
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if out deps stay the same, all our layers should be cached

# Builder stage
COPY . .
ENV SQLX_OFFLINE true

# build our project
RUN cargo build --release --bin zero2prod

# Runtime stage
FROM debian:bullseye-slim AS runtime
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
# Copy the complied binary from the builder environment to our runtime environment
COPY --from=builder /app/target/release/zero2prod zero2prod
# We need the configuration file at runtime
COPY configuration configuration
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./zero2prod"]
