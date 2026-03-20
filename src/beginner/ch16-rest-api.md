# Chapter 16: REST API Layer

GrindIt has two kinds of consumers. The Leptos frontend calls server functions --- Rust functions that execute on the server and return typed data directly to the component. But mobile apps, third-party integrations, and monitoring dashboards need something more universal: a REST API with JSON responses, versioned URLs, and machine-readable documentation. This chapter builds the REST API layer alongside the existing server functions, sharing the same database functions with zero business logic duplication.

The spotlight concept is **Axum route organization and API design** --- how to structure routes with `Router::nest` for versioning, how to annotate handlers with `#[utoipa::path]` for automatic OpenAPI documentation, how middleware layers compose in Axum's Tower-based architecture, and how to integrate SwaggerUi for interactive API exploration.

By the end of this chapter, you will have:

- A versioned API router at `/api/v1/` with health check and upload endpoints
- `#[utoipa::path]` annotations generating OpenAPI 3.0 documentation
- An `ApiDoc` struct deriving `OpenApi` that aggregates all documented endpoints
- SwaggerUi served at `/api/swagger-ui` with the OpenAPI spec at `/api/openapi.json`
- Middleware layer ordering that applies tracing, request IDs, and sessions to all routes
- A clear "two doors, one database" architecture where REST handlers and server functions call the same `db.rs` functions

---

## Spotlight: Axum Route Organization & API Design

### Two doors, one database

> **Programming Concept: What is an API?**
>
> API stands for Application Programming Interface. It is a set of rules that let one program talk to another. When you use a weather app on your phone, the app does not calculate the weather itself --- it sends a request to a weather service's API and gets back data (temperature, humidity, forecast) in a structured format.
>
> There are different kinds of APIs:
>
> - **REST API** --- the most common type for web services. It uses standard HTTP methods (GET, POST, PUT, DELETE) and URLs to identify resources. For example, `GET /api/v1/exercises` returns a list of exercises as JSON.
> - **Server functions** --- Leptos's approach. A Rust function marked with `#[server]` runs on the server and returns typed data directly to the frontend component. No manual JSON parsing needed.
>
> Think of a restaurant: the dining room entrance (server functions) is for regular customers who sit down and get served. The takeout window (REST API) serves anyone who walks up --- delivery drivers, neighboring businesses, or passersby. Both entrances lead to the same kitchen (database).
>
> REST stands for Representational State Transfer. The key ideas:
> - **Resources** have URLs: `/api/v1/exercises` is the exercises resource
> - **HTTP methods** describe actions: GET (read), POST (create), PUT (update), DELETE (remove)
> - **JSON** is the standard response format: `{"id": "abc", "name": "Back Squat"}`
> - **Status codes** indicate results: 200 (success), 404 (not found), 500 (server error)

Most web frameworks force a choice: server-rendered pages or API endpoints. Leptos with Axum gives you both. Server functions are the "front door" --- they are called by the Leptos frontend via HTTP POST requests that Leptos manages automatically. The REST API is the "side door" --- it serves external clients with standard HTTP methods and JSON responses.

Both doors lead to the same room: the `db.rs` module. A server function that lists exercises calls `list_exercises_db(&pool).await`. A REST endpoint that lists exercises calls the same function. There is no intermediate "service layer" that exists just to satisfy an architecture diagram. The database functions are the business logic.

```
┌─────────────────┐     ┌──────────────────┐
│  Leptos Frontend │     │  Mobile App / CLI │
│  (server fns)    │     │  (REST API)       │
└────────┬─────────┘     └────────┬──────────┘
         │                        │
         │   #[server]            │   GET /api/v1/exercises
         │   list_exercises()     │   list_exercises_handler()
         │                        │
         └──────────┬─────────────┘
                    │
                    ▼
            ┌───────────────┐
            │    db.rs      │
            │ list_exercises_db()
            └───────┬───────┘
                    │
                    ▼
            ┌───────────────┐
            │  PostgreSQL    │
            └───────────────┘
```

This "two doors, one database" architecture is powerful because it eliminates code duplication. If you fix a bug in `list_exercises_db()`, both the Leptos frontend and the REST API get the fix automatically. If you add a new field to the Exercise struct, both consumers see it. The database functions are the single source of truth.

### Router::nest for versioning

> **Programming Concept: What is API Versioning?**
>
> Imagine you release a mobile app that calls `GET /api/exercises`. A year later, you need to change the response format --- different field names, new structure. But users who have not updated the app still expect the old format.
>
> Versioning solves this by putting a version number in the URL: `/api/v1/exercises` is version 1, `/api/v2/exercises` is version 2. Old clients keep calling v1 (which still works), while new clients use v2. Eventually, you deprecate v1 when no one uses it anymore.
>
> This is why you see `/api/v1/` in many APIs. It is a promise: "this URL will keep working the same way until we explicitly announce its retirement."

Axum's `Router::nest` method prefixes all routes in a sub-router with a path segment:

```rust
let api_routes = Router::new()
    .route("/health_check", get(health_check))
    .route("/exercises", get(list_exercises))
    .route("/exercises/{id}", get(get_exercise));

let app = Router::new()
    .nest("/api/v1", api_routes);
```

The resulting routes are `/api/v1/health_check`, `/api/v1/exercises`, and `/api/v1/exercises/{id}`. When you need breaking changes, you create a `v2` router and nest it at `/api/v2` --- the `v1` routes continue working until you deprecate them.

The nesting is more than URL cosmetics. Each nested router can have its own state and middleware. The API router uses `UploadState` (containing the storage backend and pool), while the auth router uses `OAuthState` (containing the OAuth client and pool). Axum's type system enforces that each handler receives the state type it declared:

```rust
let api_routes = Router::new()
    .route("/upload/video", post(upload_video))
    .with_state(upload_state);

let auth_routes = Router::new()
    .route("/auth/google/login", get(google_login))
    .with_state(oauth_state);

let app = Router::new()
    .nest("/api/v1", api_routes)
    .merge(auth_routes);
```

> **Programming Concept: What is State in a Web Server?**
>
> When a web request arrives, the handler function needs access to shared resources like the database connection pool or a storage backend. But handler functions are called fresh for each request --- they do not have persistent memory between calls.
>
> "State" is data that the server holds between requests and shares with every handler that needs it. In Axum, you create a state struct, attach it to a router with `.with_state()`, and handlers declare it as a parameter:
>
> ```rust
> #[derive(Clone)]
> struct AppState {
>     pool: PgPool,
> }
>
> async fn list_exercises(
>     State(state): State<AppState>,  // Axum injects this automatically
> ) -> Json<Vec<Exercise>> {
>     let exercises = list_exercises_db(&state.pool).await.unwrap();
>     Json(exercises)
> }
> ```
>
> The `State(state)` parameter is not passed by the caller. Axum extracts it from the router's state and hands it to the function. This is called **dependency injection** --- the framework provides dependencies automatically instead of forcing you to pass them manually.

### Handler state extraction

Axum handlers declare their dependencies as function parameters. The framework extracts each parameter from the incoming request:

```rust
pub async fn upload_video(
    State(state): State<UploadState>,
    session: Session,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    // ...
}
```

`State(state)` extracts the shared application state. `session` extracts the session from the cookie (managed by `SessionManagerLayer`). `multipart` extracts the multipart form data. The return type `Result<Json<UploadResponse>, (StatusCode, String)>` becomes a JSON success response or an error with a status code and message.

This is **dependency injection at the type level**. The handler does not call `app.get_state()` or `req.session()` --- it declares what it needs, and the framework provides it. If a required extractor fails (no session cookie, invalid multipart data), Axum returns an appropriate error response before the handler body executes.

### utoipa for OpenAPI documentation

> **Programming Concept: What is OpenAPI / Swagger?**
>
> When you build an API, other developers need to know what endpoints exist, what parameters they accept, and what responses they return. You could write this documentation by hand in a wiki or a README, but it would go out of date as soon as someone changes the code.
>
> OpenAPI (formerly called Swagger) is a standard format for describing APIs. It is a JSON or YAML file that lists every endpoint with its parameters, request body, and response types. Tools like Swagger UI read this file and generate an interactive web page where developers can read the documentation and test endpoints directly in the browser.
>
> The `utoipa` crate generates this OpenAPI specification automatically from your Rust code. You annotate your handler functions with `#[utoipa::path]`, and utoipa produces the specification at compile time. If you change the code, the documentation updates automatically --- it cannot go out of date.

The `utoipa` crate generates OpenAPI 3.0 specifications from Rust code. You annotate handlers with `#[utoipa::path]` and aggregate them in an `#[derive(OpenApi)]` struct:

```rust
#[utoipa::path(
    get,
    path = "/api/v1/health_check",
    tag = "v1",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
```

The `#[utoipa::path]` attribute is a procedural macro. It does not change the function's behavior --- it generates metadata that `utoipa` collects. The `path` must match the actual mounted path (including the `/api/v1` prefix from nesting). The `tag` groups endpoints in the Swagger UI sidebar. The `responses` section documents the possible HTTP responses.

The `ApiDoc` struct collects all documented endpoints:

```rust
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check::health_check,
    ),
    tags(
        (name = "v1", description = "API v1 endpoints")
    )
)]
pub struct ApiDoc;
```

Calling `ApiDoc::openapi()` produces an `openapi::OpenApi` struct that serializes to the OpenAPI JSON specification. SwaggerUi serves this specification and renders an interactive documentation page.

### SwaggerUi integration

```rust
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

let app = Router::new()
    .nest("/api/v1", api_routes)
    .merge(
        SwaggerUi::new("/api/swagger-ui")
            .url("/api/openapi.json", ApiDoc::openapi())
    );
```

`SwaggerUi::new("/api/swagger-ui")` mounts the Swagger UI static assets at that path. The `.url("/api/openapi.json", ApiDoc::openapi())` call serves the OpenAPI JSON spec at `/api/openapi.json` and tells the Swagger UI to load it. Visit `http://localhost:3000/api/swagger-ui` in a browser to see the interactive documentation.

SwaggerUi is added with `.merge()`, not `.nest()`. Merging combines two routers at the same path level. Nesting would prefix the Swagger paths, breaking the internal links between the UI and the spec.

### Middleware layer ordering

Axum applies `.layer()` calls in **reverse order** --- the last layer added runs first on the incoming request. GrindIt's middleware stack:

```rust
let app = Router::new()
    // ... routes ...
    .layer(session_layer)                           // 4th: session management
    .layer(TraceLayer::new_for_http()               // 3rd: request tracing
        .make_span_with(...)
        .on_response(...))
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))   // 2nd: generate request ID
    .layer(PropagateRequestIdLayer::x_request_id());            // 1st: propagate existing ID
```

Execution order for an incoming request:

1. `PropagateRequestIdLayer` --- captures any incoming `x-request-id` header for propagation to the response
2. `SetRequestIdLayer` --- if no `x-request-id` exists, generates a UUID
3. `TraceLayer` --- creates a tracing span with the request ID (now guaranteed to exist)
4. `SessionManagerLayer` --- loads the session from the cookie

The ordering matters. `TraceLayer` reads the `x-request-id` header in its `make_span_with` closure. If `TraceLayer` ran before `SetRequestIdLayer`, the request ID would be "unknown" for requests that did not arrive with one. The reverse-order application ensures each layer has the data it needs from the layers that ran before it.

> **Programming Concept: Why Does Layer Order Matter?**
>
> Think of layers as people in an assembly line. Each person does one job and passes the item to the next person. But the order matters:
>
> 1. **Label maker** (SetRequestIdLayer) stamps a tracking number on the package
> 2. **Inspector** (TraceLayer) reads the tracking number and records when the package arrives and departs
> 3. **Processor** (SessionManagerLayer) opens the package and handles the contents
>
> If the inspector runs before the label maker, there is no tracking number to read. The result: "unknown" in your logs, which defeats the purpose of tracing.
>
> In Axum, the last `.layer()` call you write in code is the outermost layer --- it runs first on incoming requests. This feels backwards at first, but think of it as wrapping: each `.layer()` wraps around the previous stack, so the last wrap is the outermost shell.

---

## Building the REST API

### The routes module structure

The `src/routes/` directory organizes API-specific code:

```
src/routes/
├── mod.rs           # Module declarations, ApiDoc struct, re-exports
├── health_check.rs  # GET /api/v1/health_check
└── upload.rs        # POST /api/v1/upload/video + UploadState
```

The `mod.rs` file re-exports everything that `main.rs` needs:

```rust
mod health_check;
mod upload;

pub use health_check::health_check;
pub use upload::{upload_video, UploadState};

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check::health_check,
    ),
    tags(
        (name = "v1", description = "API v1 endpoints")
    )
)]
pub struct ApiDoc;
```

The `pub use` re-exports mean `main.rs` imports `use gritwit::routes::{health_check, upload_video, ApiDoc, UploadState}` --- a flat namespace that hides the internal module structure. Adding a new endpoint means creating a new file, adding a `mod` declaration, a `pub use`, and a `paths()` entry in `ApiDoc`.

### The health check handler

The simplest possible handler --- it takes no parameters and returns a status code:

```rust
use axum::http::StatusCode;

#[utoipa::path(
    get,
    path = "/api/v1/health_check",
    tag = "v1",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
```

> **Programming Concept: What is a Health Check?**
>
> A health check is a simple endpoint that answers the question "is this server running?" Load balancers, container orchestrators (like Kubernetes), and monitoring systems call this endpoint repeatedly to verify the service is alive.
>
> If the health check returns 200 (OK), the service is healthy. If it returns an error or times out, the infrastructure knows something is wrong and can restart the service or route traffic elsewhere.
>
> The simplest health check just returns 200 with no body. A more sophisticated one might also check the database connection, disk space, or other dependencies. GrindIt starts with the simple version --- it confirms the process is running and accepting requests.

### The upload handler with shared state

The upload endpoint demonstrates the full pattern: state extraction, authentication, validation, and delegation to a shared module:

```rust
#[derive(Clone)]
pub struct UploadState {
    pub storage: Arc<StorageBackend>,
    pub pool: sqlx::PgPool,
}
```

`UploadState` wraps an `Arc<StorageBackend>` (shared across all request handlers) and a `PgPool` (a connection pool that is already internally shared via `Arc`). The `#[derive(Clone)]` is required by Axum --- state must be cloneable because each request handler receives its own clone.

The handler authenticates via the session, validates the upload (content type, extension, file size, magic bytes), and delegates to `state.storage.upload()`:

```rust
pub async fn upload_video(
    State(state): State<UploadState>,
    session: Session,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    // Auth: extract user_id from session
    let user_id: Option<String> = session.get("user_id").await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let user_id = user_id
        .ok_or((StatusCode::UNAUTHORIZED, "Sign in to upload videos".into()))?;

    // ... validation, upload, response ...
}
```

The same `StorageBackend` and `db.rs` functions are used by both the REST handler and the Leptos server functions. The REST handler adds HTTP-specific concerns (status codes, multipart parsing) but the core logic lives in shared modules.

### Router assembly in main.rs

The full router assembly brings together API routes, auth routes, Leptos routes, and middleware:

```rust
let api_routes = Router::new()
    .route("/health_check", get(health_check))
    .route(
        "/upload/video",
        post(upload_video)
            .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)),
    )
    .with_state(upload_state);

let auth_routes = Router::new()
    .route("/auth/google/login", get(google_login))
    .route("/auth/google/callback", get(google_callback))
    .route("/auth/logout", get(oauth::logout))
    .with_state(oauth_state);

let app = Router::new()
    .nest("/api/v1", api_routes)
    .merge(auth_routes)
    .merge(SwaggerUi::new("/api/swagger-ui")
        .url("/api/openapi.json", ApiDoc::openapi()))
    .leptos_routes_with_context(/* ... */)
    .fallback(leptos_axum::file_and_error_handler(shell))
    .layer(session_layer)
    .layer(TraceLayer::new_for_http()/* ... */)
    .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
    .layer(PropagateRequestIdLayer::x_request_id());
```

Several design decisions worth understanding:

- **Per-route body limits** --- the upload route gets `DefaultBodyLimit::max(100 * 1024 * 1024)` (100 MB). Other routes use Axum's default limit (2 MB). The `.layer()` call on a specific route applies only to that route, not the entire router. This prevents a regular form submission from uploading 100 MB of data.
- **State isolation** --- `api_routes` uses `UploadState`, `auth_routes` uses `OAuthState`. They are merged into the top-level router, which uses `LeptosOptions` as its state. Axum's type system prevents handler/state mismatches --- if you accidentally wire the wrong state, the code does not compile.
- **Fallback ordering** --- `leptos_routes_with_context` handles all Leptos page routes. The `.fallback()` catches everything else and serves static files or a 404 page. API routes are nested at `/api/v1` so they never conflict with Leptos routes.

---

> **Design Insight: Pass-Through Elimination** (Ousterhout, Ch. 7)
>
> A common anti-pattern is the "pass-through method" --- a function that does nothing except call another function with the same arguments. In many web frameworks, you see `controller.list()` calling `service.list()` calling `repository.list()` where the service layer is a pass-through that adds no value. GrindIt eliminates this: REST handlers and server functions both call `db.rs` directly. There is no `ExerciseService` that wraps `ExerciseRepository`. If a handler needs to combine multiple database calls or add business rules, it does so directly. The abstraction exists only where it earns its complexity.

---

## Exercises

### Exercise 1: Create the health check endpoint with OpenAPI annotation

**Goal:** Build a `GET /api/v1/health_check` endpoint that returns `200 OK`, annotated with `#[utoipa::path]` for OpenAPI documentation.

**Instructions:**

1. Create `src/routes/health_check.rs` --- this file will contain a single handler function
2. Write an async handler function called `health_check` that takes no parameters and returns `StatusCode::OK`
3. Add a `#[utoipa::path]` attribute above the function. This attribute tells utoipa to include this endpoint in the generated documentation. It needs four pieces of information: the HTTP method (`get`), the full mounted path (`"/api/v1/health_check"`), a tag for grouping in the Swagger UI sidebar (`"v1"`), and the expected response (status 200 with a description)
4. In `src/routes/mod.rs`, declare the `health_check` module with `mod health_check;` and re-export the handler with `pub use health_check::health_check;`

<details>
<summary>Hint 1</summary>

The handler signature is `pub async fn health_check() -> StatusCode`. Axum automatically converts `StatusCode` into an HTTP response with no body. You need `use axum::http::StatusCode;` at the top of the file.
</details>

<details>
<summary>Hint 2</summary>

The `#[utoipa::path]` attribute goes directly above the function. The `path` must include the full mounted path: `"/api/v1/health_check"`, not just `"/health_check"`. Even though the route is registered as `"/health_check"` in the router (before nesting), the OpenAPI spec needs the complete URL that clients will call.
</details>

<details>
<summary>Solution</summary>

```rust
// src/routes/health_check.rs
use axum::http::StatusCode;

#[utoipa::path(
    get,
    path = "/api/v1/health_check",
    tag = "v1",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
pub async fn health_check() -> StatusCode {
    StatusCode::OK
}
```

```rust
// src/routes/mod.rs
mod health_check;

pub use health_check::health_check;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check::health_check,
    ),
    tags(
        (name = "v1", description = "API v1 endpoints")
    )
)]
pub struct ApiDoc;
```

Test with curl:
```bash
curl -i http://localhost:3000/api/v1/health_check
# HTTP/1.1 200 OK
```
</details>

### Exercise 2: Build the UploadState and upload handler

**Goal:** Create a video upload endpoint at `POST /api/v1/upload/video` that authenticates via session, validates the file, and delegates to `StorageBackend`.

**Instructions:**

1. Create `src/routes/upload.rs`
2. Define `UploadState` with two fields: `storage: Arc<StorageBackend>` (the storage backend from Chapter 13, wrapped in Arc for sharing) and `pool: PgPool` (the database connection pool). Derive `Clone` --- Axum requires state to be cloneable.
3. Define `UploadResponse` with a `url: String` field, deriving `Serialize` so Axum can convert it to JSON
4. Write the `upload_video` handler that extracts `State`, `Session`, and `Multipart`. The function should: extract `user_id` from the session (return 401 if missing), iterate over multipart fields to find the "video" field, validate the content type starts with "video/", check the file extension is in the allowed list, verify the file size is under 100 MB, validate the file's magic bytes (first few bytes that identify the format), then upload using `state.storage.upload()`
5. Re-export `UploadState` and `upload_video` from `mod.rs`

<details>
<summary>Hint 1</summary>

The return type is `Result<Json<UploadResponse>, (StatusCode, String)>`. Axum converts the `Ok` variant to a JSON response and the `Err` variant to an error response with the given status code and message body. The `?` operator works here to propagate errors early.
</details>

<details>
<summary>Hint 2</summary>

Use `multipart.next_field().await` in a `while let Some(field)` loop to iterate over form fields. Check `field.name()` to find the `"video"` field. Call `field.bytes().await` to read the file data. Fields you do not care about should be skipped with `continue`.
</details>

<details>
<summary>Hint 3</summary>

Magic byte validation checks the first bytes of the file data to verify the file is actually a video (not a renamed text file). MP4/MOV files have `ftyp` at bytes 4-8. WebM files start with `[0x1A, 0x45, 0xDF, 0xA3]`. AVI files start with `RIFF` and have `AVI ` at bytes 8-12. This prevents users from uploading malicious files by renaming them to `.mp4`.
</details>

<details>
<summary>Solution</summary>

```rust
// src/routes/upload.rs
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Serialize;
use std::sync::Arc;
use tower_sessions::Session;

use crate::storage::StorageBackend;

#[derive(Clone)]
pub struct UploadState {
    pub storage: Arc<StorageBackend>,
    pub pool: sqlx::PgPool,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub url: String,
}

const ALLOWED_EXTENSIONS: &[&str] = &["mp4", "webm", "mov", "avi", "m4v"];
const MAX_UPLOAD_BYTES: usize = 100 * 1024 * 1024;

fn is_valid_video_magic(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    // MP4 / M4V / MOV
    if data[4..8] == *b"ftyp" {
        return true;
    }
    // WebM / MKV — EBML header
    if data[0..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return true;
    }
    // AVI — RIFF....AVI
    if data[0..4] == *b"RIFF" && data[8..12] == *b"AVI " {
        return true;
    }
    false
}

pub async fn upload_video(
    State(state): State<UploadState>,
    session: Session,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    let user_id: Option<String> = session
        .get("user_id")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user_id = user_id
        .ok_or((StatusCode::UNAUTHORIZED, "Sign in to upload videos".into()))?;

    let user_uuid: uuid::Uuid = user_id
        .parse()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid session".into()))?;

    let _user = crate::db::get_user_by_id(&state.pool, user_uuid)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "User not found".into()))?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name != "video" {
            continue;
        }

        let original_name = field.file_name().unwrap_or("video.mp4").to_string();

        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        if !content_type.starts_with("video/") {
            return Err((StatusCode::BAD_REQUEST, "Only video files are allowed".into()));
        }

        let ext = std::path::Path::new(&original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Unsupported file type '.{}'", ext),
            ));
        }

        let data = field.bytes().await.map_err(|e| {
            (StatusCode::BAD_REQUEST, format!("Failed to read file: {}", e))
        })?;

        if data.len() > MAX_UPLOAD_BYTES {
            return Err((StatusCode::PAYLOAD_TOO_LARGE, "Video must be under 100 MB".into()));
        }

        if !is_valid_video_magic(&data) {
            return Err((
                StatusCode::BAD_REQUEST,
                "File content does not match a supported video format".into(),
            ));
        }

        let key = format!("{}.{}", uuid::Uuid::new_v4(), ext);

        let url = state
            .storage
            .upload(&key, &data, &content_type)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        tracing::info!(
            user_id = %user_uuid,
            file = %original_name,
            size_bytes = data.len(),
            "video uploaded"
        );

        return Ok(Json(UploadResponse { url }));
    }

    Err((StatusCode::BAD_REQUEST, "No video field found".into()))
}
```
</details>

### Exercise 3: Wire the API router with versioned nesting and SwaggerUi

**Goal:** In `main.rs`, assemble the API router with `Router::nest("/api/v1", ...)`, merge auth routes, add SwaggerUi, and connect Leptos routes.

**Instructions:**

1. Create `api_routes` --- a `Router::new()` with the health check (`GET /health_check`) and upload (`POST /upload/video`) endpoints. Attach `UploadState` with `.with_state()`. Add a per-route body limit of 100 MB to the upload route using `.layer(DefaultBodyLimit::max(100 * 1024 * 1024))`.
2. Create `auth_routes` --- a `Router::new()` with the OAuth endpoints (`/auth/google/login`, `/auth/google/callback`, `/auth/logout`). Attach `OAuthState` with `.with_state()`.
3. Build the top-level router: nest API routes at `/api/v1` with `.nest()`, merge auth routes with `.merge()`, merge SwaggerUi with `.merge()`. Then add Leptos routes and the fallback handler.
4. Apply middleware layers. Remember: the last `.layer()` added runs first. The correct order in code (bottom to top) is: `PropagateRequestIdLayer`, `SetRequestIdLayer`, `TraceLayer`, `SessionManagerLayer`.

<details>
<summary>Hint 1</summary>

The upload route needs a per-route body limit: `.route("/upload/video", post(upload_video).layer(DefaultBodyLimit::max(100 * 1024 * 1024)))`. This applies only to that route, not the entire router. Without it, Axum's default 2 MB limit would reject any video larger than 2 MB.
</details>

<details>
<summary>Hint 2</summary>

SwaggerUi is merged (not nested): `.merge(SwaggerUi::new("/api/swagger-ui").url("/api/openapi.json", ApiDoc::openapi()))`. The `url` method serves the OpenAPI spec and tells the UI where to find it. Using `.nest()` instead of `.merge()` would add a prefix to the Swagger UI's internal paths, breaking the links.
</details>

<details>
<summary>Hint 3</summary>

Layer order is reverse of execution order. From bottom to top in the code: `PropagateRequestIdLayer` (runs first), `SetRequestIdLayer`, `TraceLayer`, `SessionManagerLayer` (runs last before the handler). If you get this wrong, request IDs will not appear in trace logs.
</details>

<details>
<summary>Solution</summary>

```rust
let upload_state = UploadState {
    storage: storage.clone(),
    pool: pool.clone(),
};

let api_routes = Router::new()
    .route("/health_check", get(health_check))
    .route(
        "/upload/video",
        post(upload_video)
            .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)),
    )
    .with_state(upload_state);

let auth_routes = Router::new()
    .route("/auth/google/login", get(oauth::google_login))
    .route("/auth/google/callback", get(oauth::google_callback))
    .route("/auth/logout", get(oauth::logout))
    .with_state(oauth_state);

let app = Router::new()
    .nest("/api/v1", api_routes)
    .merge(auth_routes)
    .merge(
        SwaggerUi::new("/api/swagger-ui")
            .url("/api/openapi.json", ApiDoc::openapi()),
    )
    .leptos_routes_with_context(
        &leptos_options,
        routes,
        {
            let pool = pool.clone();
            move || provide_context(pool.clone())
        },
        {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        },
    )
    .fallback(leptos_axum::file_and_error_handler(shell))
    .layer(session_layer)
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                let request_id = request
                    .headers()
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
    .layer(PropagateRequestIdLayer::x_request_id())
    .with_state(leptos_options);
```

Verify: `curl http://localhost:3000/api/v1/health_check` returns 200, and `http://localhost:3000/api/swagger-ui` renders the interactive documentation.
</details>

### Exercise 4: Add a REST endpoint for listing exercises

**Goal:** Add `GET /api/v1/exercises` that returns all exercises as JSON, calling the same `list_exercises_db` function used by the Leptos server function.

**Instructions:**

1. Create a new file `src/routes/exercises.rs` for the exercises endpoint
2. Write a handler function called `list_exercises` that extracts `State<UploadState>` (reusing the existing state since it contains the pool) and calls `crate::db::list_exercises_db(&state.pool).await`
3. The handler should return `Result<Json<Vec<crate::db::Exercise>>, (StatusCode, String)>` --- on success, Axum serializes the exercise list to JSON; on error, it returns the status code and message
4. Add a `#[utoipa::path]` annotation with the full path `/api/v1/exercises`, tag `"v1"`, and a 200 response description
5. In `src/routes/mod.rs`, add `mod exercises;`, add `pub use exercises::list_exercises;`, and add `exercises::list_exercises` to the `paths()` list in `ApiDoc`
6. In `main.rs`, add `.route("/exercises", get(list_exercises))` to the `api_routes` router

This is the "two doors, one database" pattern in action. The Leptos server function `list_exercises()` and the REST handler `list_exercises()` both call `list_exercises_db()`. No duplication, no intermediary.

<details>
<summary>Hint 1</summary>

The handler signature: `pub async fn list_exercises(State(state): State<UploadState>) -> Result<Json<Vec<Exercise>>, (StatusCode, String)>`. You reuse `UploadState` because it already contains the pool. In a larger application, you might create a more general `ApiState`, but for GrindIt, `UploadState` works fine.
</details>

<details>
<summary>Hint 2</summary>

Call the same function the server function uses: `crate::db::list_exercises_db(&state.pool).await`. Map the error with `.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))`. The `?` operator then either unwraps the Ok value or returns the mapped error early.
</details>

<details>
<summary>Solution</summary>

```rust
// In src/routes/exercises.rs (new file)
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;

use super::upload::UploadState;

#[utoipa::path(
    get,
    path = "/api/v1/exercises",
    tag = "v1",
    responses(
        (status = 200, description = "List of all exercises")
    )
)]
pub async fn list_exercises(
    State(state): State<UploadState>,
) -> Result<Json<Vec<crate::db::Exercise>>, (StatusCode, String)> {
    let exercises = crate::db::list_exercises_db(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(exercises))
}
```

```rust
// Updated src/routes/mod.rs
mod exercises;
mod health_check;
mod upload;

pub use exercises::list_exercises;
pub use health_check::health_check;
pub use upload::{upload_video, UploadState};

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check::health_check,
        exercises::list_exercises,
    ),
    tags(
        (name = "v1", description = "API v1 endpoints")
    )
)]
pub struct ApiDoc;
```

```rust
// Updated api_routes in main.rs
let api_routes = Router::new()
    .route("/health_check", get(health_check))
    .route("/exercises", get(list_exercises))
    .route(
        "/upload/video",
        post(upload_video)
            .layer(axum::extract::DefaultBodyLimit::max(100 * 1024 * 1024)),
    )
    .with_state(upload_state);
```

Test it:
```bash
curl http://localhost:3000/api/v1/exercises | jq
```

You should see the same exercises that the Leptos frontend displays, now available as JSON for any HTTP client.
</details>

---

## Rust Gym: Axum Handler Drills

These drills practice writing Axum handlers in isolation --- extracting parameters, returning JSON, and composing routers.

### Drill 1: Write an Axum handler that returns JSON

<details>
<summary>Exercise</summary>

Write a handler that returns a JSON object with a `status` field and a `timestamp` field. The handler takes no state and returns `Json<StatusResponse>`.

```rust
use axum::response::Json;
use serde::Serialize;

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    timestamp: String,
}

async fn status() -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
```

The `Json` wrapper does two things: it sets the `Content-Type: application/json` header on the response, and it serializes the struct to JSON using serde. The caller receives `{"status":"ok","timestamp":"2024-03-15T10:30:42.123Z"}`.
</details>

### Drill 2: Extract path parameters and query parameters

<details>
<summary>Exercise</summary>

Write a handler for `GET /api/v1/exercises/{id}` that also accepts an optional `?format=brief` query parameter. Path parameters come from the URL path itself (the `{id}` part), while query parameters come after the `?`.

```rust
use axum::extract::{Path, Query};
use serde::Deserialize;

#[derive(Deserialize)]
struct ExerciseQuery {
    format: Option<String>,
}

async fn get_exercise(
    Path(id): Path<uuid::Uuid>,
    Query(query): Query<ExerciseQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let exercise = crate::db::get_exercise_db(id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    match query.format.as_deref() {
        Some("brief") => Ok(Json(serde_json::json!({
            "id": exercise.id,
            "name": exercise.name,
        }))),
        _ => Ok(Json(serde_json::to_value(exercise).unwrap())),
    }
}
```

`Path(id)` extracts from the URL path --- if the URL is `/api/v1/exercises/abc-123`, `id` becomes the UUID `abc-123`. `Query(query)` deserializes query string parameters into a struct --- `?format=brief` becomes `ExerciseQuery { format: Some("brief".to_string()) }`. Both are compile-time typed --- Axum returns 400 Bad Request if the path segment is not a valid UUID or the query string cannot be deserialized.
</details>

### Drill 3: Compose two Axum routers with different state types

<details>
<summary>Exercise</summary>

Create two routers with different state types and merge them into a single application. This demonstrates that each sub-router can carry its own dependencies without forcing a single global state struct.

```rust
#[derive(Clone)]
struct DatabaseState {
    pool: PgPool,
}

#[derive(Clone)]
struct CacheState {
    cache: Arc<DashMap<String, String>>,
}

let db_routes = Router::new()
    .route("/users", get(list_users))
    .with_state(DatabaseState { pool: pool.clone() });

let cache_routes = Router::new()
    .route("/cache/{key}", get(get_cached))
    .with_state(CacheState { cache: Arc::new(DashMap::new()) });

let app = Router::new()
    .nest("/api/v1", db_routes)
    .nest("/api/v1", cache_routes);
```

Each `with_state` call consumes the generic state type, producing a `Router<()>`. The resulting routers can be merged or nested freely because they no longer carry a state type parameter. This is how Axum achieves type-safe state without requiring a single global state struct that contains everything.
</details>

---

## DSA in Context: Middleware as Function Composition

The middleware stack you built in Exercise 3 is function composition. Each layer wraps the next, forming an onion:

```
Request → Propagate → SetId → Trace → Session → Handler
                                                    ↓
Response ← Propagate ← SetId ← Trace ← Session ← Handler
```

In mathematical terms, if each middleware is a function `f`, `g`, `h`, then the composite is `f(g(h(handler)))`. The request flows inward through the layers; the response flows outward.

**Interview version:** Tower's `Layer` trait is exactly this pattern:

```rust
// Simplified Tower Layer
trait Layer<S> {
    type Service;
    fn layer(&self, inner: S) -> Self::Service;
}
```

Each `Layer` takes an inner `Service` and returns a new `Service` that wraps it. This is the decorator pattern applied to async services. The composition is O(1) at runtime --- there is no dynamic dispatch or vtable lookup. The compiler monomorphizes the entire middleware stack into a single type.

**Bonus challenge:** Tower middleware is related to the **chain of responsibility** pattern. Each middleware decides whether to handle the request, modify it, or pass it to the next handler. How would you implement rate limiting as a Tower layer that short-circuits the chain for requests that exceed the limit?

---

## System Design Corner: API Gateway Design

**Interview question:** "Design an API gateway for a multi-service architecture."

**What we just built:** GrindIt's router is a single-service API gateway. It routes requests to different handlers based on the URL path, applies cross-cutting concerns (tracing, sessions, request IDs) via middleware, and serves documentation at `/api/swagger-ui`.

**Talking points:**

- **Versioning strategies** --- URL path (`/api/v1/`), header (`Accept: application/vnd.grindit.v1+json`), or query parameter (`?version=1`). URL path is the simplest and most discoverable. Header versioning is cleaner but harder for clients. GrindIt uses URL path versioning because it is explicit and works with every HTTP client.

- **Rate limiting** --- Tower provides `RateLimitLayer` for per-handler rate limits. For a multi-service gateway, you would use a centralized rate limiter (Redis-backed) that tracks requests per API key. The middleware checks the rate limit before forwarding the request.

- **Authentication consolidation** --- the gateway handles authentication once (via the session layer), and downstream handlers trust the authenticated identity. This avoids every handler implementing its own auth check. GrindIt's upload handler still checks the session, but in a microservices architecture, the gateway would inject the user identity as a header.

- **Documentation aggregation** --- utoipa's `ApiDoc::openapi()` generates the spec at compile time. In a microservices architecture, each service generates its own spec, and the gateway aggregates them. Tools like `utoipa-swagger-ui` can serve multiple specs from a single UI.

- **Multi-client architecture** --- the same API serves mobile apps (JSON), web apps (server functions), and CLI tools (JSON). The content negotiation is implicit: Leptos server functions use a custom binary format, while REST endpoints always return JSON. Both are served by the same process, reducing operational complexity.

---

## What You Built

This chapter built the external-facing REST API layer alongside GrindIt's server functions:

- **Two doors, one database** --- server functions serve the Leptos frontend, REST endpoints serve external clients, and both call the same `db.rs` functions. No duplication, no pass-through layers.
- **`Router::nest("/api/v1", ...)`** --- versioned URL namespacing. Each nested router carries its own state type, enforced at compile time.
- **`#[utoipa::path]`** --- OpenAPI annotations on handlers, aggregated by `#[derive(OpenApi)]` on `ApiDoc`, served as interactive documentation via `SwaggerUi`.
- **Middleware layer ordering** --- Axum applies layers bottom-up. `PropagateRequestIdLayer` runs first because it is added last. Each layer depends on the data provided by layers that ran before it.
- **State extraction** --- handlers declare their dependencies as typed parameters. `State(state)`, `Session`, `Multipart`, `Path`, `Query` --- the framework extracts each one and returns an error if extraction fails.

The REST API makes GrindIt's data accessible to any HTTP client. The Swagger UI at `/api/swagger-ui` makes it discoverable without reading source code. And the shared `db.rs` layer ensures that every client sees the same data, enforced by the same business rules.

The next chapter packages everything into a production Docker image.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds middleware as composable functions — like plates on a barbell, stackable in any order from scratch in Rust — no libraries, just std.

**→ [Middleware Composition](../ds-narratives/ch16-middleware-composition.md)**

---
