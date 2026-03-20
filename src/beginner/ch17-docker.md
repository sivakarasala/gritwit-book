# Chapter 17: Docker & Deployment

A Rust binary that runs on your laptop is not a deployed application. It needs a consistent environment, reproducible builds, and a minimal runtime image that starts in seconds. This chapter builds a production-ready Docker image using a four-stage Dockerfile: Chef (base tooling) to Planner (dependency fingerprint) to Builder (compile everything) to Runtime (minimal Debian with just the binary). The result is a container that caches dependency builds, compiles without a live database, and produces an image under 150 MB.

The spotlight concept is **multi-stage builds and build optimization** --- how Docker layer caching works, why cargo-chef exists, what `SQLX_OFFLINE=true` does, and how to install system dependencies like Dart Sass in a cross-platform way. These are not Docker-specific ideas; they are build system principles that apply to any compiled language.

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

> **Programming Concept: What is a Container?**
>
> A container is a lightweight, self-contained package that includes everything needed to run an application: the compiled program, its dependencies, configuration files, and a minimal operating system layer. Think of it as a shipping container for software --- no matter what ship (server) carries it, the contents are the same and the container works the same way.
>
> **Docker** is the most popular tool for creating and running containers. Here is the basic flow:
>
> 1. You write a **Dockerfile** --- a recipe that describes how to build the container
> 2. `docker build` follows the recipe to create an **image** --- a snapshot of the container's filesystem
> 3. `docker run` starts a **container** from the image --- a running instance of your application
>
> Why containers instead of just copying the binary to a server?
>
> - **Consistency** --- the container runs the same way on your laptop, in CI, and in production. No "it works on my machine" problems.
> - **Isolation** --- each container has its own filesystem, network, and process space. One container crashing does not affect others.
> - **Reproducibility** --- the Dockerfile is version-controlled. Anyone can rebuild the exact same image from the same Dockerfile.
> - **Portability** --- containers run on any platform that supports Docker: Linux servers, cloud platforms (AWS, GCP, Azure), or container orchestrators like Kubernetes.

A Rust compilation environment is large. The Rust toolchain, LLVM, system libraries, and the entire `target/` directory can exceed 10 GB. Shipping all of that to production is wasteful and insecure --- every unnecessary binary in the image is an attack surface.

Multi-stage builds solve this by using separate Docker images for building and running:

```dockerfile
FROM rust:latest AS builder
# ... install tools, compile ...

FROM debian:slim AS runtime
COPY --from=builder /app/binary /app/binary
CMD ["/app/binary"]
```

The `builder` stage has everything needed to compile. The `runtime` stage has only what is needed to run. Docker discards the builder stage from the final image --- it exists only during the build process. The result: a runtime image that contains the binary, its dynamic library dependencies, and nothing else.

> **Programming Concept: What is a Docker Stage?**
>
> A Dockerfile can have multiple `FROM` instructions, each starting a new **stage**. Think of stages as separate workbenches in a workshop:
>
> - **Workbench 1 (Builder)** has all the power tools: saws, drills, sanders. You use it to build the furniture.
> - **Workbench 2 (Runtime)** is clean and minimal. You place only the finished furniture on it --- no sawdust, no tools.
>
> The `COPY --from=builder` instruction moves the finished product from one workbench to another. The tools and sawdust stay behind. In Docker terms: the Rust compiler, source code, and build artifacts stay in the builder stage. Only the compiled binary moves to the runtime stage.
>
> This is why the final image is small. It does not contain the Rust toolchain (2+ GB), the `target/` directory (potentially gigabytes), or the source code. It contains only the binary and the minimal OS libraries it needs to run.

### The Rust compilation problem

Rust compilation is slow. A clean build of GrindIt takes several minutes because it compiles hundreds of dependencies. Docker rebuilds layers when their inputs change --- if you `COPY . .` and then `cargo build`, any source file change triggers a full recompile of all dependencies.

> **Programming Concept: What is Docker Layer Caching?**
>
> A Dockerfile is a sequence of instructions (`FROM`, `RUN`, `COPY`, etc.). Docker executes each instruction and saves the result as a **layer** --- a snapshot of the filesystem at that point. When you rebuild the image, Docker checks if the inputs to each instruction have changed:
>
> - If the inputs are the same, Docker reuses the cached layer (instant)
> - If the inputs changed, Docker rebuilds that layer and all subsequent layers
>
> This is why instruction order matters. Consider two approaches:
>
> ```dockerfile
> # Approach A: slow (copies everything, any change rebuilds dependencies)
> COPY . .
> RUN cargo build --release
>
> # Approach B: fast (copies dependencies first, then source code)
> COPY Cargo.toml Cargo.lock ./
> RUN cargo build --release  # builds only dependencies
> COPY . .                   # now copy source code
> RUN cargo build --release  # rebuilds only app code (deps are cached)
> ```
>
> In Approach A, changing a single `.rs` file invalidates the `COPY . .` layer, which forces `cargo build` to recompile everything --- including all 300+ dependencies. In Approach B, changing source code only invalidates the second `COPY . .` and the second `cargo build`. The dependency compilation layer is cached.

The solution has two parts:

1. **Layer ordering** --- copy dependency manifests first, build dependencies, then copy source code. Docker caches the dependency layer as long as `Cargo.toml` and `Cargo.lock` do not change.

2. **cargo-chef** --- a tool that extracts the dependency graph from your project and builds only the dependencies, without needing the source code. This creates a cached layer that survives source code changes.

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

`cargo chef prepare` scans `Cargo.toml`, `Cargo.lock`, and the project structure to produce a `recipe.json` --- a minimal file that describes what dependencies exist and their features. `cargo chef cook` downloads and compiles all dependencies based on this recipe, without needing the actual source files.

The key insight: `recipe.json` changes only when dependencies change (adding/removing crates, changing versions, or changing features). Source code changes do not affect it. Docker caches the `cargo chef cook` layer, and subsequent builds skip dependency compilation entirely.

### SQLX_OFFLINE: building without a database

SQLx validates SQL queries at compile time by connecting to a real database. This is powerful for development --- a typo in a column name is a compile error, not a runtime error. But Docker builds happen in isolated environments without database access.

`SQLX_OFFLINE=true` tells SQLx to use cached query metadata instead of connecting to a database. The cached metadata lives in the `.sqlx/` directory:

```
.sqlx/
├── query-abc123.json    # Cached metadata for each SQL query
├── query-def456.json
└── ...
```

Each JSON file contains the query text, parameter types, and return column types. SQLx generated these files during development (when `DATABASE_URL` was available) using `cargo sqlx prepare`. During the Docker build, SQLx reads these files instead of connecting to PostgreSQL.

You must commit the `.sqlx/` directory to version control. If you add a new SQL query or change an existing one, run `cargo sqlx prepare` locally to regenerate the cache, then commit the updated files.

> **Programming Concept: Why Offline Build Capability Matters**
>
> In development, your database is running on `localhost:5432`. But during a Docker build, there is no database. The build happens inside an isolated container that has no network access to your local machine.
>
> Without `SQLX_OFFLINE=true`, the build would fail because SQLx tries to connect to the database to verify SQL queries. The offline mode solves this by caching the verification results ahead of time. It is like taking a photo of the exam answers before the test --- you already know the questions are valid, so you do not need the teacher present during the exam.
>
> This pattern is common in production builds: avoid runtime dependencies during build time. The build should be self-contained --- it should not need a database, an API key, or network access to external services.

### Dart Sass: SCSS compilation in Docker

GrindIt uses SCSS for styling. cargo-leptos compiles SCSS to CSS during the build, but it needs the `sass` binary available on `$PATH`. Dart Sass is the reference implementation --- it is distributed as a standalone binary (no Node.js required).

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

### Stage 1: Chef --- Base image with all build tools

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

This stage installs everything needed to compile Rust code. Let us walk through each piece:

- **`rustlang/rust:nightly-trixie`** --- the Rust nightly toolchain on Debian Trixie. Leptos 0.8 requires nightly for some features. Think of this as starting with a workshop that already has the basic Rust tools installed.
- **`lld`** --- a faster linker than the default `ld`. The linker is the tool that combines compiled code into a single binary. `lld` reduces link time significantly for large Rust projects.
- **`clang`** --- a C compiler needed by some Rust crates that wrap C libraries (like `ring` for cryptography). Even though we are writing Rust, some dependencies include C code that must be compiled.
- **`pkg-config` and `libssl-dev`** --- required by the `openssl-sys` crate, which many HTTP and TLS libraries depend on. `libssl-dev` provides the header files for compilation; the runtime only needs the shared library.
- **`wasm32-unknown-unknown`** --- the WASM compilation target for the Leptos hydrate feature. GrindIt compiles twice: once for the server (native code) and once for the browser (WebAssembly).
- **`cargo-binstall`** --- installs pre-compiled binaries of Cargo tools, avoiding the need to compile `cargo-chef` and `cargo-leptos` from source (which would add minutes to the build).

The `rm -rf /var/lib/apt/lists/*` at the end of the `apt-get` command removes the package list cache. This is a Docker best practice --- the cache is only needed during installation and wastes space in the layer.

Then Dart Sass is installed (as shown in the Spotlight section above), completing the Chef stage with every tool needed for compilation.

### Stage 2: Planner --- Generate dependency recipe

```dockerfile
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json
```

The Planner stage copies the entire source tree and runs `cargo chef prepare`. This produces `recipe.json` --- a fingerprint of the project's dependencies. The Planner stage is rebuilt whenever any file in the project changes, but that is fast (prepare takes under a second).

The important thing is that `recipe.json` is the only output of this stage. It is copied into the Builder stage, creating a clean dependency between stages. Think of the Planner as a shopping list maker --- it scans the recipe book (your project) and writes down just the ingredients (dependencies), not the cooking instructions (source code).

### Stage 3: Builder --- Cache dependencies, then build

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

1. **Cook SSR dependencies** --- `cargo chef cook --release --no-default-features --features ssr` compiles all dependencies needed for the server-side binary. The `--no-default-features --features ssr` flags match what `cargo leptos build` uses for the server target.

2. **Cook WASM dependencies** --- `cargo chef cook --release --target wasm32-unknown-unknown --no-default-features --features hydrate` compiles all dependencies for the client-side WASM binary. This is a separate cook step because the target architecture and feature flags differ.

3. **Build the application** --- `COPY . .` brings in the source code, and `cargo leptos build --release -vv` compiles both the server binary and the WASM client bundle. The `-vv` flag enables verbose output, useful for debugging build failures in CI.

The two `cargo chef cook` layers are cached independently. When you add a new dependency, both layers rebuild. When you only change source code, both layers are cached and the build starts directly at `cargo leptos build`.

`SQLX_OFFLINE=true` tells SQLx to use the cached `.sqlx/` metadata files instead of connecting to a database. Without this, the build would fail because there is no PostgreSQL instance available inside the Docker build environment.

### Stage 4: Runtime --- Minimal production image

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

The runtime image starts from `debian:trixie-slim` --- a minimal Debian image without development tools. It installs only two packages:

- **`openssl`** --- the shared library needed by the `openssl-sys` crate at runtime. The build used `libssl-dev` (headers and static libraries); the runtime needs only the shared library.
- **`ca-certificates`** --- root CA certificates for TLS verification. Without these, HTTPS requests to external services (Google OAuth, Cloudflare R2) would fail with certificate validation errors.

Four things are copied from the Builder stage:

1. **`/app/target/release/gritwit`** --- the compiled server binary
2. **`/app/target/site`** --- the compiled WASM bundle, CSS, and static assets
3. **`/app/Cargo.toml`** --- cargo-leptos reads this at runtime for site configuration (output name, site root)
4. **`/app/configuration`** --- YAML configuration files (base.yaml, production.yaml)

The environment variables set production defaults:
- `RUST_LOG="info"` --- log level (Chapter 15)
- `LEPTOS_SITE_ADDR="0.0.0.0:3000"` --- bind to all interfaces (required in containers)
- `LEPTOS_SITE_ROOT="site"` --- where cargo-leptos looks for static assets
- `APP_ENVIRONMENT="production"` --- loads `production.yaml` overlay (Chapter 15)

> **Programming Concept: Why 0.0.0.0 Instead of 127.0.0.1?**
>
> `127.0.0.1` (also called `localhost`) means "listen only for connections from this machine." Outside a container, that works fine --- your browser is on the same machine as the server.
>
> Inside a container, `127.0.0.1` means "listen only for connections from inside this container." But you want connections from outside the container (your browser, a load balancer). `0.0.0.0` means "listen on all network interfaces" --- including the virtual network interface that Docker uses to route traffic from the host to the container.
>
> This is a common Docker gotcha. If your containerized server listens on `127.0.0.1`, it starts successfully but is unreachable from outside. Switching to `0.0.0.0` fixes it.

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

> **Programming Concept: What Does -p 3000:3000 Mean?**
>
> The `-p` flag maps a port from the host machine to the container. The format is `host_port:container_port`. So `-p 3000:3000` means "when someone connects to port 3000 on the host, forward it to port 3000 inside the container."
>
> You could use `-p 8080:3000` to access the container on port 8080 from your browser while the container still listens on port 3000 internally. The `EXPOSE 3000` instruction in the Dockerfile is just documentation --- it does not actually publish the port. The `-p` flag does the actual mapping.

Environment variables override the YAML configuration (Chapter 15's `APP_*` prefix with `__` nesting). `host.docker.internal` is a Docker-provided hostname that resolves to the host machine --- useful for connecting to a PostgreSQL instance running on the host.

### Check the image size

```bash
docker images grindit
# REPOSITORY   TAG       IMAGE ID       SIZE
# grindit      latest    abc123         ~130 MB
```

Compare this to what a single-stage build would produce (2+ GB with the full Rust toolchain). The four-stage approach reduced the image size by over 90%.

---

## Exercises

### Exercise 1: Write the Chef stage with all build tools

**Goal:** Create the first stage of the Dockerfile that installs the Rust nightly toolchain, system dependencies, WASM target, cargo-chef, cargo-leptos, and Dart Sass.

**Instructions:**

1. Start from `rustlang/rust:nightly-trixie` and name the stage `chef` using `AS chef`
2. Set the working directory to `/app` with `WORKDIR /app`
3. Install system packages: `lld`, `clang`, `pkg-config`, `libssl-dev`, `curl` using `apt-get`. Combine the `apt-get update` and `apt-get install` in a single `RUN` command with `&&` and end with `rm -rf /var/lib/apt/lists/*` to clean up
4. Add the `wasm32-unknown-unknown` target with `rustup target add`
5. Install `cargo-binstall` (downloads pre-compiled tools), then use it to install `cargo-chef` and `cargo-leptos`
6. Install Dart Sass from the GitHub release. Use the `TARGETARCH` build argument to detect whether the build is running on amd64 (Intel/AMD) or arm64 (Apple Silicon/Graviton) and download the correct version

<details>
<summary>Hint 1</summary>

Combine `apt-get update` and `apt-get install` in a single `RUN` command joined with `&&`. End with `rm -rf /var/lib/apt/lists/*` to reduce layer size. Use `--no-install-recommends` to avoid pulling in unnecessary packages. The full pattern:

```dockerfile
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends \
       package1 package2 \
    && rm -rf /var/lib/apt/lists/*
```
</details>

<details>
<summary>Hint 2</summary>

For Dart Sass, use Docker's `TARGETARCH` build argument with `ARG TARGETARCH=amd64`. Map `amd64` to `x64` and `arm64` to `arm64` using a shell `case` statement. Download the tarball with `curl -fsSL` and extract with `tar -xz -C /usr/local`. Create a symlink with `ln -sf /usr/local/dart-sass/sass /usr/local/bin/sass` so the `sass` command is available on `$PATH`.
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

Each `RUN` instruction creates a Docker layer. Separating `cargo binstall` calls into individual `RUN` instructions means Docker can cache each tool independently --- if you update only the Sass version, the cargo-chef and cargo-leptos layers are cached.
</details>

### Exercise 2: Write the Planner and Builder stages with cargo-chef

**Goal:** Create the Planner stage that generates `recipe.json`, and the Builder stage that cooks dependencies separately from source code compilation.

**Instructions:**

1. **Planner stage:** Start from `chef` (the stage you built in Exercise 1). Copy the full source tree with `COPY . .`. Run `cargo chef prepare --recipe-path recipe.json` to generate the dependency fingerprint.
2. **Builder stage:** Start from `chef` again. Copy only `recipe.json` from the Planner with `COPY --from=planner /app/recipe.json recipe.json`. Run two cook commands --- one for SSR dependencies and one for WASM dependencies. Then copy the full source tree and build with `cargo leptos build --release -vv`.
3. Set `SQLX_OFFLINE=true` before the final build step so SQLx uses cached query metadata instead of connecting to a database.

<details>
<summary>Hint 1</summary>

The SSR cook uses `--no-default-features --features ssr`. The WASM cook uses `--target wasm32-unknown-unknown --no-default-features --features hydrate`. Both use `--release`. These flags match what `cargo leptos build` uses internally for each target.
</details>

<details>
<summary>Hint 2</summary>

Place `COPY . .` after both `cargo chef cook` commands. This is the critical ordering --- source code changes invalidate the `COPY . .` layer and everything after it, but the `cook` layers remain cached. If you put `COPY . .` before the cook commands, changing any source file would force a full dependency recompile.
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
RUN cargo chef cook --release --target wasm32-unknown-unknown --no-default-features --features hydrate --recipe-path recipe.json

COPY . .

ENV SQLX_OFFLINE=true

RUN cargo leptos build --release -vv
```

The two cook steps compile dependencies for both targets. The SSR target compiles to native x86_64/arm64 code. The WASM target compiles to WebAssembly. Both are cached independently --- changing features in one does not invalidate the other.
</details>

### Exercise 3: Write the Runtime stage with minimal dependencies

**Goal:** Create a minimal production image that copies only the compiled binary, static assets, and configuration from the Builder stage.

**Instructions:**

1. Start from `debian:trixie-slim` --- a minimal Debian image without compilers or development tools
2. Install only two packages: `openssl` (needed by the binary for TLS) and `ca-certificates` (needed for HTTPS requests to external services). Clean up the package cache afterward.
3. Copy four things from the Builder stage: the compiled binary (`/app/target/release/gritwit`), the site directory (`/app/target/site`), `Cargo.toml` (needed by cargo-leptos at runtime), and the configuration directory
4. Set environment variables for production defaults: `RUST_LOG`, `LEPTOS_SITE_ADDR` (must be `0.0.0.0:3000`, not `127.0.0.1`), `LEPTOS_SITE_ROOT`, and `APP_ENVIRONMENT`
5. Expose port 3000 and set the CMD to run the binary

<details>
<summary>Hint 1</summary>

The compiled binary is at `/app/target/release/gritwit`. The site assets are at `/app/target/site`. Cargo.toml is needed because cargo-leptos reads it at runtime for configuration (like the output name and site root path). The `COPY --from=builder` syntax copies files from a named stage.
</details>

<details>
<summary>Hint 2</summary>

Binding to `127.0.0.1` inside a container makes the service unreachable from outside the container. Use `0.0.0.0:3000` for `LEPTOS_SITE_ADDR`. The `EXPOSE 3000` instruction is documentation only --- it tells users which port the container listens on but does not actually open the port. The `-p 3000:3000` flag in `docker run` does the actual port mapping.
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

The runtime image is roughly 130 MB. Compare this to the builder image (2+ GB) or a naive single-stage build that ships the entire Rust toolchain. The reduction comes from starting with a minimal base image and copying only what the application needs to run.
</details>

### Exercise 4: Optimize the Dockerfile for cache efficiency

**Goal:** Analyze the current Dockerfile and identify opportunities to improve layer caching and reduce image size.

**Instructions:**

Answer these questions by tracing through the Dockerfile layer by layer:

1. What happens to the Docker cache when you change a single `.rs` file? Which layers rebuild and which are cached?
2. What happens when you add a new crate to `Cargo.toml`? Which layers rebuild?
3. Could you use `scratch` or `alpine` instead of `debian:trixie-slim` for the runtime? What are the tradeoffs?
4. How would you add a `.dockerignore` file to reduce the build context size?

<details>
<summary>Hint 1</summary>

Trace through the Dockerfile layer by layer. The Planner stage `COPY . .` captures everything. If any file changes, the Planner re-runs. But `recipe.json` only changes when dependency metadata changes. If `recipe.json` is identical to the cached version, the Builder's `cargo chef cook` layers are cached --- even though the Planner ran again.
</details>

<details>
<summary>Hint 2</summary>

Alpine uses `musl` libc instead of `glibc`. Some Rust crates (especially those using C bindings) may not compile or behave correctly with musl. The `scratch` image has no OS at all --- you would need a fully statically linked binary, no shell for debugging, and you would need to bundle CA certificates manually.
</details>

<details>
<summary>Solution</summary>

**Cache behavior for a `.rs` file change:**
1. Planner: rebuilds (COPY . . changed) --- fast, just `cargo chef prepare`
2. Builder: `recipe.json` is unchanged, so both `cargo chef cook` layers are CACHED
3. Builder: `COPY . .` rebuilds (source changed)
4. Builder: `cargo leptos build` rebuilds --- but dependencies are already compiled, so only application code compiles
5. Runtime: rebuilds only the COPY layers

**Cache behavior for a new crate in Cargo.toml:**
1. Planner: rebuilds --- `recipe.json` changes because dependency list changed
2. Builder: both `cargo chef cook` layers REBUILD --- new dependencies must be compiled
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

Excluding `target/` is critical --- it can be gigabytes. Without a `.dockerignore`, Docker sends the entire project directory (including `target/`) to the build daemon as the "build context." A multi-gigabyte context slows down every build before the first instruction even runs.
</details>

---

## Rust Gym: Layer Optimization Drills

These drills practice understanding Docker build performance and image optimization.

### Drill 1: Analyze layer sizes

<details>
<summary>Exercise</summary>

Build the Docker image and use `docker history` to inspect each layer's size:

```bash
docker build -t grindit:latest .
docker history grindit:latest
```

Identify the largest layers. The `apt-get install` layer in the runtime stage should be small (just openssl and ca-certificates). The `COPY --from=builder` layers contain the binary and assets.

If the runtime image is larger than expected, check that you have `--no-install-recommends` in the `apt-get install` command and the cleanup commands (`autoremove`, `clean`, `rm -rf /var/lib/apt/lists/*`).
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

The second build should skip the `cargo chef cook` layers and only rebuild from `COPY . .` onward. On a typical machine, this reduces build time from 10-15 minutes to 2-3 minutes. The time savings compound over hundreds of builds in a CI pipeline.
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

Multi-platform builds are important for teams with mixed hardware (Intel desktops and Apple Silicon laptops) and for deploying to ARM-based cloud instances (which are often cheaper and more power-efficient).
</details>

---

## System Design Corner: Containerization

**Interview question:** "How would you containerize a full-stack web application with server-side rendering?"

**What we just built:** A four-stage Docker image that separates build concerns (Rust toolchain, WASM compilation, SCSS compilation) from runtime concerns (binary execution, static file serving).

**Talking points:**

- **Layer caching strategy** --- the most important optimization in Docker builds. Layers that change frequently (source code) should be late in the Dockerfile. Layers that change rarely (system packages, dependency compilation) should be early. cargo-chef takes this further by extracting the dependency fingerprint so that source code changes do not invalidate the dependency layer.

- **Build vs runtime separation** --- the build image needs compilers, linkers, and development headers. The runtime image needs only the compiled binary and its dynamic library dependencies. Multi-stage builds enforce this separation structurally --- you cannot accidentally ship the compiler to production.

- **Offline build capability** --- production builds should not depend on external services. `SQLX_OFFLINE=true` removes the database dependency. Pinned dependency versions in `Cargo.lock` ensure reproducible builds. The only external dependency during build is the crate registry, which can be mitigated with a local mirror.

- **Image size tradeoffs** --- Debian slim (~80 MB) vs Alpine (~5 MB) vs scratch (~0 MB). Smaller is not always better. Debian slim provides `apt-get` for installing debug tools, a shell for `docker exec`, and glibc compatibility. Alpine's musl libc introduces subtle runtime differences. Scratch eliminates debugging capability entirely. Choose based on your operational needs.

- **Security hardening** --- the runtime image has no compiler, no package manager (beyond apt for debugging), and no source code. The attack surface is minimal. Further hardening: run as a non-root user (`RUN useradd -r grindit && USER grindit`), use read-only file systems, and scan the image with tools like `trivy`.

- **Configuration injection** --- the image contains YAML configuration files with defaults. Secrets and environment-specific values are injected via environment variables at runtime (`docker run -e APP_DATABASE__PASSWORD=...`). This follows the twelve-factor app methodology: config in the environment, not in the image.

---

> **Design Insight: Deep Modules in Build Systems** (Ousterhout, Ch. 4)
>
> The Dockerfile presents a simple interface --- `docker build -t grindit .` --- but hides significant complexity inside: four stages, two compilation targets (native + WASM), SCSS compilation, SQLx offline mode, multi-architecture support, and layer caching optimization. Users of the image (DevOps engineers, CI systems) do not need to understand any of this. They run one command and get a production-ready container. This is the definition of a deep module: simple interface, complex implementation.

---

## What You Built

This chapter built the production Docker image for GrindIt:

- **Four-stage build** --- Chef (tools) to Planner (dependency fingerprint) to Builder (compile) to Runtime (run). Each stage has a single responsibility, and the final image contains only the compiled artifacts.
- **cargo-chef** --- generates a `recipe.json` that fingerprints the dependency graph. Docker caches the dependency compilation layer, skipping it when only source code changes. This reduces incremental builds from 10+ minutes to 2-3 minutes.
- **`SQLX_OFFLINE=true`** --- enables compilation without a live database. The `.sqlx/` directory contains cached query metadata generated during development.
- **Dart Sass** --- installed from a release archive with `TARGETARCH` for multi-platform support. The `sass` binary is needed during build (SCSS compilation) but not at runtime.
- **Minimal runtime** --- `debian:trixie-slim` with only `openssl` and `ca-certificates`. The compiled binary, site assets, and configuration are copied from the Builder. Environment variables set production defaults.

If you run `docker build -t grindit:latest .` and then `docker images grindit`, you should see an image around 130 MB. Running it with `docker run -p 3000:3000 grindit:latest` starts the server, accessible at `http://localhost:3000`.

The Docker image is the deployable unit. It contains everything needed to run GrindIt in any environment --- a cloud VM, a Kubernetes pod, or a container platform like Fly.io or Railway. The next chapter automates quality checks and deployment with CI/CD pipelines.
