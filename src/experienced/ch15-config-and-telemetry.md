# Chapter 15: Configuration & Telemetry

A web application needs different settings for local development and production. It needs structured logs that machines can parse. And it needs request tracing so you can follow a single HTTP request through every function it touches. This chapter builds all three: a layered configuration system (YAML base, environment overlay, env var override), structured logging with Bunyan JSON formatting, and request ID propagation with tower-http middleware.

The spotlight concept is **serde deep dive and configuration patterns** — how Rust's serde framework deserializes configuration from multiple sources into strongly typed structs, how `TryFrom` converts raw strings into validated types, and how the config crate merges layered sources into a single `Settings` struct. You will see why Rust's type system makes misconfiguration a compile-time or startup-time error, not a runtime surprise.

By the end of this chapter, you will have:

- A `Settings` struct hierarchy: `ApplicationSettings`, `DatabaseSettings`, `OAuthSettings`, `StorageSettings`, `SmsSettings`
- YAML configuration with `base.yaml` + `local.yaml`/`production.yaml` overlay + `APP_*` environment variable override
- An `Environment` enum with `TryFrom<String>` for safe parsing
- Structured logging with `tracing`, `tracing-bunyan-formatter`, and `EnvFilter`
- `TraceLayer` middleware with custom span fields (method, URI, request ID, status, latency)
- `SetRequestIdLayer` and `PropagateRequestIdLayer` for request ID propagation

---

## Spotlight: Serde Deep Dive & Configuration Patterns

### The layered configuration problem

Every application needs configuration: database URLs, API keys, feature flags, port numbers. The challenge is that these values differ between environments:

| Setting | Local | Production |
|---|---|---|
| `application.host` | `127.0.0.1` | `0.0.0.0` |
| `database.require_ssl` | `false` | `true` |
| `storage.backend` | `"local"` | `"r2"` |
| `database.password` | `"password"` | `(from env var)` |

In JavaScript, you typically use `dotenv` and `process.env`:

```javascript
// Node.js: all config is string-based
const port = parseInt(process.env.PORT || "3000");
const ssl = process.env.DB_SSL === "true";
// No compile-time guarantees. Typos in env var names are silent.
```

> **Coming from JS?** JavaScript config is stringly typed — everything is a `string` or `undefined`, and you parse at runtime. Rust's config crate deserializes directly into typed structs. A missing field is a startup error, not a runtime `undefined`. A `port` field that contains `"abc"` fails at deserialization, not when you try to bind the socket.

### Serde: the serialization framework

Serde (SERialize/DEserialize) is Rust's universal serialization framework. The `#[derive(Deserialize)]` macro generates code that can construct your struct from any supported format — JSON, YAML, TOML, environment variables, or custom sources.

```rust
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}
```

The `#[serde(deserialize_with = "...")]` attribute tells serde to use a custom deserialization function for the `port` field. The `deserialize_number_from_string` function (from the `serde_aux` crate) accepts both `3000` (number) and `"3000"` (string) and converts both to `u16`. This is necessary because environment variables are always strings, but YAML files can represent numbers natively.

### The Secret\<T\> wrapper

Database passwords and API keys should never appear in logs, error messages, or debug output. The `secrecy` crate provides `Secret<T>` — a wrapper that redacts the inner value in `Debug` and `Display` implementations:

```rust
use secrecy::Secret;

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    // ...
}
```

If you accidentally `println!("{:?}", config.database)`, the password appears as `Secret([REDACTED])`. To access the actual value, you must call `expose_secret()`:

```rust
use secrecy::ExposeSecret;

let connection_string = format!(
    "postgresql://{}:{}@{}:{}/{}",
    self.username,
    self.password.expose_secret(),  // explicit opt-in to access the secret
    self.host,
    self.port,
    self.database_name,
);
```

The `expose_secret()` call is a code smell detector — you can grep for it to find every place that accesses raw secrets. In code review, seeing `expose_secret()` should trigger extra scrutiny.

### The Settings struct hierarchy

GrindIt's configuration is a tree of structs:

```rust
#[derive(Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub oauth: OAuthSettings,
    pub storage: StorageSettings,
    #[serde(default)]
    pub sms: Option<SmsSettings>,
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
    pub channel_binding: bool,
}

#[derive(Deserialize, Clone)]
pub struct OAuthSettings {
    pub google_client_id: Secret<String>,
    pub google_client_secret: Secret<String>,
    pub redirect_url: String,
}

#[derive(Deserialize, Clone)]
pub struct StorageSettings {
    pub backend: String,
    pub r2_account_id: Option<String>,
    pub r2_access_key: Option<Secret<String>>,
    pub r2_secret_key: Option<Secret<String>>,
    pub r2_bucket: Option<String>,
    pub r2_public_url: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct SmsSettings {
    pub api_key: Secret<String>,
}
```

Several patterns:

- **Nested structs** — `Settings` contains `ApplicationSettings`, `DatabaseSettings`, etc. Serde maps YAML nesting to struct nesting automatically. `application.port` in YAML becomes `settings.application.port` in Rust.
- **`#[serde(default)]`** — the `sms` field is `Option<SmsSettings>` with `#[serde(default)]`. If the `sms` section is missing from the YAML, serde fills it with `None` instead of erroring. This makes SMS support optional.
- **`Option` fields in `StorageSettings`** — the R2 fields are `Option` because they are only required when `backend` is `"r2"`. The `from_config` method in Chapter 13 validates this at runtime with `expect()`.

### Design Insight: Complexity layers

John Ousterhout's *A Philosophy of Software Design* describes **layers of abstraction** where each layer hides the complexity of the layers below. The configuration system has three layers:

1. **YAML files** — human-readable, version-controlled, contain defaults and non-secret settings
2. **Environment variables** — machine-injected, contain secrets and deployment-specific overrides
3. **Rust structs** — strongly typed, validated at startup, used by all application code

Application code only sees layer 3. It accesses `settings.database.port` as a `u16` — it does not know whether that value came from `base.yaml`, `production.yaml`, or the `APP_DATABASE__PORT` environment variable. Each layer hides the ones below.

---

## Building the Configuration System

### The Environment enum

The `APP_ENVIRONMENT` variable determines which overlay file to load. It must be one of two values:

```rust
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. \
                 Use either `local` or `production`.",
                other
            )),
        }
    }
}
```

`TryFrom<String>` is a standard library trait for fallible conversions. It converts a `String` into an `Environment`, returning an error for invalid values. The `to_lowercase()` call makes parsing case-insensitive — "Production", "PRODUCTION", and "production" all work.

The `as_str()` method returns `&'static str` — a string with static lifetime. The literals `"local"` and `"production"` are baked into the binary and live for the entire program duration. This is safe to return because static references never dangle.

### The get_configuration function

```rust
pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir()
        .expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = Config::builder()
        .add_source(File::from(
            configuration_directory.join("base.yaml")))
        .add_source(File::from(
            configuration_directory.join(environment_filename)))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}
```

The three-layer merge:

1. **`base.yaml`** — loaded first. Contains defaults shared across all environments:

```yaml
application:
  host: "127.0.0.1"
  port: 3000
database:
  host: "localhost"
  port: 5432
  username: "postgres"
  password: "password"
  database_name: "gritwit"
  require_ssl: false
  channel_binding: false
oauth:
  google_client_id: "PLACEHOLDER"
  google_client_secret: "PLACEHOLDER"
  redirect_url: "http://localhost:3000/auth/google/callback"
storage:
  backend: "local"
```

2. **`local.yaml` or `production.yaml`** — loaded second, overrides base values. The production overlay changes SSL and storage:

```yaml
# production.yaml
application:
  host: "0.0.0.0"
  port: 3000
database:
  require_ssl: true
  channel_binding: false
storage:
  backend: "r2"
```

Only the fields that differ from base need to be specified. The config crate deep-merges nested structs — `production.yaml` overrides `database.require_ssl` without affecting `database.host` or `database.port`.

3. **Environment variables** — loaded last, highest priority. The naming convention: `APP_{SECTION}__{FIELD}`. Double underscore (`__`) separates nesting levels:

```bash
# These override any YAML values
APP_DATABASE__PASSWORD=real_production_password
APP_DATABASE__HOST=db.production.internal
APP_OAUTH__GOOGLE_CLIENT_ID=real_client_id
APP_OAUTH__GOOGLE_CLIENT_SECRET=real_client_secret
```

The `prefix_separator("_")` strips the `APP_` prefix. The `separator("__")` maps `DATABASE__PASSWORD` to `database.password`. This convention means secrets never appear in YAML files that get committed to git — they exist only in the deployment environment.

### DatabaseSettings helper methods

The `DatabaseSettings` struct provides two methods for creating database connections:

```rust
impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        let sslmode = if self.require_ssl { "require" } else { "disable" };
        let channel_binding = if self.channel_binding { "require" } else { "disable" };
        format!(
            "postgresql://{}:{}@{}:{}/{}?sslmode={}&channel_binding={}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name,
            sslmode,
            channel_binding,
        )
    }

    pub fn connection_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Disable
        };
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(self.password.expose_secret())
            .database(&self.database_name)
            .ssl_mode(ssl_mode)
    }
}
```

Two methods, two purposes:

- **`connection_string()`** — returns a URL string. Useful for tools like `sqlx-cli` that accept connection strings.
- **`connection_options()`** — returns a typed `PgConnectOptions` struct. Used by `PgPoolOptions::connect_lazy_with()` in `main.rs`. The typed approach avoids URL-encoding issues and provides better error messages.

The `connect_lazy_with` call in `main.rs` is worth noting:

```rust
let pool = PgPoolOptions::new()
    .connect_lazy_with(app_config.database.connection_options());
```

`connect_lazy_with` does not establish a database connection at startup. It creates a pool that connects on first use. This means the server can start even if the database is temporarily unavailable — useful in container orchestration where services start in parallel.

---

## Structured Logging with Tracing

### Why structured logging?

Traditional logging produces human-readable text:

```
[2024-03-15 10:30:42] INFO: POST /api/v1/upload/video - 200 OK (45ms)
```

Structured logging produces machine-parseable JSON:

```json
{
  "v": 0,
  "name": "gritwit",
  "msg": "response",
  "level": 30,
  "time": "2024-03-15T10:30:42.123Z",
  "method": "POST",
  "uri": "/api/v1/upload/video",
  "request_id": "a1b2c3d4-e5f6-7890",
  "status": 200,
  "latency_ms": 45
}
```

The JSON format lets you query logs with tools like `jq`, Elasticsearch, or Datadog:

```bash
# Find all slow requests
cat logs.json | jq 'select(.latency_ms > 1000)'

# Find all errors for a specific request
cat logs.json | jq 'select(.request_id == "a1b2c3d4")'
```

### The tracing ecosystem

Rust's `tracing` crate provides the instrumentation layer. Three concepts:

1. **Spans** — named contexts with structured fields. A span represents a unit of work (an HTTP request, a database query, a function call). Spans can nest.
2. **Events** — point-in-time occurrences within a span. `tracing::info!("response")` creates an event.
3. **Subscribers** — consumers that process spans and events. The subscriber decides what to do with the data — print it, send it to a service, or discard it.

### Building the subscriber

```rust
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
```

The subscriber is built from three layers:

1. **`EnvFilter`** — controls which log levels are emitted. `EnvFilter::try_from_default_env()` reads the `RUST_LOG` environment variable (`RUST_LOG=debug` enables debug logs). The fallback `env_filter` parameter (typically `"info"`) is used when `RUST_LOG` is not set.

2. **`JsonStorageLayer`** — collects span fields into a JSON map. When a span has fields like `method = "POST"` and `uri = "/upload"`, this layer stores them so the formatting layer can include them in the output.

3. **`BunyanFormattingLayer`** — formats each event as a Bunyan-compatible JSON line. Bunyan is a JSON logging format created by Joyent. It includes `v` (version), `name` (application), `msg`, `level`, `time`, and all span fields.

### The Sink generic parameter

```rust
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
```

The `Sink` parameter determines where logs are written. In production, it is `std::io::stdout` (logs go to stdout, where a log aggregator picks them up). In tests, it could be `std::io::sink` (discard all output) or a `Vec<u8>` for assertion.

The `for<'a> MakeWriter<'a>` bound is a **Higher-Ranked Trait Bound (HRTB)** — it says "for any lifetime `'a`, `Sink` must implement `MakeWriter<'a>`." This is necessary because the tracing system creates writers with varying lifetimes depending on when events occur. `std::io::stdout` satisfies this bound because it can create writers for any lifetime.

### Rust Gym: Custom serde deserialization

<details>
<summary>Exercise: write a custom deserializer for Duration from seconds</summary>

```rust
use serde::{self, Deserialize, Deserializer};
use std::time::Duration;

pub fn deserialize_duration_from_secs<'de, D>(
    deserializer: D,
) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(secs))
}

// Usage:
#[derive(Deserialize)]
pub struct SessionSettings {
    #[serde(deserialize_with = "deserialize_duration_from_secs")]
    pub timeout: Duration,
}
```

The pattern: deserialize to a primitive type, then convert to the target type. The `Deserializer` trait handles the format-specific parsing (YAML number, JSON number, env var string). Your function handles the semantic conversion (number to Duration).
</details>

### Rust Gym: TryFrom for validated types

<details>
<summary>Exercise: implement TryFrom for a LogLevel enum</summary>

```rust
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl TryFrom<String> for LogLevel {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" | "warning" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            other => Err(format!(
                "'{}' is not a valid log level. Use: debug, info, warn, error",
                other
            )),
        }
    }
}
```

`TryFrom` is the idiomatic pattern for "parse this string into a validated type." The `type Error = String` keeps the error type simple. For library code, you would define a custom error type; for application code, `String` is sufficient.
</details>

---

## Request Tracing with Tower Middleware

### The middleware stack

In `main.rs`, the Axum router is wrapped with three middleware layers:

```rust
let app = Router::new()
    // ... routes ...
    .layer(session_layer)
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                let request_id = request.headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown");
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    status = tracing::field::Empty,
                    latency_ms = tracing::field::Empty,
                )
            })
            .on_response(
                |response: &axum::http::Response<_>,
                 latency: std::time::Duration,
                 span: &tracing::Span| {
                    span.record("status", response.status().as_u16());
                    span.record("latency_ms", latency.as_millis() as u64);
                    tracing::info!("response");
                },
            ),
    )
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
    .layer(PropagateRequestIdLayer::x_request_id());
```

Axum applies layers in **reverse order** — the last `.layer()` call runs first. So the execution order for each request is:

1. **`PropagateRequestIdLayer`** — copies the incoming `x-request-id` header to the response. If a load balancer or API gateway set a request ID, it passes through.
2. **`SetRequestIdLayer`** — if no `x-request-id` header exists, generates a new UUID and sets it. `MakeRequestUuid` generates UUIDs using `uuid::Uuid::new_v4()`.
3. **`TraceLayer`** — creates a tracing span for the request, logs the response with status and latency.
4. **`SessionManagerLayer`** — manages the session cookie.

### The TraceLayer span

```rust
tracing::info_span!(
    "http_request",
    method = %request.method(),
    uri = %request.uri(),
    request_id = %request_id,
    status = tracing::field::Empty,
    latency_ms = tracing::field::Empty,
)
```

The span is created with `info_span!` — it only appears in logs when the log level is INFO or higher. The fields use the `%` sigil for `Display` formatting. Two fields — `status` and `latency_ms` — are declared as `Empty`. They are populated later in the `on_response` callback:

```rust
span.record("status", response.status().as_u16());
span.record("latency_ms", latency.as_millis() as u64);
```

Why declare empty fields upfront? Because tracing spans are **write-once** — you can only record values for fields that were declared when the span was created. If you omit `status` from the `info_span!` macro, `span.record("status", ...)` would silently do nothing.

### Request ID propagation

The request ID serves two purposes:

1. **Correlation** — when debugging an issue, you can filter all log lines for a specific request: `jq 'select(.request_id == "abc-123")'`. This is invaluable when multiple requests are interleaved in the log stream.

2. **Distributed tracing** — if GrindIt calls external services, the request ID can be forwarded as a header. The external service includes it in its logs, creating a trace across service boundaries.

```rust
.layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
.layer(PropagateRequestIdLayer::x_request_id())
```

`SetRequestIdLayer` generates a UUID for each incoming request that does not already have an `x-request-id` header. `PropagateRequestIdLayer` copies the `x-request-id` from the request to the response, so clients (or intermediaries) can see the ID.

### System Design: Observability

Observability has three pillars:

1. **Logs** — discrete events with context (what happened, when, in which request). GrindIt implements this with tracing + Bunyan.
2. **Metrics** — numeric measurements over time (request count, latency percentiles, error rate). GrindIt does not implement metrics yet, but `tower-http` provides `MetricsLayer` for future use.
3. **Traces** — the path of a request through the system. GrindIt's request ID propagation is the foundation — it enables correlating logs across a single request. Full distributed tracing (OpenTelemetry, Jaeger) would be the next step for a microservices architecture.

The structured JSON logs are the most important investment. With structured logs:

- **Debugging** becomes `jq` queries instead of `grep` guesswork
- **Alerting** becomes possible — a monitoring tool can parse JSON and trigger on `level >= 50` (error)
- **Performance analysis** becomes easy — filter by latency to find slow requests

---

## Wiring It All Together in main.rs

### Startup sequence

```rust
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let subscriber = get_subscriber("gritwit".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let app_config = configuration::get_configuration()
        .expect("Failed to read configuration");

    let pool = PgPoolOptions::new()
        .connect_lazy_with(app_config.database.connection_options());

    sqlx::migrate!().run(&pool).await
        .expect("Could not run database migrations");

    // ... session store, OAuth client, storage backend ...

    let storage = std::sync::Arc::new(
        StorageBackend::from_config(&app_config.storage)
    );
    tracing::info!("storage backend: {}", app_config.storage.backend);

    // ... router setup, middleware stack ...

    tracing::info!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service()).await.unwrap();
}
```

The startup order matters:

1. **`dotenvy::dotenv()`** — loads `.env` file into environment variables. The `.ok()` ignores errors (the file may not exist in production, where env vars are set by the platform).
2. **Subscriber initialization** — must happen before any `tracing::info!` calls. After this point, all log events are captured.
3. **Configuration loading** — reads YAML files and env vars. `expect()` panics on failure — a misconfigured server should not start.
4. **Database pool** — `connect_lazy_with` does not connect yet. The first query triggers the connection.
5. **Migrations** — runs pending SQL migrations. This is the one place where the server blocks on the database at startup.
6. **Storage backend** — constructed from config, wrapped in `Arc` for sharing.
7. **Router + middleware** — the middleware stack is applied, and the server starts listening.

The `tracing::info!` calls at key points (storage backend, listening address) create breadcrumbs in the log. When debugging a startup issue, these messages confirm which stages completed successfully.

---

## Exercises

### Exercise 1: Build Settings struct hierarchy

Define the full `Settings` struct hierarchy in `src/configuration.rs`: `Settings`, `ApplicationSettings`, `DatabaseSettings`, `OAuthSettings`, `StorageSettings`, and `SmsSettings`. Use `Secret<String>` for sensitive fields, `#[serde(deserialize_with)]` for port numbers, and `#[serde(default)]` for optional sections.

<details>
<summary>Hints</summary>

- All structs need `#[derive(Deserialize, Clone)]`
- `Secret<String>` fields work with serde automatically — the `secrecy` crate provides the `Deserialize` impl
- Use `serde_aux::field_attributes::deserialize_number_from_string` for `port` fields
- `StorageSettings` has `Option<Secret<String>>` for R2 credentials — they are only present when backend is "r2"
- `SmsSettings` is wrapped in `Option` with `#[serde(default)]` — the entire section may be absent
</details>

<details>
<summary>Solution</summary>

```rust
use config::{Config, File};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub oauth: OAuthSettings,
    pub storage: StorageSettings,
    #[serde(default)]
    pub sms: Option<SmsSettings>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
    pub channel_binding: bool,
}

#[derive(Deserialize, Clone)]
pub struct OAuthSettings {
    pub google_client_id: Secret<String>,
    pub google_client_secret: Secret<String>,
    pub redirect_url: String,
}

#[derive(Deserialize, Clone)]
pub struct StorageSettings {
    pub backend: String,
    pub r2_account_id: Option<String>,
    pub r2_access_key: Option<Secret<String>>,
    pub r2_secret_key: Option<Secret<String>>,
    pub r2_bucket: Option<String>,
    pub r2_public_url: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct SmsSettings {
    pub api_key: Secret<String>,
}
```

Notice `DatabaseSettings` does not derive `Debug` — it contains a `Secret<String>`, and while `Secret` does implement `Debug` (printing `[REDACTED]`), omitting `Debug` on the parent is an extra safety measure. `ApplicationSettings` derives `Debug` because it contains no secrets.
</details>

### Exercise 2: Implement YAML loading with base + environment overlay

Write `get_configuration()` and the `Environment` enum. The function should load `base.yaml`, overlay with `local.yaml` or `production.yaml` based on `APP_ENVIRONMENT`, and override with `APP_*` environment variables using `__` as the nesting separator.

<details>
<summary>Hints</summary>

- Read `APP_ENVIRONMENT` with `std::env::var`, default to `"local"`
- Use `.try_into()` to convert the string to `Environment` via `TryFrom`
- Build the config with `Config::builder().add_source(...).add_source(...).add_source(...).build()?`
- The env var source: `config::Environment::with_prefix("APP").prefix_separator("_").separator("__")`
- Call `settings.try_deserialize::<Settings>()` to convert to the typed struct
</details>

<details>
<summary>Solution</summary>

```rust
pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{} is not a supported environment. \
                 Use either `local` or `production`.",
                other
            )),
        }
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir()
        .expect("Failed to determine the current directory");
    let configuration_directory = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");

    let environment_filename = format!("{}.yaml", environment.as_str());

    let settings = Config::builder()
        .add_source(File::from(configuration_directory.join("base.yaml")))
        .add_source(File::from(
            configuration_directory.join(environment_filename),
        ))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}
```

The layered merge order — base, then environment, then env vars — means each layer can override the previous. Environment variables have the highest priority, which is the standard pattern for containerized deployments where secrets are injected as env vars.
</details>

### Exercise 3: Set up tracing with Bunyan formatter and TraceLayer middleware

Create `src/telemetry.rs` with `get_subscriber` and `init_subscriber`. Then add `TraceLayer` to the Axum router in `main.rs` with custom span fields for method, URI, request ID, status, and latency.

<details>
<summary>Hints</summary>

- `get_subscriber` returns `impl Subscriber + Send + Sync` — use `Registry::default().with(...)` to compose layers
- The `Sink` generic needs `for<'a> MakeWriter<'a> + Send + Sync + 'static`
- Use `EnvFilter::try_from_default_env()` with a fallback for the default level
- `LogTracer::init()` bridges the `log` crate to `tracing` (for libraries that use `log`)
- In `TraceLayer`, use `tracing::field::Empty` for fields populated later
- In `on_response`, use `span.record("status", ...)` to fill the empty fields
</details>

<details>
<summary>Solution</summary>

```rust
// src/telemetry.rs
use tracing::{subscriber::set_global_default, Subscriber};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
```

In `main.rs`:

```rust
let subscriber = get_subscriber("gritwit".into(), "info".into(), std::io::stdout);
init_subscriber(subscriber);

// ... later, in the router:
.layer(
    TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<_>| {
            let request_id = request.headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                request_id = %request_id,
                status = tracing::field::Empty,
                latency_ms = tracing::field::Empty,
            )
        })
        .on_response(
            |response: &axum::http::Response<_>,
             latency: std::time::Duration,
             span: &tracing::Span| {
                span.record("status", response.status().as_u16());
                span.record("latency_ms", latency.as_millis() as u64);
                tracing::info!("response");
            },
        ),
)
```

The `get_subscriber` function is generic over the output sink, making it testable — in tests, pass `std::io::sink()` to suppress log output, or pass a buffer to capture logs for assertions.
</details>

### Exercise 4: Add request ID propagation (SetRequestIdLayer + PropagateRequestIdLayer)

Add `SetRequestIdLayer` and `PropagateRequestIdLayer` from `tower-http` to the middleware stack. The layers should use the `x-request-id` header. If a request arrives without the header, generate a UUID. Propagate the header to the response.

<details>
<summary>Hints</summary>

- Import `tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer}`
- `SetRequestIdLayer::x_request_id(MakeRequestUuid)` generates UUIDs for requests missing the header
- `PropagateRequestIdLayer::x_request_id()` copies the header from request to response
- Layer order matters in Axum — layers are applied bottom-up. Place `PropagateRequestIdLayer` last (runs first), `SetRequestIdLayer` second-to-last, `TraceLayer` above them
- The `TraceLayer` span reads the `x-request-id` header — it must run after `SetRequestIdLayer` has set it
</details>

<details>
<summary>Solution</summary>

```rust
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

let app = Router::new()
    // ... routes ...
    .layer(session_layer)
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                let request_id = request.headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown");
                tracing::info_span!(
                    "http_request",
                    method = %request.method(),
                    uri = %request.uri(),
                    request_id = %request_id,
                    status = tracing::field::Empty,
                    latency_ms = tracing::field::Empty,
                )
            })
            .on_response(
                |response: &axum::http::Response<_>,
                 latency: std::time::Duration,
                 span: &tracing::Span| {
                    span.record("status", response.status().as_u16());
                    span.record("latency_ms", latency.as_millis() as u64);
                    tracing::info!("response");
                },
            ),
    )
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
    .layer(PropagateRequestIdLayer::x_request_id());
```

Execution order (bottom-up): `PropagateRequestIdLayer` runs first (captures the incoming ID for propagation), `SetRequestIdLayer` runs second (generates an ID if none exists), `TraceLayer` runs third (reads the now-guaranteed ID for the span), `SessionManagerLayer` runs last.

The response includes the `x-request-id` header. A client can log this header and include it in bug reports, enabling the backend team to find all logs for that specific request.
</details>

---

## Summary

This chapter built the infrastructure layer that makes GrindIt production-ready:

- **Layered configuration** — `base.yaml` provides defaults, environment files override per deployment, environment variables override everything. The `config` crate merges all three sources and serde deserializes into strongly typed structs. A typo in a field name is a startup error, not a runtime crash.
- **`Secret<T>`** — prevents accidental logging of passwords and API keys. `expose_secret()` is the only way to access the inner value, making secret access grep-able and reviewable.
- **`TryFrom<String>`** — the idiomatic pattern for parsing strings into validated types. The `Environment` enum only accepts "local" or "production" — invalid values are caught at startup.
- **Structured logging** — Bunyan JSON format with `tracing`, `JsonStorageLayer`, and `BunyanFormattingLayer`. Machine-parseable logs enable `jq` queries, alerting, and performance analysis.
- **Request tracing** — `TraceLayer` creates spans with method, URI, request ID, status, and latency. `SetRequestIdLayer` generates UUIDs. `PropagateRequestIdLayer` forwards IDs to responses.

Together, these systems ensure that when something goes wrong in production, you can find out what happened, which request caused it, and what configuration was active — without adding `println!` statements and redeploying.

The next chapter builds the REST API layer with OpenAPI documentation and Swagger UI.
