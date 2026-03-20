# Chapter 17: Docker & Deployment
# 4-stage multi-stage build: Chef → Planner → Builder → Runtime
# Deep module: complex build process hidden behind `docker build .`

# Stage 1: Chef — install cargo-chef for dependency caching
FROM rustlang/rust:nightly AS chef
RUN cargo install cargo-chef cargo-leptos
WORKDIR /app

# Stage 2: Planner — analyze dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder — build dependencies (cached) then the app
FROM chef AS builder

# Install Dart Sass for SCSS compilation
ARG TARGETARCH
RUN if [ "$TARGETARCH" = "arm64" ]; then \
        SASS_URL="https://github.com/sass/dart-sass/releases/download/1.69.5/dart-sass-1.69.5-linux-arm64.tar.gz"; \
    else \
        SASS_URL="https://github.com/sass/dart-sass/releases/download/1.69.5/dart-sass-1.69.5-linux-x64.tar.gz"; \
    fi && \
    curl -fsSL "$SASS_URL" | tar -xz -C /usr/local/bin --strip-components=1 dart-sass/sass

# Cache dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build the actual application
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo leptos build --release

# Stage 4: Runtime — minimal image
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the built artifacts
COPY --from=builder /app/target/release/grindit /app/grindit
COPY --from=builder /app/target/site /app/site
COPY --from=builder /app/configuration /app/configuration

ENV APP_ENVIRONMENT=production
ENV LEPTOS_SITE_ROOT=/app/site
EXPOSE 3000

ENTRYPOINT ["/app/grindit"]
