# Chapter 13: Video Uploads

Every exercise in the library should be demonstrable. A text description of a Turkish Get-Up helps, but a fifteen-second video removes all ambiguity. This chapter builds the video upload pipeline: file selection on the client, multipart upload to the server, magic byte validation, and storage in either a local directory or Cloudflare R2. The same `upload()` method works for both — the caller never knows which backend is active.

The spotlight concept is **smart pointers and enum-based abstraction** — specifically `Arc<StorageBackend>` for sharing a storage backend across async handlers, and the pattern of using an enum with `match` dispatch as a strategy pattern. You will see how `Arc` enables shared ownership without runtime garbage collection, why `Box` appears inside enum variants for large types, and how enum dispatch compares to trait objects for polymorphism.

By the end of this chapter, you will have:

- A `StorageBackend` enum with `Local` and `R2 { bucket, public_url }` variants
- A `from_config()` constructor that reads `StorageSettings` and builds the correct variant
- An `async fn upload(&self, key, data, content_type)` method with `match` dispatch
- Magic byte validation for MP4, WebM, and AVI containers
- An Axum multipart upload route with auth, size limits, and extension allowlists
- A `VideoUpload` Leptos component with URL paste and file upload modes

---

## Spotlight: Smart Pointers (Arc) & Enum-Based Abstraction

### The problem: sharing state across async handlers

An Axum server spawns a new task for every incoming request. Each task runs on a different thread. If two requests arrive simultaneously and both need the storage backend, they both need access to the same `StorageBackend` value — but Rust's ownership rules say a value can have only one owner.

In JavaScript, this is not a problem:

```javascript
// Node.js: the module-level variable is shared by all request handlers
const s3 = new S3Client({ region: "auto" });

app.post("/upload", async (req, res) => {
  await s3.putObject({ ... }); // every handler shares the same client
});
```

The JavaScript runtime garbage-collects the `s3` client when all references are gone. Rust has no garbage collector. Instead, it provides `Arc` — Atomic Reference Counting.

### Arc: shared ownership with reference counting

`Arc<T>` wraps a value of type `T` and maintains a counter of how many `Arc` pointers reference that value. When you clone an `Arc`, the counter increments. When an `Arc` drops, the counter decrements. When the counter reaches zero, the value is freed.

```rust
use std::sync::Arc;

let storage = Arc::new(StorageBackend::from_config(&config.storage));

// Clone is cheap — it increments a counter, not the data
let storage_for_handler = storage.clone();
let storage_for_another = storage.clone();
// All three point to the same StorageBackend in memory
```

The "Atomic" in `Arc` means the counter uses atomic CPU instructions — safe for concurrent access across threads. This is what makes `Arc` different from `Rc` (Reference Counted), which is single-threaded only. In an async web server, you always want `Arc`.

> **Coming from JS?** TypeScript's discriminated unions (`type Storage = { kind: "local", path: string } | { kind: "r2", bucket: string }`) look similar to Rust enums, but they cannot hold methods or enforce exhaustiveness at compile time. A `switch` on `kind` does not error if you add a new variant. Rust's `match` does.

### The StorageBackend enum

Here is GrindIt's storage abstraction:

```rust
pub enum StorageBackend {
    Local,
    R2 {
        bucket: Box<s3::Bucket>,
        public_url: String,
    },
}
```

Two things to notice:

1. **Asymmetric variants.** `Local` carries no data — it writes to a fixed `public/videos/` directory. `R2` carries a configured S3 bucket and a public URL prefix. This is perfectly legal in Rust enums — each variant can hold different types and amounts of data.

2. **`Box<s3::Bucket>`** — the `Bucket` struct is large (it contains credentials, endpoint URLs, region info). Wrapping it in `Box` moves the data to the heap, so the `StorageBackend` enum itself stays small. Without `Box`, the enum's size would equal its largest variant. With `Box`, the `R2` variant stores a pointer (8 bytes) instead of the full `Bucket` struct.

### Why enum dispatch, not trait objects?

Rust offers two approaches to polymorphism:

**Enum dispatch** (what we use here):
```rust
impl StorageBackend {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, String> {
        match self {
            StorageBackend::Local => { /* write to disk */ }
            StorageBackend::R2 { bucket, public_url } => { /* PUT to R2 */ }
        }
    }
}
```

**Trait objects** (the alternative):
```rust
#[async_trait]
trait StorageBackend: Send + Sync {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, String>;
}

struct LocalStorage;
struct R2Storage { bucket: s3::Bucket, public_url: String }

// Each struct implements the trait separately
```

Enum dispatch wins here for three reasons:

- **Closed set.** GrindIt will support Local and R2. If you add S3 later, you add a variant and the compiler tells you every `match` that needs updating. Trait objects are for open sets where third parties add implementations.
- **No vtable overhead.** Enum dispatch compiles to a simple branch. Trait objects use dynamic dispatch through a vtable pointer.
- **Simpler construction.** `StorageBackend::from_config()` returns a concrete type, not `Box<dyn StorageBackend>`.

The tradeoff: if you had ten storage backends, the `match` arms would become unwieldy. For two or three, enum dispatch is clearer.

### How Arc and the enum work together

In `main.rs`, the storage backend is constructed once and wrapped in `Arc`:

```rust
let storage = Arc::new(StorageBackend::from_config(&app_config.storage));
```

The `Arc<StorageBackend>` is then placed inside an `UploadState` struct that Axum passes to handlers:

```rust
#[derive(Clone)]
pub struct UploadState {
    pub storage: Arc<StorageBackend>,
    pub pool: sqlx::PgPool,
}
```

`UploadState` derives `Clone`. When Axum clones the state for a new request, it clones the `Arc` (cheap — increments a counter) and the `PgPool` (also cheap — it is internally `Arc`-based). No data is copied. A hundred concurrent uploads all share the same `StorageBackend` in memory.

---

## Building the Storage Backend

### The from_config constructor

`from_config` reads the `StorageSettings` struct and constructs the correct variant:

```rust
impl StorageBackend {
    pub fn from_config(config: &StorageSettings) -> Self {
        match config.backend.as_str() {
            "r2" => {
                let account_id = config.r2_account_id.as_ref()
                    .expect("R2 account_id required");
                let access_key = config.r2_access_key.as_ref()
                    .expect("R2 access_key required")
                    .expose_secret().clone();
                let secret_key = config.r2_secret_key.as_ref()
                    .expect("R2 secret_key required")
                    .expose_secret().clone();
                let bucket_name = config.r2_bucket.as_ref()
                    .expect("R2 bucket required");
                let public_url = config.r2_public_url.as_ref()
                    .expect("R2 public_url required").clone();

                let endpoint = format!(
                    "https://{}.r2.cloudflarestorage.com", account_id
                );
                let region = s3::Region::Custom {
                    region: "auto".to_string(),
                    endpoint,
                };
                let credentials = s3::creds::Credentials::new(
                    Some(&access_key), Some(&secret_key),
                    None, None, None,
                ).expect("Failed to create R2 credentials");

                let bucket = s3::Bucket::new(bucket_name, region, credentials)
                    .expect("Failed to create R2 bucket")
                    .with_path_style();

                StorageBackend::R2 { bucket, public_url }
            }
            _ => StorageBackend::Local,
        }
    }
}
```

Several patterns worth noting:

- **`expose_secret()`** — the `Secret<String>` wrapper from the `secrecy` crate prevents secrets from appearing in logs or debug output. You must explicitly call `expose_secret()` to access the inner value. This is defense-in-depth: even if you accidentally `println!("{:?}", config)`, the secrets stay hidden.
- **The `expect()` calls** — if the config says `backend: "r2"` but the R2 fields are missing, the server panics at startup. This is intentional. A misconfigured server should fail fast, not silently fall back to local storage in production.
- **`with_path_style()`** — Cloudflare R2 requires path-style URLs (`endpoint/bucket/key`) rather than virtual-hosted-style (`bucket.endpoint/key`). This is an S3 compatibility quirk.

### The upload method

```rust
pub async fn upload(
    &self,
    key: &str,
    data: &[u8],
    content_type: &str,
) -> Result<String, String> {
    match self {
        StorageBackend::Local => {
            let upload_dir = std::path::Path::new("public/videos");
            tokio::fs::create_dir_all(upload_dir).await
                .map_err(|e| format!("Failed to create upload dir: {}", e))?;
            let filepath = upload_dir.join(key);
            tokio::fs::write(&filepath, data).await
                .map_err(|e| format!("Failed to save file: {}", e))?;
            Ok(format!("/videos/{}", key))
        }
        StorageBackend::R2 { bucket, public_url } => {
            let path = format!("videos/{}", key);
            bucket.put_object_with_content_type(&path, data, content_type).await
                .map_err(|e| format!("R2 upload failed: {}", e))?;
            Ok(format!("{}/{}", public_url.trim_end_matches('/'), path))
        }
    }
}
```

The return value is a `String` URL. Local storage returns a relative path (`/videos/abc.mp4`) that the server serves as a static file. R2 returns an absolute URL (`https://cdn.grindit.app/videos/abc.mp4`) pointing to the public bucket.

The caller — the upload route — does not know or care which backend is active. It calls `state.storage.upload(key, data, content_type)` and gets back a URL. This is the strategy pattern: the algorithm (upload) varies by backend, but the interface is identical.

### Design Insight: Deep modules

John Ousterhout's *A Philosophy of Software Design* argues that the best modules have **simple interfaces and deep implementations**. `StorageBackend.upload()` has a four-parameter interface. Behind that interface, the Local variant creates directories and writes files; the R2 variant authenticates with AWS-compatible credentials, serializes the request, handles retries, and uploads over HTTPS. The caller sees none of this. The module is deep — it hides significant complexity behind a narrow surface.

---

## Magic Byte Validation

File extensions can lie. A user could rename `malware.exe` to `malware.mp4`. The upload route validates the actual file content by checking magic bytes — the first few bytes of a file that identify its format:

```rust
fn is_valid_video_magic(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }

    // MP4 / M4V / MOV — ftyp box at offset 4
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
```

Each video container format has a signature:

- **MP4/M4V/MOV**: The ISO Base Media File Format places an `ftyp` (file type) box starting at byte 4. The first 4 bytes are the box size. This covers `.mp4`, `.m4v`, and `.mov` because they all derive from the same container format.
- **WebM/MKV**: Both use the Matroska container, which starts with the EBML (Extensible Binary Meta Language) header `0x1A 0x45 0xDF 0xA3`.
- **AVI**: Microsoft's container uses the RIFF (Resource Interchange File Format) with `RIFF` at offset 0 and `AVI ` at offset 8.

The `*b"ftyp"` syntax creates a byte string literal and dereferences it to compare with the slice. The `[0x1A, 0x45, 0xDF, 0xA3]` array uses hex literals for bytes that are not printable ASCII.

### DSA connection: Strategy pattern — enum dispatch vs trait objects

The strategy pattern encapsulates an algorithm behind a common interface, allowing the algorithm to vary independently from the code that uses it. In the Gang of Four book, this requires an interface (or abstract class) and concrete implementations.

Rust offers two encodings:

| Approach | Dispatch | Extension | Overhead |
|---|---|---|---|
| Enum + `match` | Static (compiler resolves at compile time) | Closed — add a variant, compiler finds all call sites | Zero — compiles to a branch |
| `dyn Trait` | Dynamic (vtable lookup at runtime) | Open — anyone can implement the trait | One pointer indirection per call |

For GrindIt's two storage backends, enum dispatch is the right choice. For a plugin system where users register custom backends at runtime, trait objects would be necessary.

### System Design: File upload pipeline

A production file upload pipeline has four stages:

1. **Validation** — check auth, file size, extension, magic bytes. Reject early to avoid wasting resources.
2. **Storage** — write to disk or object storage. Generate a unique key (UUID) to prevent collisions and path traversal attacks.
3. **CDN** — in production, serve files through a CDN (Cloudflare R2's public bucket acts as both storage and CDN). The public URL is the CDN URL.
4. **Cleanup** — when an exercise is deleted, its video should be deleted too. GrindIt does not implement this yet — the video becomes orphaned. A production system would use a background job or database trigger.

The upload route implements the first three stages. The key generation uses `uuid::Uuid::new_v4()` — a random UUID that is virtually impossible to collide with or guess.

---

## The Axum Upload Route

### Route structure

```rust
pub async fn upload_video(
    State(state): State<UploadState>,
    session: Session,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
```

The function signature tells the story:

- **`State(state)`** — extracts the shared `UploadState` (which contains `Arc<StorageBackend>` and `PgPool`).
- **`session: Session`** — extracts the tower-sessions `Session` for auth checking.
- **`mut multipart: Multipart`** — Axum's multipart extractor. It is `mut` because reading fields consumes them (each field can only be read once).
- **Return type** — `Result<Json<UploadResponse>, (StatusCode, String)>`. On success, a JSON response with the video URL. On failure, an HTTP status code and error message.

### Auth guard

```rust
let user_id: Option<String> = session.get("user_id").await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

let user_id = user_id
    .ok_or((StatusCode::UNAUTHORIZED, "Sign in to upload videos".into()))?;

let user_uuid: uuid::Uuid = user_id.parse()
    .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid session".into()))?;

let _user = crate::db::get_user_by_id(&state.pool, user_uuid).await
    .map_err(|_| (StatusCode::UNAUTHORIZED, "User not found".into()))?;
```

This is the same guard pattern from Chapter 12, but adapted to Axum's error type. The upload route is a plain Axum handler (not a Leptos server function), so it returns `(StatusCode, String)` instead of `ServerFnError`. The four-step validation is: session exists, user_id exists, UUID parses, user still in database.

### Multipart field processing

```rust
while let Some(field) = multipart.next_field().await
    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))?
{
    let name = field.name().unwrap_or("").to_string();
    if name != "video" { continue; }

    let original_name = field.file_name().unwrap_or("video.mp4").to_string();
    let content_type = field.content_type()
        .unwrap_or("application/octet-stream").to_string();

    if !content_type.starts_with("video/") {
        return Err((StatusCode::BAD_REQUEST, "Only video files are allowed".into()));
    }
```

The `while let` loop iterates over multipart fields. It skips any field not named `"video"` — this allows the form to include other fields (like a CSRF token) without breaking the upload. The field name, file name, and content type are all `Option`s because a malformed request might omit them.

### Layered validation

The route validates in order of cheapest to most expensive:

1. **Content type** — a string comparison, almost free
2. **Extension** — another string comparison against an allowlist
3. **Size** — `data.len()` after reading the bytes
4. **Magic bytes** — inspects the first 12 bytes of the file content

```rust
let ext = std::path::Path::new(&original_name)
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or("")
    .to_lowercase();

if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
    return Err((StatusCode::BAD_REQUEST,
        format!("Unsupported file type '.{}'.", ext)));
}

let data = field.bytes().await
    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read file: {}", e)))?;

if data.len() > MAX_UPLOAD_BYTES {
    return Err((StatusCode::PAYLOAD_TOO_LARGE, "Video must be under 100 MB".into()));
}

if !is_valid_video_magic(&data) {
    return Err((StatusCode::BAD_REQUEST,
        "File content does not match a supported video format".into()));
}
```

The extension check uses `Path::extension()` — a standard library method that correctly handles edge cases like double extensions (`video.backup.mp4`) and files with no extension. The `and_then(|e| e.to_str())` chain handles the case where the extension is not valid UTF-8.

### Key generation and upload

```rust
let key = format!("{}.{}", uuid::Uuid::new_v4(), ext);

let url = state.storage.upload(&key, &data, &content_type).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

tracing::info!(
    user_id = %user_uuid,
    file = %original_name,
    size_bytes = data.len(),
    "video uploaded"
);

return Ok(Json(UploadResponse { url }));
```

The key is `{uuid}.{ext}` — for example, `a1b2c3d4-e5f6-7890-abcd-ef1234567890.mp4`. The UUID prevents filename collisions and path traversal attacks (a malicious filename like `../../../etc/passwd` would fail the extension check, and even if it passed, the UUID key replaces the original name entirely).

The `tracing::info!` macro logs the upload with structured fields — user, original filename, and byte size. This structured log is consumed by the Bunyan formatter (Chapter 15) and can be queried in production.

---

## The VideoUpload Component

### Client-side architecture

The `VideoUpload` component provides two modes: paste a URL (YouTube, Vimeo) or upload a file. It takes five signals as props — all owned by the parent form:

```rust
#[component]
pub fn VideoUpload(
    video_mode: RwSignal<String>,
    url_input: RwSignal<String>,
    video_preview: RwSignal<String>,
    video_url: RwSignal<String>,
    upload_error: RwSignal<String>,
) -> impl IntoView {
```

This is the "signals as props" pattern from Chapter 11. The parent (the exercise form) owns the state; the child renders the UI. When the user selects a file, `video_preview` updates to show the filename. When the upload completes, `video_url` updates with the server URL. The parent reads `video_url` when saving the exercise.

### The file upload bridge

The actual upload happens through a `wasm_bindgen` bridge function. The `upload_video_file` function in `voice.rs` uses `#[wasm_bindgen(inline_js)]` to define JavaScript that runs in the browser:

```rust
#[wasm_bindgen(inline_js = "
export async function upload_video_file(file_input_id) {
    const input = document.getElementById(file_input_id);
    if (!input || !input.files || !input.files[0]) { return ''; }
    const file = input.files[0];
    // ... client-side validation ...
    const form = new FormData();
    form.append('video', file);
    const resp = await fetch('/api/v1/upload/video', {
        method: 'POST', body: form
    });
    if (!resp.ok) { throw new Error(await resp.text()); }
    const data = await resp.json();
    return data.url;
}
")]
extern "C" {
    #[wasm_bindgen(catch)]
    pub async fn upload_video_file(file_input_id: &str) -> Result<JsValue, JsValue>;
}
```

Why use inline JS instead of pure Rust with `web_sys`? The `FormData` and `File` APIs require several layers of `web_sys` bindings, `JsCast` downcasts, and `JsFuture` conversions. The inline JS approach is 10 lines. This is a pragmatic choice — use Rust for business logic, use JS for browser API glue where the binding cost exceeds the safety benefit.

The `#[wasm_bindgen(catch)]` attribute converts JavaScript exceptions into Rust `Result::Err` — the thrown `Error` becomes a `JsValue` that the caller can inspect.

---

## Rust Gym

### Arc::new and Arc::clone

```rust
use std::sync::Arc;

// Create an Arc-wrapped value
let backend = Arc::new(StorageBackend::Local);

// Clone is cheap — just increments the reference count
let clone1 = Arc::clone(&backend);  // explicit syntax
let clone2 = backend.clone();       // method syntax (same thing)

// All three point to the same allocation
// When all three drop, the StorageBackend is freed
```

`Arc::clone(&backend)` is the idiomatic form — it signals to readers that this is a reference-count increment, not a deep copy. `backend.clone()` works identically but can be confused with a deep clone when reading code quickly.

### Enum dispatch with match

<details>
<summary>Exercise: add an S3 variant to StorageBackend</summary>

Add an `S3 { bucket: Box<s3::Bucket>, cdn_url: String }` variant and update the `upload` method. The compiler will tell you exactly where to add the new arm.

```rust
pub enum StorageBackend {
    Local,
    R2 { bucket: Box<s3::Bucket>, public_url: String },
    S3 { bucket: Box<s3::Bucket>, cdn_url: String },
}

impl StorageBackend {
    pub async fn upload(
        &self, key: &str, data: &[u8], content_type: &str,
    ) -> Result<String, String> {
        match self {
            StorageBackend::Local => { /* ... */ }
            StorageBackend::R2 { bucket, public_url } => { /* ... */ }
            StorageBackend::S3 { bucket, cdn_url } => {
                let path = format!("videos/{}", key);
                bucket.put_object_with_content_type(&path, data, content_type)
                    .await
                    .map_err(|e| format!("S3 upload failed: {}", e))?;
                Ok(format!("{}/{}", cdn_url.trim_end_matches('/'), path))
            }
        }
    }
}
```

Because `match` is exhaustive, adding a variant without updating all `match` expressions is a compile error. The compiler enforces that every code path handles the new backend.
</details>

### Async methods on enums

<details>
<summary>Exercise: add a delete method to StorageBackend</summary>

```rust
impl StorageBackend {
    pub async fn delete(&self, key: &str) -> Result<(), String> {
        match self {
            StorageBackend::Local => {
                let filepath = std::path::Path::new("public/videos").join(key);
                tokio::fs::remove_file(&filepath).await
                    .map_err(|e| format!("Failed to delete file: {}", e))
            }
            StorageBackend::R2 { bucket, .. } => {
                let path = format!("videos/{}", key);
                bucket.delete_object(&path).await
                    .map_err(|e| format!("R2 delete failed: {}", e))?;
                Ok(())
            }
        }
    }
}
```

The `..` in `R2 { bucket, .. }` ignores the `public_url` field — the delete operation does not need it. This is a destructuring pattern that tells the reader "I know there are more fields, and I intentionally do not use them."
</details>

---

## Exercises

### Exercise 1: Define StorageBackend enum with Local/R2 variants and `from_config()` constructor

Create `src/storage.rs` with the `StorageBackend` enum. The `Local` variant carries no data. The `R2` variant carries `bucket: Box<s3::Bucket>` and `public_url: String`. Implement `from_config(config: &StorageSettings) -> Self` that reads the backend field and constructs the correct variant.

<details>
<summary>Hints</summary>

- Match on `config.backend.as_str()` — the YAML field is a plain string
- For R2, use `config.r2_account_id.as_ref().expect(...)` to extract required fields
- Call `expose_secret()` on `Secret<String>` fields to get the inner value
- Construct the R2 endpoint as `https://{account_id}.r2.cloudflarestorage.com`
- Use `s3::Region::Custom` with `region: "auto"` for Cloudflare R2
- Default to `StorageBackend::Local` for any unrecognized backend string
</details>

<details>
<summary>Solution</summary>

```rust
use crate::configuration::StorageSettings;
use secrecy::ExposeSecret;

pub enum StorageBackend {
    Local,
    R2 {
        bucket: Box<s3::Bucket>,
        public_url: String,
    },
}

impl StorageBackend {
    pub fn from_config(config: &StorageSettings) -> Self {
        match config.backend.as_str() {
            "r2" => {
                let account_id = config.r2_account_id.as_ref()
                    .expect("R2 account_id required");
                let access_key = config.r2_access_key.as_ref()
                    .expect("R2 access_key required")
                    .expose_secret().clone();
                let secret_key = config.r2_secret_key.as_ref()
                    .expect("R2 secret_key required")
                    .expose_secret().clone();
                let bucket_name = config.r2_bucket.as_ref()
                    .expect("R2 bucket required");
                let public_url = config.r2_public_url.as_ref()
                    .expect("R2 public_url required").clone();

                let endpoint = format!(
                    "https://{}.r2.cloudflarestorage.com", account_id
                );
                let region = s3::Region::Custom {
                    region: "auto".to_string(),
                    endpoint,
                };
                let credentials = s3::creds::Credentials::new(
                    Some(&access_key), Some(&secret_key),
                    None, None, None,
                ).expect("Failed to create R2 credentials");

                let bucket = s3::Bucket::new(bucket_name, region, credentials)
                    .expect("Failed to create R2 bucket")
                    .with_path_style();

                StorageBackend::R2 { bucket, public_url }
            }
            _ => StorageBackend::Local,
        }
    }
}
```

The `expect()` calls are intentional — if the config specifies R2 but omits required fields, the server should fail at startup, not silently fall back to local storage.
</details>

### Exercise 2: Implement `async fn upload(&self, key, data, content_type)` with match dispatch

Add the `upload` method to `StorageBackend`. For `Local`, create the `public/videos/` directory (if missing), write the file, and return a relative URL. For `R2`, upload to the bucket and return the public CDN URL.

<details>
<summary>Hints</summary>

- Use `tokio::fs::create_dir_all` for async directory creation
- Use `tokio::fs::write` for async file writing
- The local URL format is `/videos/{key}`
- The R2 URL format is `{public_url}/videos/{key}`
- Use `trim_end_matches('/')` on the public URL to avoid double slashes
- Both paths return `Result<String, String>` — map errors with `format!`
</details>

<details>
<summary>Solution</summary>

```rust
impl StorageBackend {
    pub async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<String, String> {
        match self {
            StorageBackend::Local => {
                let upload_dir = std::path::Path::new("public/videos");
                tokio::fs::create_dir_all(upload_dir).await
                    .map_err(|e| format!("Failed to create upload dir: {}", e))?;
                let filepath = upload_dir.join(key);
                tokio::fs::write(&filepath, data).await
                    .map_err(|e| format!("Failed to save file: {}", e))?;
                Ok(format!("/videos/{}", key))
            }
            StorageBackend::R2 { bucket, public_url } => {
                let path = format!("videos/{}", key);
                bucket.put_object_with_content_type(&path, data, content_type).await
                    .map_err(|e| format!("R2 upload failed: {}", e))?;
                Ok(format!("{}/{}", public_url.trim_end_matches('/'), path))
            }
        }
    }
}
```

The method takes `&self` — it borrows the `StorageBackend` immutably. This is safe because `upload` does not modify the backend configuration. The `async` keyword requires the caller to `.await` the result, which is necessary because both local I/O and network I/O are asynchronous in Tokio.
</details>

### Exercise 3: Build the Axum multipart upload route with magic byte validation and size limits

Create `src/routes/upload.rs` with the `upload_video` handler. It should: extract the session and verify the user, iterate multipart fields looking for "video", validate content type, extension, size (100 MB max), and magic bytes, generate a UUID key, call `state.storage.upload()`, and return the URL as JSON.

<details>
<summary>Hints</summary>

- The handler signature: `async fn upload_video(State(state): State<UploadState>, session: Session, mut multipart: Multipart) -> Result<Json<UploadResponse>, (StatusCode, String)>`
- Use `while let Some(field) = multipart.next_field().await` to iterate fields
- Skip fields where `field.name() != Some("video")`
- Validate the extension against `["mp4", "webm", "mov", "avi", "m4v"]`
- Read bytes with `field.bytes().await`, then check `data.len() > MAX_UPLOAD_BYTES`
- Check magic bytes with `is_valid_video_magic(&data)` after reading
- Generate key: `format!("{}.{}", uuid::Uuid::new_v4(), ext)`
</details>

<details>
<summary>Solution</summary>

```rust
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
pub struct UploadResponse { pub url: String }

const ALLOWED_EXTENSIONS: &[&str] = &["mp4", "webm", "mov", "avi", "m4v"];
const MAX_UPLOAD_BYTES: usize = 100 * 1024 * 1024;

fn is_valid_video_magic(data: &[u8]) -> bool {
    if data.len() < 12 { return false; }
    if data[4..8] == *b"ftyp" { return true; }
    if data[0..4] == [0x1A, 0x45, 0xDF, 0xA3] { return true; }
    if data[0..4] == *b"RIFF" && data[8..12] == *b"AVI " { return true; }
    false
}

pub async fn upload_video(
    State(state): State<UploadState>,
    session: Session,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    // Auth guard
    let user_id: Option<String> = session.get("user_id").await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let user_id = user_id
        .ok_or((StatusCode::UNAUTHORIZED, "Sign in to upload videos".into()))?;
    let user_uuid: uuid::Uuid = user_id.parse()
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid session".into()))?;
    let _user = crate::db::get_user_by_id(&state.pool, user_uuid).await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "User not found".into()))?;

    // Process upload
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))?
    {
        if field.name().unwrap_or("") != "video" { continue; }
        let original_name = field.file_name().unwrap_or("video.mp4").to_string();
        let content_type = field.content_type()
            .unwrap_or("application/octet-stream").to_string();

        if !content_type.starts_with("video/") {
            return Err((StatusCode::BAD_REQUEST, "Only video files are allowed".into()));
        }

        let ext = std::path::Path::new(&original_name)
            .extension().and_then(|e| e.to_str())
            .unwrap_or("").to_lowercase();
        if !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            return Err((StatusCode::BAD_REQUEST,
                format!("Unsupported file type '.{}'", ext)));
        }

        let data = field.bytes().await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Read failed: {}", e)))?;
        if data.len() > MAX_UPLOAD_BYTES {
            return Err((StatusCode::PAYLOAD_TOO_LARGE, "Video must be under 100 MB".into()));
        }
        if !is_valid_video_magic(&data) {
            return Err((StatusCode::BAD_REQUEST, "Not a supported video format".into()));
        }

        let key = format!("{}.{}", uuid::Uuid::new_v4(), ext);
        let url = state.storage.upload(&key, &data, &content_type).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

        tracing::info!(user_id = %user_uuid, file = %original_name,
            size_bytes = data.len(), "video uploaded");
        return Ok(Json(UploadResponse { url }));
    }

    Err((StatusCode::BAD_REQUEST, "No video field found".into()))
}
```

The validation order (content type, extension, size, magic bytes) puts cheap checks first. If the content type is wrong, we never read the bytes. If the extension is wrong, we never check the size. This is the fail-fast principle applied to resource consumption.
</details>

### Exercise 4: Build the VideoUpload Leptos component with WASM file selection

Create `src/components/video_upload.rs` with a `VideoUpload` component that provides two modes: URL paste and file upload. The component takes five `RwSignal` props controlled by the parent form. File upload mode shows a file picker with client-side validation and a clear button.

<details>
<summary>Hints</summary>

- Use `class:active=move || video_mode.get() == "url"` for the mode toggle buttons
- When switching modes, clear the other mode's state (URL input or file preview)
- The file input uses `accept=".mp4,.webm,.mov,.avi,.m4v"` to filter the OS file picker
- On file selection, extract the filename with `rsplit(['/', '\\']).next()` and set `video_preview`
- Show the clear button only when `video_preview` is non-empty
- Display `upload_error` below the file section when non-empty
</details>

<details>
<summary>Solution</summary>

```rust
use leptos::prelude::*;

#[component]
pub fn VideoUpload(
    video_mode: RwSignal<String>,
    url_input: RwSignal<String>,
    video_preview: RwSignal<String>,
    video_url: RwSignal<String>,
    upload_error: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="video-upload">
            <div class="video-mode-toggle">
                <button type="button" class="video-mode-btn"
                    class:active=move || video_mode.get() == "url"
                    on:click=move |_| {
                        video_mode.set("url".to_string());
                        video_preview.set(String::new());
                        video_url.set(String::new());
                        upload_error.set(String::new());
                    }
                >" URL"</button>
                <button type="button" class="video-mode-btn"
                    class:active=move || video_mode.get() == "file"
                    on:click=move |_| {
                        video_mode.set("file".to_string());
                        url_input.set(String::new());
                        upload_error.set(String::new());
                    }
                >" File"</button>
            </div>

            {move || (video_mode.get() == "url").then(|| view! {
                <input type="text" class="video-url-input"
                    placeholder="Paste YouTube or Vimeo URL"
                    prop:value=move || url_input.get()
                    on:input=move |ev| url_input.set(event_target_value(&ev))
                />
            })}

            {move || (video_mode.get() == "file").then(|| view! {
                <div class="video-file-section">
                    <label class="video-upload-label" for="exercise-video-input">
                        " Choose File"
                    </label>
                    <input type="file" id="exercise-video-input"
                        accept=".mp4,.webm,.mov,.avi,.m4v"
                        class="video-file-input"
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            if !val.is_empty() {
                                let name = val.rsplit(['/', '\\'])
                                    .next().unwrap_or(&val).to_string();
                                video_preview.set(name);
                                video_url.set(String::new());
                            }
                        }
                    />
                    {move || {
                        let preview = video_preview.get();
                        (!preview.is_empty()).then(|| view! {
                            <div class="video-selected">
                                <span class="video-filename">{preview}</span>
                                <button type="button" class="video-clear"
                                    on:click=move |_| {
                                        video_preview.set(String::new());
                                        video_url.set(String::new());
                                    }
                                >"x"</button>
                            </div>
                        })
                    }}
                </div>
            })}

            {move || {
                let err = upload_error.get();
                (!err.is_empty()).then(|| view! {
                    <div class="video-error">{err}</div>
                })
            }}
        </div>
    }
}
```

The component does not perform the actual upload — that happens in the parent form's save handler, which calls the `upload_video_file` JS bridge. This separation keeps the component focused on file selection and preview, while the parent coordinates the upload timing (only on form submit, not on file selection).
</details>

---

## Summary

This chapter introduced two core Rust patterns through the video upload pipeline:

- **`Arc<T>`** — shared ownership with atomic reference counting. Cheap to clone, safe across threads, freed when the last reference drops. Used to share `StorageBackend` across all Axum handlers.
- **Enum-based abstraction** — the strategy pattern via `enum` + `match`. `StorageBackend::upload()` hides the difference between writing to local disk and uploading to Cloudflare R2. Adding a new backend means adding a variant — the compiler finds every `match` that needs updating.
- **Magic byte validation** — defense-in-depth beyond file extensions. Check the actual bytes to confirm the file is what it claims to be.
- **Multipart upload** — Axum's `Multipart` extractor provides async field iteration with content type and file name metadata.
- **WASM-JS bridge** — `#[wasm_bindgen(inline_js)]` for browser APIs where pure Rust bindings would be verbose.

The next chapter builds on the WASM bridge to implement PWA features: service worker registration, theme toggle with localStorage, and an install banner with iOS detection.

---

### DS Deep Dive

Your `Arc<StorageBackend>` shares one value across five handlers. But what IS Arc? How does atomic reference counting work? This deep dive builds Rc and Arc from scratch — raw pointers, atomic operations, and the Drop that frees memory.

**→ [Arc & Smart Pointers — "The Equipment Checkout System"](../ds-narratives/ch13-arc-smart-pointers.md)**
