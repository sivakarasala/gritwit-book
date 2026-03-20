# Chapter 17: Docker & Deployment

A Rust binary that runs on your laptop is not a deployed application. It needs a consistent environment, reproducible builds, and a minimal runtime image that starts in seconds. This chapter builds a production-ready Docker image using a four-stage Dockerfile: Chef (base tooling) to Planner (dependency fingerprint) to Builder (compile everything) to Runtime (minimal Debian with just the binary). The result is a container that caches dependency builds, compiles without a live database, and produces an image under 150 MB.

The spotlight concept is **multi-stage builds and build optimization** — how Docker layer caching works, why cargo-chef exists, what `SQLX_OFFLINE=true` does, and how to install system dependencies like Dart Sass in a cross-platform way. These are not Docker-specific ideas; they are build system principles that apply to any compiled language.

By the end of this chapter, you will have:

- A four-stage Dockerfile: Chef, Planner, Builder, Runtime
- cargo-chef integration for dependency caching (rebuild only when `Cargo.toml` or `Cargo.lock` changes)
- `SQLX_OFFLINE=true` for building without a live PostgreSQL connection
- Dart Sass installed from a release archive with multi-architecture support
- WASM target compilation for the Leptos hydrate feature
- A minimal runtime image with only `openssl`, `ca-certificates`, and the compiled binary
- Environment variables for production configuration overrides

---

## Spotlight: Multi-Stage Builds & Build Optimization

### Why multi-stage builds?

A Rust compilation environment is large. The Rust toolchain, LLVM, system libraries, and the entire `target/` directory can exceed 10 GB. Shipping all of that to production is wasteful and insecure — every unnecessary binary in the image is an attack surface.

Multi-stage builds solve this by using separate Docker images for building and running:

```dockerfile
FROM rust:latest AS builder
# ... install tools, compile ...

FROM debian:slim AS runtime
COPY --from=builder /app/binary /app/binary
CMD ["/app/binary"]
```

The `builder` stage has everything needed to compile. The `runtime` stage has only what is needed to run. Docker discards the builder stage from the final image — it exists only during the build process. The result: a runtime image that contains the binary, its dynamic library dependencies, and nothing else.

> **Coming from JS?** Node.js Docker images face a similar problem — `node_modules` can be hundreds of megabytes. Multi-stage builds copy only `node_modules` production dependencies to the runtime image. Rust's advantage: the compiled binary has no `node_modules` equivalent. Once compiled, the source code, Cargo registry, and build artifacts are irrelevant. The binary is self-contained (with dynamic linking to system libraries like OpenSSL).

### The Rust compilation problem

Rust compilation is slow. A clean build of GrindIt takes several minutes because it compiles hundreds of dependencies. Docker rebuilds layers when their inputs change — if you `COPY . .` and then `cargo build`, any source file change triggers a full recompile of all dependencies.

The solution has two parts:

1. **Layer ordering** — copy dependency manifests first, build dependencies, then copy source code. Docker caches the dependency layer as long as `Cargo.toml` and `Cargo.lock` do not change.

2. **cargo-chef** — a tool that extracts the dependency graph from your project and builds only the dependencies, without needing the source code. This creates a cached layer that survives source code changes.

### cargo-chef: the dependency caching tool

cargo-chef works in two phases:

```dockerfile
# Phase 1: Prepare a "recipe" — a minimal description of dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Phase 2: Cook dependencies from the recipe (source code not needed)
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Phase 3: Copy source and build (dependencies already cached)
COPY . .
RUN cargo leptos build --release
```

`cargo chef prepare` scans `Cargo.toml`, `Cargo.lock`, and the project structure to produce a `recipe.json` — a minimal file that describes what dependencies exist and their features. `cargo chef cook` downloads and compiles all dependencies based on this recipe, without needing the actual source files.

The key insight: `recipe.json` changes only when dependencies change (adding/removing crates, changing versions, or changing features). Source code changes do not affect it. Docker caches the `cargo chef cook` layer, and subsequent builds skip dependency compilation entirely.

> **Coming from Go?** Go modules have a similar trick: `COPY go.mod go.sum ./ && go mod download` caches dependency downloads. But Go compilation is fast enough that rebuilding the application binary is seconds, not minutes. Rust's heavy compile times make dependency caching essential — without cargo-chef, a one-line source change triggers a multi-minute rebuild.

### SQLX_OFFLINE: building without a database

SQLx validates SQL queries at compile time by connecting to a real database. This is powerful for development — a typo in a column name is a compile error, not a runtime error. But Docker builds happen in isolated environments without database access.

`SQLX_OFFLINE=true` tells SQLx to use cached query metadata instead of connecting to a database. The cached metadata lives in the `.sqlx/` directory:

```
.sqlx/
├── query-abc123.json    # Cached metadata for each SQL query
├── query-def456.json
└── ...
```

Each JSON file contains the query text, parameter types, and return column types. SQLx generated these files during development (when `DATABASE_URL` was available) using `cargo sqlx prepare`. During the Docker build, SQLx reads these files instead of connecting to PostgreSQL.

You must commit the `.sqlx/` directory to version control. If you add a new SQL query or change an existing one, run `cargo sqlx prepare` locally to regenerate the cache, then commit the updated files.

### Dart Sass: SCSS compilation in Docker

GrindIt uses SCSS for styling. cargo-leptos compiles SCSS to CSS during the build, but it needs the `sass` binary available on `$PATH`. Dart Sass is the reference implementation — it is distributed as a standalone binary (no Node.js required).

The installation handles multi-architecture support:

```dockerfile
ARG TARGETARCH=amd64
ARG DART_SASS_VERSION=1.83.4
RUN set -eux; \
    case "${TARGETARCH}" in \
        amd64) SASS_ARCH="x64" ;; \
        arm64) SASS_ARCH="arm64" ;; \
        *) SASS_ARCH="x64" ;; \
    esac; \
    curl -fsSL "https://github.com/sass/dart-sass/releases/download/${DART_SASS_VERSION}/dart-sass-${DART_SASS_VERSION}-linux-${SASS_ARCH}.tar.gz" \
    | tar -xz -C /usr/local; \
    ln -sf /usr/local/dart-sass/sass /usr/local/bin/sass
```

`TARGETARCH` is a Docker build argument that reflects the target platform (amd64 for x86_64, arm64 for Apple Silicon or AWS Graviton). The `case` statement maps Docker's architecture names to Dart Sass's release naming convention. The `ln -sf` creates a symlink so `sass` is available on `$PATH`.

---

## The Four-Stage Dockerfile

### Stage 1: Chef — Base image with all build tools

```dockerfile
FROM rustlang/rust:nightly-trixie AS chef

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends \
       lld clang pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

RUN curl -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash

RUN cargo binstall cargo-chef -y
RUN cargo binstall cargo-leptos -y
```

This stage installs everything needed to compile Rust code:

- **`rustlang/rust:nightly-trixie`** — the Rust nightly toolchain on Debian Trixie. Leptos 0.8 requires nightly for some features.
- **`lld`** — a faster linker than the default `ld`. Reduces link time significantly for large Rust projects.
- **`clang`** — needed by some `*-sys` crates that compile C code (like `ring` for cryptography).
- **`pkg-config` and `libssl-dev`** — required by the `openssl-sys` crate, which many HTTP and TLS libraries depend on.
- **`wasm32-unknown-unknown`** — the WASM compilation target for the Leptos hydrate feature.
- **`cargo-binstall`** — installs pre-compiled binaries of Cargo tools, avoiding the need to compile `cargo-chef` and `cargo-leptos` from source (which would add minutes to the build).

The `rm -rf /var/lib/apt/lists/*` at the end of the `apt-get` command removes the package list cache. This is a Docker best practice — the cache is only needed during installation and wastes space in the layer.

Then Dart Sass is installed (as shown in the Spotlight section above), completing the Chef stage with every tool needed for compilation.

### Stage 2: Planner — Generate dependency recipe

```dockerfile
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json
```

The Planner stage copies the entire source tree and runs `cargo chef prepare`. This produces `recipe.json` — a fingerprint of the project's dependencies. The Planner stage is rebuilt whenever any file in the project changes, but that is fast (prepare takes under a second).

The important thing is that `recipe.json` is the only output of this stage. It is copied into the Builder stage, creating a clean dependency between stages.

### Stage 3: Builder — Cache dependencies, then build

```dockerfile
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --no-default-features --features ssr --recipe-path recipe.json
RUN cargo chef cook --release --target wasm32-unknown-unknown --no-default-features --features hydrate --recipe-path recipe.json

COPY . .

ENV SQLX_OFFLINE=true

RUN cargo leptos build --release -vv
```

This stage has three phases:

1. **Cook SSR dependencies** — `cargo chef cook --release --no-default-features --features ssr` compiles all dependencies needed for the server-side binary. The `--no-default-features --features ssr` flags match what `cargo leptos build` uses for the server target.

2. **Cook WASM dependencies** — `cargo chef cook --release --target wasm32-unknown-unknown --no-default-features --features hydrate` compiles all dependencies for the client-side WASM binary. This is a separate cook step because the target architecture and feature flags differ.

3. **Build the application** — `COPY . .` brings in the source code, and `cargo leptos build --release -vv` compiles both the server binary and the WASM client bundle. The `-vv` flag enables verbose output, useful for debugging build failures in CI.

The two `cargo chef cook` layers are cached independently. When you add a new dependency, both layers rebuild. When you only change source code, both layers are cached and the build starts directly at `cargo leptos build`.

`SQLX_OFFLINE=true` tells SQLx to use the cached `.sqlx/` metadata files instead of connecting to a database. Without this, the build would fail because there is no PostgreSQL instance available inside the Docker build environment.

### Stage 4: Runtime — Minimal production image

```dockerfile
FROM debian:trixie-slim AS runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gritwit /app/gritwit
COPY --from=builder /app/target/site /app/site
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/configuration /app/configuration

ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="site"
ENV APP_ENVIRONMENT="production"

EXPOSE 3000

CMD ["/app/gritwit"]
```

The runtime image starts from `debian:trixie-slim` — a minimal Debian image without development tools. It installs only two packages:

- **`openssl`** — the shared library needed by the `openssl-sys` crate at runtime. The build used `libssl-dev` (headers and static libraries); the runtime needs only the shared library.
- **`ca-certificates`** — root CA certificates for TLS verification. Without these, HTTPS requests to external services (Google OAuth, Cloudflare R2) would fail with certificate validation errors.

Four things are copied from the Builder stage:

1. **`/app/target/release/gritwit`** — the compiled server binary
2. **`/app/target/site`** — the compiled WASM bundle, CSS, and static assets
3. **`/app/Cargo.toml`** — cargo-leptos reads this at runtime for site configuration (output name, site root)
4. **`/app/configuration`** — YAML configuration files (base.yaml, production.yaml)

The environment variables set production defaults:
- `RUST_LOG="info"` — log level
- `LEPTOS_SITE_ADDR="0.0.0.0:3000"` — bind to all interfaces (required in containers)
- `LEPTOS_SITE_ROOT="site"` — where cargo-leptos looks for static assets
- `APP_ENVIRONMENT="production"` — loads `production.yaml` overlay

> **Coming from JS?** A Node.js production image typically includes the Node runtime (~150 MB), `node_modules` (varies), and your source code. The Rust runtime image has no runtime — the binary is native code. The base image is ~80 MB (slim Debian), and the binary is ~20-40 MB. The total is often smaller than `node_modules` alone.

---

## Building and Running the Image

### Build the image

```bash
docker build -t grindit:latest .
```

The first build downloads the Rust toolchain, all crate dependencies, and compiles everything. Expect 10-15 minutes. Subsequent builds with only source code changes skip the dependency layers and take 2-3 minutes.

### Run the container

```bash
docker run -p 3000:3000 \
    -e APP_DATABASE__HOST=host.docker.internal \
    -e APP_DATABASE__PASSWORD=your_password \
    -e APP_OAUTH__GOOGLE_CLIENT_ID=your_client_id \
    -e APP_OAUTH__GOOGLE_CLIENT_SECRET=your_client_secret \
    grindit:latest
```

Environment variables override the YAML configuration (Chapter 15's `APP_*` prefix with `__` nesting). `host.docker.internal` is a Docker-provided hostname that resolves to the host machine — useful for connecting to a PostgreSQL instance running on the host.

### Check the image size

```bash
docker images grindit
# REPOSITORY   TAG       IMAGE ID       SIZE
# grindit      latest    abc123         ~130 MB
```

Compare this to what a single-stage build would produce (2+ GB with the full Rust toolchain).

---

## Exercises

### Exercise 1: Write the Chef stage with all build tools

**Goal:** Create the first stage of the Dockerfile that installs the Rust nightly toolchain, system dependencies, WASM target, cargo-chef, cargo-leptos, and Dart Sass.

**Instructions:**
1. Start from `rustlang/rust:nightly-trixie`
2. Install `lld`, `clang`, `pkg-config`, `libssl-dev`, `curl` with `apt-get`
3. Add the `wasm32-unknown-unknown` target with `rustup`
4. Install `cargo-binstall`, then use it to install `cargo-chef` and `cargo-leptos`
5. Install Dart Sass from the GitHub release with multi-architecture support

<details>
<summary>Hint 1</summary>

Combine `apt-get update` and `apt-get install` in a single `RUN` command joined with `&&`. End with `rm -rf /var/lib/apt/lists/*` to reduce layer size. Use `--no-install-recommends` to avoid pulling in unnecessary packages.
</details>

<details>
<summary>Hint 2</summary>

For Dart Sass, use Docker's `TARGETARCH` build argument. Map `amd64` to `x64` and `arm64` to `arm64` using a shell `case` statement. Download the tarball with `curl -fsSL` and extract with `tar -xz -C /usr/local`. Create a symlink with `ln -sf /usr/local/dart-sass/sass /usr/local/bin/sass`.
</details>

<details>
<summary>Solution</summary>

```dockerfile
FROM rustlang/rust:nightly-trixie AS chef

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends \
       lld clang pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown

RUN curl -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash

RUN cargo binstall cargo-chef -y
RUN cargo binstall cargo-leptos -y

ARG TARGETARCH=amd64
ARG DART_SASS_VERSION=1.83.4
RUN set -eux; \
    case "${TARGETARCH}" in \
        amd64) SASS_ARCH="x64" ;; \
        arm64) SASS_ARCH="arm64" ;; \
        *) SASS_ARCH="x64" ;; \
    esac; \
    curl -fsSL "https://github.com/sass/dart-sass/releases/download/${DART_SASS_VERSION}/dart-sass-${DART_SASS_VERSION}-linux-${SASS_ARCH}.tar.gz" \
    | tar -xz -C /usr/local; \
    ln -sf /usr/local/dart-sass/sass /usr/local/bin/sass
```

Each `RUN` instruction creates a Docker layer. Separating `cargo binstall` calls into individual `RUN` instructions means Docker can cache each tool independently — if you update only the Sass version, the cargo-chef and cargo-leptos layers are cached.
</details>

### Exercise 2: Write the Planner and Builder stages with cargo-chef

**Goal:** Create the Planner stage that generates `recipe.json`, and the Builder stage that cooks dependencies separately from source code compilation.

**Instructions:**
1. Planner: copy the full source, run `cargo chef prepare`
2. Builder: copy `recipe.json` from Planner, cook SSR dependencies, cook WASM dependencies, then copy source and build with `cargo leptos build`
3. Set `SQLX_OFFLINE=true` before the final build step

<details>
<summary>Hint 1</summary>

The SSR cook uses `--no-default-features --features ssr`. The WASM cook uses `--target wasm32-unknown-unknown --no-default-features --features hydrate`. Both use `--release`.
</details>

<details>
<summary>Hint 2</summary>

Place `COPY . .` after both `cargo chef cook` commands. This is the critical ordering — source code changes invalidate the `COPY . .` layer and everything after it, but the `cook` layers remain cached.
</details>

<details>
<summary>Solution</summary>

```dockerfile
# Stage 2: Planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --no-default-features --features ssr --recipe-path recipe.json
RUN cargo chef cook --release --target wasm32-unknown-unknown --no-default-features --features hydrate --recipe-json recipe.json

COPY . .

ENV SQLX_OFFLINE=true

RUN cargo leptos build --release -vv
```

The two cook steps compile dependencies for both targets. The SSR target compiles to native x86_64/arm64 code. The WASM target compiles to WebAssembly. Both are cached independently — changing features in one does not invalidate the other.
</details>

### Exercise 3: Write the Runtime stage with minimal dependencies

**Goal:** Create a minimal production image that copies only the compiled binary, static assets, and configuration from the Builder stage.

**Instructions:**
1. Start from `debian:trixie-slim`
2. Install only `openssl` and `ca-certificates`
3. Copy the compiled binary, site directory, Cargo.toml, and configuration directory from the Builder
4. Set environment variables for production defaults
5. Expose port 3000 and set the CMD

<details>
<summary>Hint 1</summary>

The compiled binary is at `/app/target/release/gritwit`. The site assets are at `/app/target/site`. Cargo.toml is needed because cargo-leptos reads it at runtime for configuration.
</details>

<details>
<summary>Hint 2</summary>

The key environment variables: `RUST_LOG`, `LEPTOS_SITE_ADDR` (must be `0.0.0.0:3000` in containers, not `127.0.0.1`), `LEPTOS_SITE_ROOT`, and `APP_ENVIRONMENT`. Binding to `127.0.0.1` inside a container makes the service unreachable from outside.
</details>

<details>
<summary>Solution</summary>

```dockerfile
FROM debian:trixie-slim AS runtime

WORKDIR /app

RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gritwit /app/gritwit
COPY --from=builder /app/target/site /app/site
COPY --from=builder /app/Cargo.toml /app/Cargo.toml
COPY --from=builder /app/configuration /app/configuration

ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:3000"
ENV LEPTOS_SITE_ROOT="site"
ENV APP_ENVIRONMENT="production"

EXPOSE 3000

CMD ["/app/gritwit"]
```

The `EXPOSE 3000` instruction is documentation — it tells users which port the container listens on. It does not actually publish the port. The `-p 3000:3000` flag in `docker run` does the actual port mapping.
</details>

### Exercise 4: Optimize the Dockerfile for cache efficiency

**Goal:** Analyze the current Dockerfile and identify opportunities to improve layer caching and reduce image size.

**Instructions:**
1. What happens to the Docker cache when you change a single `.rs` file? Which layers rebuild?
2. What happens when you add a new crate to `Cargo.toml`? Which layers rebuild?
3. Could you use `scratch` or `alpine` instead of `debian:trixie-slim` for the runtime? What are the tradeoffs?
4. How would you add a `.dockerignore` file to reduce the build context size?

<details>
<summary>Hint 1</summary>

Trace through the Dockerfile layer by layer. The Planner stage `COPY . .` captures everything. If any file changes, the Planner re-runs. But `recipe.json` only changes when dependency metadata changes. If `recipe.json` is identical, the Builder's `cargo chef cook` layers are cached.
</details>

<details>
<summary>Hint 2</summary>

Alpine uses `musl` libc instead of `glibc`. Some Rust crates (especially those using C bindings) may not compile or behave correctly with musl. You would need to cross-compile with `x86_64-unknown-linux-musl`. The `scratch` image has no OS at all — you would need a fully statically linked binary.
</details>

<details>
<summary>Solution</summary>

**Cache behavior for a `.rs` file change:**
1. Planner: rebuilds (COPY . . changed) — fast, just `cargo chef prepare`
2. Builder: `recipe.json` is unchanged, so both `cargo chef cook` layers are CACHED
3. Builder: `COPY . .` rebuilds (source changed)
4. Builder: `cargo leptos build` rebuilds — but dependencies are already compiled, so only application code compiles
5. Runtime: rebuilds only the COPY layers

**Cache behavior for a new crate in Cargo.toml:**
1. Planner: rebuilds — `recipe.json` changes because dependency list changed
2. Builder: both `cargo chef cook` layers REBUILD — new dependencies must be compiled
3. Everything after also rebuilds

**Alpine vs Debian:**
- Alpine with musl: smaller image (~5 MB base vs ~80 MB), but musl has subtle differences from glibc (DNS resolution, locale handling, memory allocation). Some crates like `ring` require extra build flags for musl.
- Scratch: smallest possible (~0 MB base), but no shell (no `docker exec -it ... bash` for debugging), no CA certificates (must bundle them), no dynamic linker (must compile with `RUSTFLAGS="-C target-feature=+crt-static"`).
- Debian slim is the pragmatic default: large enough to include debugging tools, small enough for production.

**`.dockerignore` file:**
```
target/
.git/
.env
*.md
.github/
```

Excluding `target/` is critical — it can be gigabytes. Excluding `.git/` removes the git history. The `.env` file should never enter the Docker build context (it may contain secrets).
</details>

---

## Rust Gym: Layer Optimization Drills

### Drill 1: Analyze layer sizes

<details>
<summary>Exercise</summary>

Build the Docker image and use `docker history` to inspect each layer's size:

```bash
docker build -t grindit:latest .
docker history grindit:latest
```

Identify the largest layers. The `cargo chef cook` layers should be the largest (compiled dependencies). The `COPY --from=builder` layers should be relatively small (just the binary and assets).

If the `apt-get install` layer in the runtime stage is larger than expected, ensure you have `--no-install-recommends` and the cleanup commands (`autoremove`, `clean`, `rm -rf /var/lib/apt/lists/*`).
</details>

### Drill 2: Measure cache effectiveness

<details>
<summary>Exercise</summary>

Measure the build time with and without cache:

```bash
# Clean build (no cache)
docker build --no-cache -t grindit:latest .
# Note the time

# Change a single .rs file, rebuild
# Edit src/routes/health_check.rs (add a comment)
docker build -t grindit:latest .
# Note the time — should be much faster
```

The second build should skip the `cargo chef cook` layers and only rebuild from `COPY . .` onward. On a typical machine, this reduces build time from 10-15 minutes to 2-3 minutes.
</details>

### Drill 3: Multi-platform build

<details>
<summary>Exercise</summary>

Build for both amd64 and arm64 using Docker buildx:

```bash
docker buildx create --name multiarch --use
docker buildx build --platform linux/amd64,linux/arm64 -t grindit:latest .
```

The `TARGETARCH` build argument automatically takes the correct value for each platform. The Dart Sass `case` statement selects the right architecture-specific binary. The Rust toolchain compiles for the target platform via QEMU emulation (slow but functional) or native builders.
</details>

---

## System Design Corner: Containerization

**Interview question:** "How would you containerize a full-stack web application with server-side rendering?"

**What we just built:** A four-stage Docker image that separates build concerns (Rust toolchain, WASM compilation, SCSS compilation) from runtime concerns (binary execution, static file serving).

**Talking points:**

- **Layer caching strategy** — the most important optimization in Docker builds. Layers that change frequently (source code) should be late in the Dockerfile. Layers that change rarely (system packages, dependency compilation) should be early. cargo-chef takes this further by extracting the dependency fingerprint so that source code changes do not invalidate the dependency layer.

- **Build vs runtime separation** — the build image needs compilers, linkers, and development headers. The runtime image needs only the compiled binary and its dynamic library dependencies. Multi-stage builds enforce this separation structurally — you cannot accidentally ship the compiler to production.

- **Offline build capability** — production builds should not depend on external services. `SQLX_OFFLINE=true` removes the database dependency. Pinned dependency versions in `Cargo.lock` ensure reproducible builds. The only external dependency during build is the crate registry, which can be mitigated with a local mirror.

- **Image size tradeoffs** — Debian slim (~80 MB) vs Alpine (~5 MB) vs scratch (~0 MB). Smaller is not always better. Debian slim provides `apt-get` for installing debug tools, a shell for `docker exec`, and glibc compatibility. Alpine's musl libc introduces subtle runtime differences. Scratch eliminates debugging capability entirely. Choose based on your operational needs.

- **Security hardening** — the runtime image has no compiler, no package manager (beyond apt for debugging), and no source code. The attack surface is minimal. Further hardening: run as a non-root user (`RUN useradd -r grindit && USER grindit`), use read-only file systems, and scan the image with tools like `trivy`.

- **Configuration injection** — the image contains YAML configuration files with defaults. Secrets and environment-specific values are injected via environment variables at runtime (`docker run -e APP_DATABASE__PASSWORD=...`). This follows the twelve-factor app methodology: config in the environment, not in the image.

---

> **Design Insight: Deep Modules in Build Systems** (Ousterhout, Ch. 4)
>
> The Dockerfile presents a simple interface — `docker build -t grindit .` — but hides significant complexity inside: four stages, two compilation targets (native + WASM), SCSS compilation, SQLx offline mode, multi-architecture support, and layer caching optimization. Users of the image (DevOps, CI systems) do not need to understand any of this. They run one command and get a production-ready container. This is the definition of a deep module: simple interface, complex implementation.

---

## Summary

This chapter built the production Docker image for GrindIt:

- **Four-stage build** — Chef (tools) to Planner (dependency fingerprint) to Builder (compile) to Runtime (run). Each stage has a single responsibility, and the final image contains only the compiled artifacts.
- **cargo-chef** — generates a `recipe.json` that fingerprints the dependency graph. Docker caches the dependency compilation layer, skipping it when only source code changes. This reduces incremental builds from 10+ minutes to 2-3 minutes.
- **`SQLX_OFFLINE=true`** — enables compilation without a live database. The `.sqlx/` directory contains cached query metadata generated during development.
- **Dart Sass** — installed from a release archive with `TARGETARCH` for multi-platform support. The `sass` binary is needed during build (SCSS compilation) but not at runtime.
- **Minimal runtime** — `debian:trixie-slim` with only `openssl` and `ca-certificates`. The compiled binary, site assets, and configuration are copied from the Builder. Environment variables set production defaults.

The Docker image is the deployable unit. It contains everything needed to run GrindIt in any environment — a cloud VM, a Kubernetes pod, or a container platform like Fly.io or Railway. The next chapter automates quality checks and deployment with CI/CD pipelines.
