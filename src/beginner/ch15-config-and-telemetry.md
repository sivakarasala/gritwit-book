# Chapter 15: Configuration & Telemetry

A web application needs different settings for local development and production. It needs structured logs that machines can parse. And it needs request tracing so you can follow a single HTTP request through every function it touches. This chapter builds all three: a layered configuration system (YAML base, environment overlay, env var override), structured logging with Bunyan JSON formatting, and request ID propagation with tower-http middleware.

The spotlight concept is **serde deep dive and configuration patterns** --- how Rust's serde framework deserializes configuration from multiple sources into strongly typed structs, how `TryFrom` converts raw strings into validated types, and how the config crate merges layered sources into a single `Settings` struct. You will see why Rust's type system makes misconfiguration a compile-time or startup-time error, not a runtime surprise.

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

> **Programming Concept: What is Configuration?**
>
> Configuration is the settings that control how your application behaves, stored outside the code itself. Think of a car: the engine (code) is the same for every driver, but each driver adjusts the seat position, mirror angles, and radio presets (configuration).
>
> Why separate configuration from code?
>
> - **Different environments**: Your laptop uses a local database on `localhost:5432`. The production server uses a managed database on `db.production.internal:5432`. The code is identical --- only the configuration changes.
> - **Secrets**: Database passwords and API keys should never appear in source code (which gets committed to git and visible to anyone with repo access). Configuration lets you inject secrets from the environment.
> - **Flexibility**: Changing a port number should not require recompiling the entire application.
>
> Common configuration formats:
> - **YAML** (`.yaml`) --- human-readable, supports nesting, used by GrindIt
> - **TOML** (`.toml`) --- Rust's favorite format, used by `Cargo.toml`
> - **JSON** (`.json`) --- universal but verbose, no comments allowed
> - **Environment variables** --- key-value pairs set by the operating system or container platform

### Serde: the serialization framework

Serde (SERialize/DEserialize) is Rust's universal serialization framework. The `#[derive(Deserialize)]` macro generates code that can construct your struct from any supported format --- JSON, YAML, TOML, environment variables, or custom sources.

```rust
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}
```

> **Programming Concept: What is Serialization and Deserialization?**
>
> You first saw serialization in Chapter 9, where structs were converted to JSON for server functions. Here is a deeper look:
>
> - **Serialization** = converting a Rust struct into a portable format (like YAML or JSON)
> - **Deserialization** = converting a portable format back into a Rust struct
>
> When GrindIt starts up, it reads a YAML file like this:
>
> ```yaml
> application:
>   host: "127.0.0.1"
>   port: 3000
> ```
>
> Serde's `Deserialize` derive macro generates code that maps `application.host` to the `host` field and `application.port` to the `port` field. If the YAML says `port: "three thousand"`, deserialization fails with a clear error --- not a runtime crash later when you try to bind the socket.
>
> The key insight: serde does not care about the format. The same `#[derive(Deserialize)]` works for JSON, YAML, TOML, and even environment variables. The format-specific parsing is handled by a separate crate (`serde_yaml`, `serde_json`, etc.), while serde provides the bridge to your structs.

The `#[serde(deserialize_with = "...")]` attribute tells serde to use a custom deserialization function for the `port` field. The `deserialize_number_from_string` function (from the `serde_aux` crate) accepts both `3000` (number) and `"3000"` (string) and converts both to `u16`. This is necessary because environment variables are always strings, but YAML files can represent numbers natively.

### The Secret\<T\> wrapper

Database passwords and API keys should never appear in logs, error messages, or debug output. The `secrecy` crate provides `Secret<T>` --- a wrapper that redacts the inner value in `Debug` and `Display` implementations:

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

> **Programming Concept: Why Protect Secrets in Code?**
>
> Imagine you are debugging a problem and add `println!("{:?}", config)` to see what is happening. Without `Secret<T>`, the database password gets printed to the console, saved in log files, and potentially exposed to anyone who can read those logs.
>
> This is not hypothetical --- leaked credentials in logs are one of the most common security incidents. The `Secret<T>` wrapper makes accidental exposure impossible:
>
> - **Without Secret:** `DatabaseSettings { username: "postgres", password: "s3cr3t_p@ssw0rd!" }`
> - **With Secret:** `DatabaseSettings { username: "postgres", password: Secret([REDACTED]) }`
>
> The `expose_secret()` call serves as a code smell detector --- you can search the codebase for every place that accesses raw secrets. In code review, seeing `expose_secret()` should trigger extra scrutiny: "Is this the right place to access the raw password? Could we pass the Secret wrapper instead?"

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

Several patterns to understand:

- **Nested structs** --- `Settings` contains `ApplicationSettings`, `DatabaseSettings`, etc. Serde maps YAML nesting to struct nesting automatically. `application.port` in YAML becomes `settings.application.port` in Rust.
- **`#[serde(default)]`** --- the `sms` field is `Option<SmsSettings>` with `#[serde(default)]`. If the `sms` section is missing from the YAML, serde fills it with `None` instead of erroring. This makes SMS support optional.
- **`Option` fields in `StorageSettings`** --- the R2 fields are `Option` because they are only required when `backend` is `"r2"`. The `from_config` method in Chapter 13 validates this at runtime with `expect()`.

### Design Insight: Complexity layers

John Ousterhout's *A Philosophy of Software Design* describes **layers of abstraction** where each layer hides the complexity of the layers below. The configuration system has three layers:

1. **YAML files** --- human-readable, version-controlled, contain defaults and non-secret settings
2. **Environment variables** --- machine-injected, contain secrets and deployment-specific overrides
3. **Rust structs** --- strongly typed, validated at startup, used by all application code

Application code only sees layer 3. It accesses `settings.database.port` as a `u16` --- it does not know whether that value came from `base.yaml`, `production.yaml`, or the `APP_DATABASE__PORT` environment variable. Each layer hides the ones below.

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

> **Programming Concept: What is TryFrom?**
>
> `TryFrom` is a standard library trait for conversions that might fail. You have used `parse()` on strings before --- `"42".parse::<i32>()` returns `Ok(42)`, while `"hello".parse::<i32>()` returns `Err(...)`. `TryFrom` is the same idea, but for any type-to-type conversion.
>
> The pattern:
>
> ```rust
> impl TryFrom<InputType> for OutputType {
>     type Error = ErrorType;
>
>     fn try_from(input: InputType) -> Result<Self, Self::Error> {
>         // Validate and convert, or return an error
>     }
> }
> ```
>
> After implementing `TryFrom<String>` for `Environment`, you can convert strings like this:
>
> ```rust
> let env: Environment = "production".to_string().try_into()?;
> // or equivalently:
> let env = Environment::try_from("production".to_string())?;
> ```
>
> The `to_lowercase()` call makes parsing case-insensitive --- "Production", "PRODUCTION", and "production" all work. Invalid values like "staging" get a clear error message instead of a silent fallback.

The `as_str()` method returns `&'static str` --- a string with static lifetime. The literals `"local"` and `"production"` are baked into the binary and live for the entire program duration. This is safe to return because static references never dangle.

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

The three-layer merge is the heart of the configuration system. Let us trace through each layer:

**Layer 1: `base.yaml`** --- loaded first. Contains defaults shared across all environments:

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

> **Programming Concept: What is YAML?**
>
> YAML (YAML Ain't Markup Language) is a human-friendly data format. It uses indentation to show nesting (like Python) instead of curly braces (like JSON). Here is a side-by-side comparison:
>
> **YAML:**
> ```yaml
> application:
>   host: "127.0.0.1"
>   port: 3000
> ```
>
> **JSON (same data):**
> ```json
> {
>   "application": {
>     "host": "127.0.0.1",
>     "port": 3000
>   }
> }
> ```
>
> YAML advantages over JSON:
> - Comments are allowed (`# this is a comment`)
> - Less punctuation (no quotes required for most strings, no commas between items)
> - More readable for configuration files
>
> The indentation must be spaces, not tabs. Each level of nesting is typically 2 spaces. The `application:` line creates a section, and `host:` and `port:` are fields within that section. Serde maps this nesting directly to Rust struct nesting.

**Layer 2: `local.yaml` or `production.yaml`** --- loaded second, overrides base values:

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

Only the fields that differ from base need to be specified. The config crate deep-merges nested structs --- `production.yaml` overrides `database.require_ssl` without affecting `database.host` or `database.port`.

**Layer 3: Environment variables** --- loaded last, highest priority:

```bash
# These override any YAML values
APP_DATABASE__PASSWORD=real_production_password
APP_DATABASE__HOST=db.production.internal
APP_OAUTH__GOOGLE_CLIENT_ID=real_client_id
APP_OAUTH__GOOGLE_CLIENT_SECRET=real_client_secret
```

The `prefix_separator("_")` strips the `APP_` prefix. The `separator("__")` maps `DATABASE__PASSWORD` to `database.password`. This convention means secrets never appear in YAML files that get committed to git --- they exist only in the deployment environment.

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

- **`connection_string()`** --- returns a URL string. Useful for tools like `sqlx-cli` that accept connection strings.
- **`connection_options()`** --- returns a typed `PgConnectOptions` struct. Used by `PgPoolOptions::connect_lazy_with()` in `main.rs`. The typed approach avoids URL-encoding issues and provides better error messages.

The `connect_lazy_with` call in `main.rs` is worth noting:

```rust
let pool = PgPoolOptions::new()
    .connect_lazy_with(app_config.database.connection_options());
```

`connect_lazy_with` does not establish a database connection at startup. It creates a pool that connects on first use. This means the server can start even if the database is temporarily unavailable --- useful in container orchestration where services start in parallel.

---

## Structured Logging with Tracing

### Why structured logging?

> **Programming Concept: What is Logging?**
>
> Logging is recording events that happen while your program runs. It is like a ship's logbook --- the captain writes down what happened, when, and any relevant details. When something goes wrong, the log tells you what happened leading up to the problem.
>
> In code, logging looks like:
>
> ```rust
> tracing::info!("User logged in");
> tracing::error!("Database connection failed: {}", error);
> ```
>
> **Log levels** indicate severity:
> - **trace** --- extremely detailed, usually only enabled during debugging
> - **debug** --- useful during development
> - **info** --- normal operations worth recording (server started, user logged in)
> - **warn** --- something unexpected but not fatal
> - **error** --- something failed
>
> In production, you typically set the level to `info` (which includes `info`, `warn`, and `error`, but excludes `debug` and `trace`).

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

The difference is like the difference between a handwritten diary and a spreadsheet. Both contain the same information, but the spreadsheet can be sorted, filtered, and queried programmatically. When your production server handles thousands of requests per second, the ability to filter logs by request ID or latency is the difference between finding a bug in seconds and searching for hours.

### The tracing ecosystem

Rust's `tracing` crate provides the instrumentation layer. Three concepts:

1. **Spans** --- named contexts with structured fields. A span represents a unit of work (an HTTP request, a database query, a function call). Spans can nest --- a request span might contain a database query span, which contains a serialization span.
2. **Events** --- point-in-time occurrences within a span. `tracing::info!("response")` creates an event.
3. **Subscribers** --- consumers that process spans and events. The subscriber decides what to do with the data --- print it, send it to a service, or discard it.

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

The subscriber is built from three layers, each handling one concern:

1. **`EnvFilter`** --- controls which log levels are emitted. `EnvFilter::try_from_default_env()` reads the `RUST_LOG` environment variable (`RUST_LOG=debug` enables debug logs). The fallback `env_filter` parameter (typically `"info"`) is used when `RUST_LOG` is not set.

2. **`JsonStorageLayer`** --- collects span fields into a JSON map. When a span has fields like `method = "POST"` and `uri = "/upload"`, this layer stores them so the formatting layer can include them in the output.

3. **`BunyanFormattingLayer`** --- formats each event as a Bunyan-compatible JSON line. Bunyan is a JSON logging format created by Joyent. It includes `v` (version), `name` (application), `msg`, `level`, `time`, and all span fields.

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

> **Programming Concept: What is a Generic Parameter?**
>
> You saw generics in Chapter 9. Here is a practical application: the `Sink` parameter determines where logs are written.
>
> In production, you pass `std::io::stdout` --- logs go to standard output, where a log aggregator picks them up. In tests, you might pass `std::io::sink()` (discard all output) to keep test output clean. The function does not need to know or care --- it works with any type that can produce a writer.
>
> The `for<'a> MakeWriter<'a>` bound is an advanced feature called a Higher-Ranked Trait Bound (HRTB). You do not need to understand the details yet --- just know that it means "this type can create writers with any lifetime." `std::io::stdout` satisfies this because it can always create a new stdout handle when asked.

---

## Request Tracing with Tower Middleware

### The middleware stack

> **Programming Concept: What is Middleware?**
>
> Middleware is code that runs before and/or after every request. Think of it like checkpoints at an airport:
>
> 1. **Security check** (authentication middleware) --- verifies your identity
> 2. **Baggage scan** (request ID middleware) --- tags your luggage with a tracking number
> 3. **Customs declaration** (tracing middleware) --- records what you are bringing in and measures how long processing takes
>
> Each checkpoint processes the traveler (request) and passes them to the next one. In Axum, middleware layers wrap around your routes, forming an "onion" --- each layer adds behavior around the core request handler.

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

Axum applies layers in **reverse order** --- the last `.layer()` call runs first. So the execution order for each request is:

1. **`PropagateRequestIdLayer`** --- copies the incoming `x-request-id` header to the response. If a load balancer or API gateway set a request ID, it passes through.
2. **`SetRequestIdLayer`** --- if no `x-request-id` header exists, generates a new UUID and sets it. `MakeRequestUuid` generates UUIDs using `uuid::Uuid::new_v4()`.
3. **`TraceLayer`** --- creates a tracing span for the request, logs the response with status and latency.
4. **`SessionManagerLayer`** --- manages the session cookie.

> **Programming Concept: Why Reverse Order?**
>
> Think of wrapping a present. The first wrapping you add (innermost) is the last one the recipient removes. Axum's `.layer()` calls work the same way --- each layer wraps around the previous ones.
>
> ```
> PropagateRequestIdLayer (outermost --- runs first)
>   SetRequestIdLayer
>     TraceLayer
>       SessionManagerLayer (innermost --- runs last on the way in, first on the way out)
>         Your route handler
> ```
>
> On the way in (request), layers run outer to inner. On the way out (response), layers run inner to outer. This is why `PropagateRequestIdLayer` is added last but runs first --- it needs to capture the incoming `x-request-id` header before any other layer modifies it.

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

The span is created with `info_span!` --- it only appears in logs when the log level is INFO or higher. The fields use the `%` sigil for `Display` formatting. Two fields --- `status` and `latency_ms` --- are declared as `Empty`. They are populated later in the `on_response` callback:

```rust
span.record("status", response.status().as_u16());
span.record("latency_ms", latency.as_millis() as u64);
```

Why declare empty fields upfront? Because tracing spans are **write-once** --- you can only record values for fields that were declared when the span was created. If you omit `status` from the `info_span!` macro, `span.record("status", ...)` would silently do nothing. The `Empty` placeholder reserves the slot for later filling.

### Request ID propagation

The request ID serves two purposes:

1. **Correlation** --- when debugging an issue, you can filter all log lines for a specific request: `jq 'select(.request_id == "abc-123")'`. This is invaluable when multiple requests are interleaved in the log stream.

2. **Distributed tracing** --- if GrindIt calls external services, the request ID can be forwarded as a header. The external service includes it in its logs, creating a trace across service boundaries.

```rust
.layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
.layer(PropagateRequestIdLayer::x_request_id())
```

`SetRequestIdLayer` generates a UUID for each incoming request that does not already have an `x-request-id` header. `PropagateRequestIdLayer` copies the `x-request-id` from the request to the response, so clients (or intermediaries) can see the ID.

### System Design: Observability

Observability has three pillars:

1. **Logs** --- discrete events with context (what happened, when, in which request). GrindIt implements this with tracing + Bunyan.
2. **Metrics** --- numeric measurements over time (request count, latency percentiles, error rate). GrindIt does not implement metrics yet, but `tower-http` provides `MetricsLayer` for future use.
3. **Traces** --- the path of a request through the system. GrindIt's request ID propagation is the foundation --- it enables correlating logs across a single request. Full distributed tracing (OpenTelemetry, Jaeger) would be the next step for a microservices architecture.

The structured JSON logs are the most important investment. With structured logs:

- **Debugging** becomes `jq` queries instead of `grep` guesswork
- **Alerting** becomes possible --- a monitoring tool can parse JSON and trigger on `level >= 50` (error)
- **Performance analysis** becomes easy --- filter by latency to find slow requests

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

The startup order matters --- each step depends on the previous one:

1. **`dotenvy::dotenv()`** --- loads `.env` file into environment variables. The `.ok()` ignores errors (the file may not exist in production, where env vars are set by the platform).
2. **Subscriber initialization** --- must happen before any `tracing::info!` calls. After this point, all log events are captured.
3. **Configuration loading** --- reads YAML files and env vars. `expect()` panics on failure --- a misconfigured server should not start.
4. **Database pool** --- `connect_lazy_with` does not connect yet. The first query triggers the connection.
5. **Migrations** --- runs pending SQL migrations. This is the one place where the server blocks on the database at startup.
6. **Storage backend** --- constructed from config, wrapped in `Arc` for sharing (Chapter 13).
7. **Router + middleware** --- the middleware stack is applied, and the server starts listening.

The `tracing::info!` calls at key points (storage backend, listening address) create breadcrumbs in the log. When debugging a startup issue, these messages confirm which stages completed successfully.

When you run the server, you should see JSON output in the terminal:

```json
{"v":0,"name":"gritwit","msg":"storage backend: local","level":30,"time":"..."}
{"v":0,"name":"gritwit","msg":"listening on http://127.0.0.1:3000","level":30,"time":"..."}
```

When you make a request, you see the request span with method, URI, status, and latency:

```json
{"v":0,"name":"gritwit","msg":"response","level":30,"method":"GET","uri":"/","request_id":"a1b2c3d4","status":200,"latency_ms":12}
```

---

## Rust Gym

These drills practice serde deserialization and type conversion patterns in isolation.

### Drill 1: Implement TryFrom for a simple enum

<details>
<summary>Exercise: implement TryFrom&lt;String&gt; for a LogLevel enum</summary>

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

Notice how `"warn" | "warning"` accepts two spellings for the same level. This is a common pattern when parsing user input --- be generous in what you accept, strict in what you produce.
</details>

### Drill 2: Deserialize a struct from YAML

<details>
<summary>Exercise: define a struct and deserialize it from a YAML string</summary>

```rust
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct WorkoutConfig {
    name: String,
    max_duration_minutes: u32,
    allow_modifications: bool,
}

fn main() {
    let yaml = r#"
        name: "Morning WOD"
        max_duration_minutes: 60
        allow_modifications: true
    "#;

    let config: WorkoutConfig = serde_yaml::from_str(yaml)
        .expect("Failed to parse YAML");

    println!("{:?}", config);
    // WorkoutConfig { name: "Morning WOD", max_duration_minutes: 60, allow_modifications: true }
}
```

The `r#"..."#` syntax is a raw string literal --- it does not process escape characters, so you can include quotes and newlines without escaping. The indentation in the YAML string does not matter as long as it is consistent.
</details>

### Drill 3: Use Secret\<String\> to protect a value

<details>
<summary>Exercise: create a struct with a Secret field and verify it is redacted in Debug output</summary>

```rust
use secrecy::{Secret, ExposeSecret};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ApiConfig {
    endpoint: String,
    #[serde(rename = "api_key")]
    key: Secret<String>,
}

fn main() {
    // Debug output redacts the secret
    let config = ApiConfig {
        endpoint: "https://api.example.com".to_string(),
        key: Secret::new("sk_live_abc123".to_string()),
    };

    println!("{:?}", config);
    // ApiConfig { endpoint: "https://api.example.com", key: Secret([REDACTED]) }

    // Must explicitly opt in to access the secret
    println!("Key: {}", config.key.expose_secret());
    // Key: sk_live_abc123
}
```
</details>

---

## Exercises

### Exercise 1: Build Settings struct hierarchy

**Goal:** Define the full configuration type system in `src/configuration.rs`.

**Instructions:**

1. Define `Settings`, `ApplicationSettings`, `DatabaseSettings`, `OAuthSettings`, `StorageSettings`, and `SmsSettings`
2. All structs need `#[derive(Deserialize, Clone)]`
3. Use `Secret<String>` for: `database.password`, `oauth.google_client_id`, `oauth.google_client_secret`, `storage.r2_access_key`, `storage.r2_secret_key`, `sms.api_key`
4. Use `#[serde(deserialize_with = "deserialize_number_from_string")]` for port fields (both `application.port` and `database.port`)
5. Use `#[serde(default)]` on `sms: Option<SmsSettings>` to make SMS optional
6. Make R2 fields in `StorageSettings` optional with `Option<...>`

<details>
<summary>Hint 1: The imports you need</summary>

```rust
use config::{Config, File};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
```

The `serde_aux` crate provides helper functions for common deserialization needs. `deserialize_number_from_string` handles the case where a YAML file has `port: 3000` (number) but an environment variable has `APP_APPLICATION__PORT=3000` (string).
</details>

<details>
<summary>Hint 2: Why Secret&lt;String&gt; works with serde</summary>

The `secrecy` crate implements `Deserialize` for `Secret<String>` automatically. You do not need any special attribute --- serde treats it like a regular `String` during deserialization, but wraps it in `Secret` afterward. The `Debug` and `Display` implementations are what change --- they show `[REDACTED]` instead of the actual value.
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

Notice `DatabaseSettings` does not derive `Debug` --- it contains a `Secret<String>`, and while `Secret` does implement `Debug` (printing `[REDACTED]`), omitting `Debug` on the parent is an extra safety measure. `ApplicationSettings` derives `Debug` because it contains no secrets.
</details>

### Exercise 2: Implement YAML loading with base + environment overlay

**Goal:** Write the `get_configuration()` function and the `Environment` enum.

**Instructions:**

1. Define `Environment` with `Local` and `Production` variants
2. Implement `as_str()` to return the filename-friendly name
3. Implement `TryFrom<String>` with case-insensitive parsing and a clear error message
4. Write `get_configuration()` that:
   - Reads `APP_ENVIRONMENT` from the environment, defaulting to `"local"`
   - Converts it to `Environment` with `.try_into()`
   - Builds a `Config` with three sources: `base.yaml`, environment overlay, and `APP_*` env vars
   - Deserializes into `Settings`
5. Create `configuration/base.yaml` with all default values
6. Create `configuration/local.yaml` with local overrides (if any)

<details>
<summary>Hint: The environment variable naming convention</summary>

```rust
config::Environment::with_prefix("APP")
    .prefix_separator("_")
    .separator("__")
```

This means:
- `APP_APPLICATION__PORT=8080` maps to `application.port = 8080`
- `APP_DATABASE__PASSWORD=secret` maps to `database.password = "secret"`
- The `APP_` prefix is stripped, the `__` double underscore separates nesting levels
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

The layered merge order --- base, then environment, then env vars --- means each layer can override the previous. Environment variables have the highest priority, which is the standard pattern for containerized deployments where secrets are injected as env vars.
</details>

### Exercise 3: Set up tracing with Bunyan formatter and TraceLayer middleware

**Goal:** Create the structured logging system and attach it to the Axum router.

**Instructions:**

1. Create `src/telemetry.rs` with two functions:
   - `get_subscriber(name, env_filter, sink)` --- builds a layered subscriber with `EnvFilter`, `JsonStorageLayer`, and `BunyanFormattingLayer`
   - `init_subscriber(subscriber)` --- installs the subscriber globally and bridges the `log` crate
2. In `main.rs`, initialize the subscriber before any other code
3. Add `TraceLayer` to the router with a custom `make_span_with` that includes: method, URI, request_id (from headers), and empty slots for status and latency_ms
4. Add an `on_response` callback that fills the status and latency_ms fields

<details>
<summary>Hint 1: The Registry + layers pattern</summary>

```rust
Registry::default()
    .with(env_filter)        // layer 1: filter by level
    .with(JsonStorageLayer)  // layer 2: collect span fields
    .with(formatting_layer)  // layer 3: format as JSON
```

Each `.with()` adds a layer. The layers are processed in order for each event.
</details>

<details>
<summary>Hint 2: Empty span fields</summary>

```rust
tracing::info_span!(
    "http_request",
    status = tracing::field::Empty,      // filled later
    latency_ms = tracing::field::Empty,  // filled later
)
```

Declare fields as `Empty` when creating the span, then fill them with `span.record()` when the data becomes available (in `on_response`).
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

The `get_subscriber` function is generic over the output sink, making it testable --- in tests, pass `std::io::sink()` to suppress log output, or pass a buffer to capture logs for assertions.
</details>

### Exercise 4: Add request ID propagation (SetRequestIdLayer + PropagateRequestIdLayer)

**Goal:** Every request gets a unique ID that appears in logs and in the response headers.

**Instructions:**

1. Import `tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer}`
2. Add `SetRequestIdLayer::x_request_id(MakeRequestUuid)` to the middleware stack --- this generates a UUID for requests that do not already have one
3. Add `PropagateRequestIdLayer::x_request_id()` to the middleware stack --- this copies the ID from the request to the response
4. Remember: layers run in reverse order. `PropagateRequestIdLayer` must be added **after** `SetRequestIdLayer` in code (so it runs **before** it in execution order)
5. The `TraceLayer` must run after both ID layers, so it can read the `x-request-id` header

<details>
<summary>Hint: Verifying the request ID</summary>

After implementing this, you can verify it works:

```bash
# Make a request and check the response headers
curl -v http://localhost:3000/

# You should see in the response headers:
# x-request-id: a1b2c3d4-e5f6-7890-abcd-ef1234567890

# The same ID appears in the server's JSON log output
```
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

## DSA in Context

This chapter does not introduce a new DSA pattern. The configuration and telemetry systems are infrastructure --- they support the features built in other chapters rather than implementing algorithmic logic. The layered configuration merge could be viewed as a **priority queue** (higher-priority sources override lower ones), but that framing is a stretch for interview purposes.

The more relevant interview connection is the middleware stack, which was covered in the System Design Corner.

---

## System Design Corner: Observability

**Interview question:** "Design a logging system for a web application serving 10K requests per second."

**What we just built:** Structured JSON logging with request ID propagation and layered middleware.

**Talking points:**

- **Structured vs unstructured logs** --- JSON logs are 2x larger but enable automated querying. At 10K req/s, you cannot read logs manually --- you need `jq`, Elasticsearch, or Datadog to find needles in the haystack.
- **Request ID correlation** --- every log line from a single request shares a request ID. This turns "find the error" from a global search into a targeted filter. Without request IDs, debugging concurrent requests is nearly impossible.
- **Log levels** --- `info` for production (3 log lines per request: start, response, error). `debug` for development (dozens of lines per request). The `EnvFilter` lets you change levels without recompiling.
- **Sampling** --- at 10K req/s, logging every request produces ~26 GB/day of JSON. In practice, you would sample (log 1 in 10 requests at `info`, all requests at `error`) or use a log aggregation service with retention policies.
- **The three pillars** --- logs (what happened), metrics (how much), traces (the path). GrindIt implements logs with the foundation for traces (request IDs). Metrics would be the next investment.

---

## Design Insight: Complexity Layers

> John Ousterhout's *A Philosophy of Software Design* (Ch. 7) describes how well-designed systems use layers of abstraction to manage complexity. The configuration system exemplifies this:
>
> - **Layer 1 (YAML files)** hides from humans the fact that some values will be overridden by environment variables.
> - **Layer 2 (config crate)** hides from Rust code the fact that values came from YAML, env vars, or defaults.
> - **Layer 3 (typed structs)** hides from application code the fact that `port` was originally a string `"3000"` in an env var.
>
> Application code calls `settings.application.port` and gets a `u16`. It does not know or care about layers 1 and 2. This is information hiding in action --- each layer absorbs complexity so the layers above do not have to deal with it.

---

## What You Built

This chapter built the infrastructure layer that makes GrindIt production-ready:

- **Layered configuration** --- `base.yaml` provides defaults, environment files override per deployment, environment variables override everything. The `config` crate merges all three sources and serde deserializes into strongly typed structs. A typo in a field name is a startup error, not a runtime crash.
- **`Secret<T>`** --- prevents accidental logging of passwords and API keys. `expose_secret()` is the only way to access the inner value, making secret access grep-able and reviewable.
- **`TryFrom<String>`** --- the idiomatic pattern for parsing strings into validated types. The `Environment` enum only accepts "local" or "production" --- invalid values are caught at startup.
- **Structured logging** --- Bunyan JSON format with `tracing`, `JsonStorageLayer`, and `BunyanFormattingLayer`. Machine-parseable logs enable `jq` queries, alerting, and performance analysis.
- **Request tracing** --- `TraceLayer` creates spans with method, URI, request ID, status, and latency. `SetRequestIdLayer` generates UUIDs. `PropagateRequestIdLayer` forwards IDs to responses.

If you run `cargo leptos watch` and open your browser to `http://localhost:3000`, you should see structured JSON logs flowing in the terminal. Each request produces a log line with the method, URI, status code, latency, and a unique request ID. Try making several requests and filtering the output with `jq`:

```bash
cargo leptos watch 2>&1 | jq 'select(.msg == "response")'
```

Together, these systems ensure that when something goes wrong in production, you can find out what happened, which request caused it, and what configuration was active --- without adding `println!` statements and redeploying.

The next chapter builds the REST API layer with OpenAPI documentation and Swagger UI.
