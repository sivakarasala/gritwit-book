# Chapter 13: Video Uploads

Every exercise in the library should be demonstrable. A text description of a Turkish Get-Up helps, but a fifteen-second video removes all ambiguity. This chapter builds the video upload pipeline: file selection on the client, multipart upload to the server, magic byte validation, and storage in either a local directory or Cloudflare R2. The same `upload()` method works for both --- the caller never knows which backend is active.

The spotlight concept is **smart pointers and enum-based abstraction** --- specifically `Arc<StorageBackend>` for sharing a storage backend across async handlers, and the pattern of using an enum with `match` dispatch as a strategy pattern. You will see how `Arc` enables shared ownership without runtime garbage collection, why `Box` appears inside enum variants for large types, and how enum dispatch compares to trait objects for polymorphism.

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

An Axum server spawns a new task for every incoming request. Each task runs on a different thread. If two requests arrive simultaneously and both need the storage backend, they both need access to the same `StorageBackend` value --- but Rust's ownership rules say a value can have only one owner.

Think about it this way: you have a single printer in an office. Twenty people need to use it. You cannot give each person their own copy of the printer --- that is wasteful. You also cannot hand the printer to one person and leave everyone else waiting. What you need is a way for everyone to share the same printer, with some coordination so they do not print on top of each other.

Rust solves this with `Arc` --- Atomic Reference Counting.

> **Programming Concept: What is a Smart Pointer?**
>
> A pointer is a variable that holds the memory address of another value --- it "points to" where the data lives. A **smart pointer** is a pointer that also manages the lifecycle of the data it points to. Think of it like a library lending system:
>
> - A **raw pointer** is like writing down a book's shelf location on a sticky note. The note tells you where the book is, but it does not track whether the book has been moved or returned.
> - A **smart pointer** is like a library card system. The system knows who has the book, tracks when it is returned, and prevents conflicts (like two people checking out the same book simultaneously).
>
> Rust has several smart pointer types:
>
> - **`Box<T>`** --- puts data on the heap (like moving a large item to a storage unit instead of keeping it on your desk). You are the sole owner.
> - **`Rc<T>`** --- Reference Counted. Multiple owners of the same data, but only in single-threaded code.
> - **`Arc<T>`** --- Atomic Reference Counted. Multiple owners of the same data, safe across threads. The "atomic" means the counting mechanism works correctly even when multiple threads access it simultaneously.
>
> When you clone an `Arc`, no data is copied. The smart pointer just increments a counter: "one more person is using this." When an `Arc` is dropped, the counter decrements. When the counter reaches zero --- meaning nobody is using it anymore --- the data is freed.

### Arc: shared ownership with reference counting

`Arc<T>` wraps a value of type `T` and maintains a counter of how many `Arc` pointers reference that value. When you clone an `Arc`, the counter increments. When an `Arc` drops, the counter decrements. When the counter reaches zero, the value is freed.

```rust
use std::sync::Arc;

let storage = Arc::new(StorageBackend::from_config(&config.storage));

// Clone is cheap --- it increments a counter, not the data
let storage_for_handler = storage.clone();
let storage_for_another = storage.clone();
// All three point to the same StorageBackend in memory
```

The "Atomic" in `Arc` means the counter uses atomic CPU instructions --- safe for concurrent access across threads. This is what makes `Arc` different from `Rc` (Reference Counted), which is single-threaded only. In an async web server, you always want `Arc`.

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

1. **Asymmetric variants.** `Local` carries no data --- it writes to a fixed `public/videos/` directory. `R2` carries a configured S3 bucket and a public URL prefix. This is perfectly legal in Rust enums --- each variant can hold different types and amounts of data.

2. **`Box<s3::Bucket>`** --- the `Bucket` struct is large (it contains credentials, endpoint URLs, region info). Wrapping it in `Box` moves the data to the heap, so the `StorageBackend` enum itself stays small. Without `Box`, the enum's size would equal its largest variant. With `Box`, the `R2` variant stores a pointer (8 bytes) instead of the full `Bucket` struct.

> **Programming Concept: What is an Enum with Data?**
>
> You have used enums since Chapter 7 --- `UserRole::Athlete`, `UserRole::Coach`. Those variants were plain labels, like colors in a traffic light. Rust enums can also carry data inside each variant, making them much more powerful.
>
> Think of a shipping package:
>
> - A **letter** needs no extra packaging --- it is just the envelope.
> - A **parcel** needs a box with dimensions (width, height, depth) and a weight.
> - A **pallet** needs a forklift flag, a weight, and a stack limit.
>
> In Rust:
>
> ```rust
> enum Shipment {
>     Letter,                                         // no data
>     Parcel { width: f32, height: f32, weight: f32 },  // struct-like variant
>     Pallet { weight: f32, stack_limit: u8 },          // different fields
> }
> ```
>
> Each variant carries exactly the data it needs --- no more, no less. This is fundamentally different from a struct, where every instance has every field. A `Letter` does not waste memory on width or height fields it does not need.

### Why enum dispatch, not trait objects?

Rust offers two approaches to polymorphism (using different types through a common interface):

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
- **No vtable overhead.** Enum dispatch compiles to a simple branch. Trait objects use dynamic dispatch through a vtable pointer (an extra layer of indirection at runtime).
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

`UploadState` derives `Clone`. When Axum clones the state for a new request, it clones the `Arc` (cheap --- increments a counter) and the `PgPool` (also cheap --- it is internally `Arc`-based). No data is copied. A hundred concurrent uploads all share the same `StorageBackend` in memory.

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

Several patterns worth understanding:

- **`expose_secret()`** --- the `Secret<String>` wrapper from the `secrecy` crate prevents secrets from appearing in logs or debug output. You must explicitly call `expose_secret()` to access the inner value. This is defense-in-depth: even if you accidentally `println!("{:?}", config)`, the secrets stay hidden.
- **The `expect()` calls** --- if the config says `backend: "r2"` but the R2 fields are missing, the server panics at startup. This is intentional. A misconfigured server should fail fast, not silently fall back to local storage in production.
- **`with_path_style()`** --- Cloudflare R2 requires path-style URLs (`endpoint/bucket/key`) rather than virtual-hosted-style (`bucket.endpoint/key`). This is an S3 compatibility quirk.

> **Programming Concept: What is a File Upload?**
>
> When you click "Choose File" in a web browser and select a video, the browser reads the file's raw bytes from your computer. To send those bytes to the server, the browser uses a format called **multipart form data** --- it packages the file bytes along with metadata (filename, content type) into a structured message.
>
> Think of it like mailing a package:
>
> - The **file bytes** are the contents of the package
> - The **filename** is the label on the outside
> - The **content type** (like `video/mp4`) is a declaration of what is inside
> - The **multipart boundary** is the tape separating different items in the same shipment
>
> The server receives this package, validates the contents (is it actually a video? is it too large?), and stores it somewhere --- either on the local disk or in cloud storage.

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

The caller --- the upload route --- does not know or care which backend is active. It calls `state.storage.upload(key, data, content_type)` and gets back a URL. This is the strategy pattern: the algorithm (upload) varies by backend, but the interface is identical.

### Design Insight: Deep modules

John Ousterhout's *A Philosophy of Software Design* argues that the best modules have **simple interfaces and deep implementations**. `StorageBackend.upload()` has a four-parameter interface. Behind that interface, the Local variant creates directories and writes files; the R2 variant authenticates with AWS-compatible credentials, serializes the request, handles retries, and uploads over HTTPS. The caller sees none of this. The module is deep --- it hides significant complexity behind a narrow surface.

A shallow alternative would expose `upload_to_local(path, data)` and `upload_to_r2(bucket, credentials, path, data)` separately, forcing every caller to know which backend it is talking to. The deep module hides that decision.

---

## Magic Byte Validation

File extensions can lie. A user could rename `malware.exe` to `malware.mp4`. The upload route validates the actual file content by checking magic bytes --- the first few bytes of a file that identify its format.

> **Programming Concept: What are Magic Bytes?**
>
> Every file format starts with a specific sequence of bytes that acts like a fingerprint. These are called **magic bytes** (or a **file signature**). When a program wants to identify what kind of file it is looking at, it reads the first few bytes and compares them against known signatures.
>
> Think of it like the first few notes of a song. If you hear "da-da-da-DUM," you know it is Beethoven's Fifth Symphony. Similarly:
>
> - If the bytes at position 4-7 spell "ftyp," the file is an MP4 video
> - If the first four bytes are `1A 45 DF A3` (in hexadecimal), the file is a WebM video
> - If the first four bytes spell "RIFF" and bytes 8-11 spell "AVI ", the file is an AVI video
>
> This is more reliable than checking the file extension, because anyone can rename a file. The magic bytes are part of the file content itself --- they cannot be changed without breaking the file format.

```rust
fn is_valid_video_magic(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }

    // MP4 / M4V / MOV --- ftyp box at offset 4
    if data[4..8] == *b"ftyp" {
        return true;
    }

    // WebM / MKV --- EBML header
    if data[0..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return true;
    }

    // AVI --- RIFF....AVI
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

### DSA connection: Strategy pattern --- enum dispatch vs trait objects

The strategy pattern encapsulates an algorithm behind a common interface, allowing the algorithm to vary independently from the code that uses it. In the Gang of Four book, this requires an interface (or abstract class) and concrete implementations.

Rust offers two encodings:

| Approach | Dispatch | Extension | Overhead |
|---|---|---|---|
| Enum + `match` | Static (compiler resolves at compile time) | Closed --- add a variant, compiler finds all call sites | Zero --- compiles to a branch |
| `dyn Trait` | Dynamic (vtable lookup at runtime) | Open --- anyone can implement the trait | One pointer indirection per call |

For GrindIt's two storage backends, enum dispatch is the right choice. For a plugin system where users register custom backends at runtime, trait objects would be necessary.

### System Design: File upload pipeline

A production file upload pipeline has four stages:

1. **Validation** --- check auth, file size, extension, magic bytes. Reject early to avoid wasting resources.
2. **Storage** --- write to disk or object storage. Generate a unique key (UUID) to prevent collisions and path traversal attacks.
3. **CDN** --- in production, serve files through a CDN (Cloudflare R2's public bucket acts as both storage and CDN). The public URL is the CDN URL.
4. **Cleanup** --- when an exercise is deleted, its video should be deleted too. GrindIt does not implement this yet --- the video becomes orphaned. A production system would use a background job or database trigger.

> **Programming Concept: What is a CDN?**
>
> A CDN (Content Delivery Network) is a network of servers spread across the world. When a user in Tokyo requests a video, the CDN serves it from a server in Tokyo --- not from your origin server in Virginia. This reduces latency dramatically.
>
> Think of it like a library system. Instead of one central library where everyone must travel, there are branch libraries in every neighborhood. Each branch keeps copies of popular books. When you request a book, you get it from the nearest branch.
>
> Cloudflare R2 acts as both storage (where the video lives permanently) and CDN (where it is served from the nearest edge location). The `public_url` in our `StorageBackend::R2` variant is the CDN URL.

The upload route implements the first three stages. The key generation uses `uuid::Uuid::new_v4()` --- a random UUID that is virtually impossible to collide with or guess.

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

- **`State(state)`** --- extracts the shared `UploadState` (which contains `Arc<StorageBackend>` and `PgPool`).
- **`session: Session`** --- extracts the tower-sessions `Session` for auth checking.
- **`mut multipart: Multipart`** --- Axum's multipart extractor. It is `mut` because reading fields consumes them (each field can only be read once). Think of opening a sealed envelope --- once you tear it open, it is consumed.
- **Return type** --- `Result<Json<UploadResponse>, (StatusCode, String)>`. On success, a JSON response with the video URL. On failure, an HTTP status code and error message.

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

This is the same guard pattern from Chapter 12, but adapted to Axum's error type. The upload route is a plain Axum handler (not a Leptos server function), so it returns `(StatusCode, String)` instead of `ServerFnError`. The four-step validation ensures: session exists, user_id exists, UUID parses correctly, and the user still exists in the database.

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

The `while let` loop iterates over multipart fields. It skips any field not named `"video"` --- this allows the form to include other fields (like a CSRF token) without breaking the upload. The field name, file name, and content type are all `Option`s because a malformed request might omit them.

### Layered validation

The route validates in order of cheapest to most expensive:

1. **Content type** --- a string comparison, almost free
2. **Extension** --- another string comparison against an allowlist
3. **Size** --- `data.len()` after reading the bytes
4. **Magic bytes** --- inspects the first 12 bytes of the file content

> **Programming Concept: Why Validate in This Order?**
>
> Imagine you are a bouncer at a club. You check in this order:
>
> 1. **Do they have an invitation?** (quick glance --- reject immediately if not)
> 2. **Is their name on the guest list?** (check a list --- slightly more work)
> 3. **Do they pass the metal detector?** (requires walking through --- takes time)
> 4. **Does their bag pass inspection?** (open and search --- most expensive)
>
> You do not inspect bags before checking invitations. Each step is more expensive than the last, so you want to reject bad requests as early and cheaply as possible. This is the **fail-fast** principle applied to resource consumption.

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

The extension check uses `Path::extension()` --- a standard library method that correctly handles edge cases like double extensions (`video.backup.mp4`) and files with no extension. The `and_then(|e| e.to_str())` chain handles the case where the extension is not valid UTF-8.

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

The key is `{uuid}.{ext}` --- for example, `a1b2c3d4-e5f6-7890-abcd-ef1234567890.mp4`. The UUID prevents filename collisions and path traversal attacks (a malicious filename like `../../../etc/passwd` would fail the extension check, and even if it passed, the UUID key replaces the original name entirely).

---

## The VideoUpload Component

### Client-side architecture

The `VideoUpload` component provides two modes: paste a URL (YouTube, Vimeo) or upload a file. It takes five signals as props --- all owned by the parent form:

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

The actual upload happens through a `wasm_bindgen` bridge function. The `upload_video_file` function uses `#[wasm_bindgen(inline_js)]` to define JavaScript that runs in the browser:

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

Why use inline JS instead of pure Rust with `web_sys`? The `FormData` and `File` APIs require several layers of `web_sys` bindings, `JsCast` downcasts, and `JsFuture` conversions. The inline JS approach is 10 lines. This is a pragmatic choice --- use Rust for business logic, use JS for browser API glue where the binding cost exceeds the safety benefit.

The `#[wasm_bindgen(catch)]` attribute converts JavaScript exceptions into Rust `Result::Err` --- the thrown `Error` becomes a `JsValue` that the caller can inspect.

---

## Rust Gym

These drills focus on the spotlight concepts: `Arc`, enum dispatch, and async methods on enums. They are simpler than the exercises above --- the goal is to build muscle memory with the core patterns.

### Drill 1: Create and clone an Arc

```rust
use std::sync::Arc;

// Create an Arc-wrapped value
let backend = Arc::new(StorageBackend::Local);

// Clone is cheap --- just increments the reference count
let clone1 = Arc::clone(&backend);  // explicit syntax (preferred)
let clone2 = backend.clone();       // method syntax (same thing)

// All three point to the same allocation
// When all three drop, the StorageBackend is freed
```

**Try it yourself:** Create an `Arc<String>` containing `"hello"`, clone it three times, and print each clone. Verify they all print the same string. Then use `Arc::strong_count(&original)` to print how many references exist.

<details>
<summary>Solution</summary>

```rust
use std::sync::Arc;

fn main() {
    let original = Arc::new("hello".to_string());
    let clone1 = Arc::clone(&original);
    let clone2 = Arc::clone(&original);
    let clone3 = Arc::clone(&original);

    println!("{}", original);   // hello
    println!("{}", clone1);     // hello
    println!("{}", clone2);     // hello
    println!("{}", clone3);     // hello
    println!("Reference count: {}", Arc::strong_count(&original)); // 4
}
```
</details>

### Drill 2: Enum dispatch with match

<details>
<summary>Exercise: add an S3 variant to a simplified StorageBackend</summary>

Given this simplified enum, add an `S3 { cdn_url: String }` variant and update the `url_prefix` method. The compiler will tell you exactly where to add the new arm.

```rust
enum Storage {
    Local,
    R2 { public_url: String },
    // Add S3 here
}

impl Storage {
    fn url_prefix(&self) -> &str {
        match self {
            Storage::Local => "/videos",
            Storage::R2 { public_url } => public_url,
            // Add S3 arm here
        }
    }
}
```

```rust
// Solution:
enum Storage {
    Local,
    R2 { public_url: String },
    S3 { cdn_url: String },
}

impl Storage {
    fn url_prefix(&self) -> &str {
        match self {
            Storage::Local => "/videos",
            Storage::R2 { public_url } => public_url,
            Storage::S3 { cdn_url } => cdn_url,
        }
    }
}
```

Because `match` is exhaustive, adding a variant without updating all `match` expressions is a compile error. The compiler enforces that every code path handles the new backend.
</details>

### Drill 3: Box for large enum variants

<details>
<summary>Exercise: compare enum sizes with and without Box</summary>

```rust
use std::mem::size_of;

struct SmallConfig { name: String }        // ~24 bytes
struct LargeConfig { data: [u8; 1024] }    // 1024 bytes

enum WithoutBox {
    Small(SmallConfig),
    Large(LargeConfig),
}

enum WithBox {
    Small(SmallConfig),
    Large(Box<LargeConfig>),
}

fn main() {
    println!("Without Box: {} bytes", size_of::<WithoutBox>());
    // Prints: ~1032 (the size of the largest variant + discriminant)
    println!("With Box: {} bytes", size_of::<WithBox>());
    // Prints: ~32 (SmallConfig size + discriminant, since Box is 8 bytes)
}
```

The lesson: `Box` keeps the enum small by moving large data to the heap. This matters when you store many enum values or pass them across function boundaries.
</details>

---

## Exercises

### Exercise 1: Define StorageBackend enum with Local/R2 variants and `from_config()` constructor

**Goal:** Create `src/storage.rs` with the `StorageBackend` enum.

**Instructions:**

1. Define the `StorageBackend` enum with two variants: `Local` (no data) and `R2` (with `bucket: Box<s3::Bucket>` and `public_url: String`)
2. Implement a `from_config(config: &StorageSettings) -> Self` method
3. In `from_config`, match on `config.backend.as_str()`:
   - If `"r2"`, extract all required fields (account_id, access_key, secret_key, bucket, public_url), build the S3 region and credentials, create the bucket, and return `StorageBackend::R2`
   - For any other value, return `StorageBackend::Local`
4. Use `expect()` for required R2 fields --- the server should panic at startup if misconfigured

<details>
<summary>Hint 1: Extracting secret values</summary>

The R2 credentials are wrapped in `Option<Secret<String>>`. To get the inner string:

```rust
let access_key = config.r2_access_key.as_ref()    // Option<&Secret<String>>
    .expect("R2 access_key required")               // &Secret<String>
    .expose_secret()                                 // &String
    .clone();                                        // String
```

The chain: unwrap the Option, expose the secret, clone to own the value.
</details>

<details>
<summary>Hint 2: Building the R2 endpoint</summary>

Cloudflare R2 uses a custom S3-compatible endpoint:

```rust
let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
let region = s3::Region::Custom {
    region: "auto".to_string(),
    endpoint,
};
```

The `"auto"` region tells the S3 client not to validate the region name --- Cloudflare uses its own naming.
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

The `expect()` calls are intentional --- if the config specifies R2 but omits required fields, the server should fail at startup, not silently fall back to local storage.
</details>

### Exercise 2: Implement `async fn upload(&self, key, data, content_type)` with match dispatch

**Goal:** Add the `upload` method to `StorageBackend` so both variants can store files through the same interface.

**Instructions:**

1. Add an `async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, String>` method
2. Use `match self` to dispatch to the correct implementation
3. For `Local`: create the `public/videos/` directory with `tokio::fs::create_dir_all`, write the file with `tokio::fs::write`, and return `Ok(format!("/videos/{}", key))`
4. For `R2`: upload with `bucket.put_object_with_content_type`, and return the public URL
5. Map all errors with `format!` to produce helpful error messages

<details>
<summary>Hint: Why tokio::fs instead of std::fs?</summary>

Standard library file operations (`std::fs::write`) block the current thread while the disk I/O completes. In an async web server, blocking a thread means no other requests can be processed on that thread. `tokio::fs::write` runs the I/O on a background thread and returns a Future, so the server thread can handle other requests while waiting for the disk.
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

The method takes `&self` --- it borrows the `StorageBackend` immutably. This is safe because `upload` does not modify the backend configuration. The `async` keyword requires the caller to `.await` the result, which is necessary because both local I/O and network I/O are asynchronous in Tokio.
</details>

### Exercise 3: Build the Axum multipart upload route with magic byte validation and size limits

**Goal:** Create `src/routes/upload.rs` with a complete upload handler that validates and stores video files.

**Instructions:**

1. Define `UploadState` (with `Arc<StorageBackend>` and `PgPool`) and `UploadResponse` (with `url: String`)
2. Define constants: `ALLOWED_EXTENSIONS = ["mp4", "webm", "mov", "avi", "m4v"]` and `MAX_UPLOAD_BYTES = 100 * 1024 * 1024`
3. Write the `is_valid_video_magic` function that checks for MP4 (ftyp at offset 4), WebM (EBML header), and AVI (RIFF...AVI) signatures
4. Write the `upload_video` handler that:
   - Extracts the session and verifies the user (4-step auth check)
   - Iterates multipart fields with `while let Some(field) = ...`
   - Skips non-"video" fields
   - Validates content type, extension, size, and magic bytes (in that order, cheapest first)
   - Generates a UUID key and calls `state.storage.upload()`
   - Returns the URL as JSON

<details>
<summary>Hint 1: The validation order matters</summary>

Check in this order: content type (free string comparison) -> extension (string lookup) -> size (read bytes, then check length) -> magic bytes (inspect first 12 bytes). If any check fails, return an error immediately --- do not proceed to more expensive checks.
</details>

<details>
<summary>Hint 2: Extracting the file extension</summary>

```rust
let ext = std::path::Path::new(&original_name)
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or("")
    .to_lowercase();
```

This chain handles edge cases: files with no extension, files with non-UTF-8 names, and case-insensitive comparison.
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

**Goal:** Create `src/components/video_upload.rs` with a component that lets users paste a URL or select a file.

**Instructions:**

1. The component takes five `RwSignal` props: `video_mode`, `url_input`, `video_preview`, `video_url`, and `upload_error`
2. Create two mode toggle buttons ("URL" and "File") that switch `video_mode` between `"url"` and `"file"`
3. When switching modes, clear the other mode's state
4. In URL mode, show a text input bound to `url_input`
5. In file mode, show a file picker with `accept=".mp4,.webm,.mov,.avi,.m4v"`
6. When a file is selected, extract the filename and set `video_preview`
7. Show a clear button when `video_preview` is non-empty
8. Display `upload_error` below the file section when non-empty

<details>
<summary>Hint 1: The mode toggle pattern</summary>

```rust
<button type="button" class="video-mode-btn"
    class:active=move || video_mode.get() == "url"
    on:click=move |_| {
        video_mode.set("url".to_string());
        // Clear file mode state
        video_preview.set(String::new());
        video_url.set(String::new());
        upload_error.set(String::new());
    }
>" URL"</button>
```

The `class:active` directive adds the "active" CSS class when the condition is true. This is how the selected tab gets highlighted.
</details>

<details>
<summary>Hint 2: Extracting the filename</summary>

The file input's value includes a fake path (browsers hide the real path for security). Extract just the filename:

```rust
let name = val.rsplit(['/', '\\']).next().unwrap_or(&val).to_string();
```

`rsplit` splits from the right. For `"C:\fakepath\video.mp4"`, it gives `"video.mp4"` as the first element.
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

The component does not perform the actual upload --- that happens in the parent form's save handler, which calls the `upload_video_file` JS bridge. This separation keeps the component focused on file selection and preview, while the parent coordinates the upload timing (only on form submit, not on file selection).
</details>

---

## DSA in Context: Strategy Pattern

The code you wrote in Exercise 2 uses the **strategy pattern**. The `upload` method behaves differently depending on which variant of `StorageBackend` is active, but the caller always uses the same interface.

**Interview version:** "Implement a notification system that supports Email, SMS, and Push notifications. Each channel has different delivery logic, but the caller should call the same `send(message)` method regardless of channel."

**Bonus challenge:** Extend the notification system so a single message can be sent to multiple channels simultaneously. How does `Arc` help if the channels need to be shared across async tasks?

---

## System Design Corner: File Upload Pipeline

**Interview question:** "Design a video upload system for a fitness app with 100K users."

**What we just built:** A two-backend storage system with layered validation.

**Talking points:**

- **Validation ordering** --- cheapest checks first (content type, extension) before expensive ones (reading bytes, magic byte inspection). This minimizes resource waste on malformed uploads.
- **UUID keys** --- prevent filename collisions and path traversal attacks. A million users can upload `video.mp4` and each gets a unique key.
- **Local vs cloud storage** --- the strategy pattern lets you develop locally (fast, no credentials needed) and deploy to production (R2/S3, CDN-backed) without changing application code.
- **Content-type enforcement** --- defense in depth. Check the declared type, the extension, AND the actual bytes. Any one of these could be spoofed; together they are robust.
- **Size limits** --- 100 MB max prevents storage abuse and keeps upload times reasonable. For larger files, you would implement chunked upload with resumability.

---

## Design Insight: Deep Modules

> John Ousterhout's *A Philosophy of Software Design* (Ch. 4) argues that the best modules have simple interfaces and deep implementations. `StorageBackend.upload()` has a four-parameter interface: key, data, content type, and the implicit `&self`. Behind that interface, the Local variant creates directories and writes files; the R2 variant authenticates with AWS-compatible credentials, serializes the request, handles retries, and uploads over HTTPS. The caller sees none of this. A shallow alternative would expose `upload_to_local()` and `upload_to_r2()` separately, forcing every caller to know which backend is active.

---

## What You Built

This chapter introduced two core Rust patterns through the video upload pipeline:

- **`Arc<T>`** --- shared ownership with atomic reference counting. Cheap to clone, safe across threads, freed when the last reference drops. Used to share `StorageBackend` across all Axum handlers.
- **Enum-based abstraction** --- the strategy pattern via `enum` + `match`. `StorageBackend::upload()` hides the difference between writing to local disk and uploading to Cloudflare R2. Adding a new backend means adding a variant --- the compiler finds every `match` that needs updating.
- **Magic byte validation** --- defense-in-depth beyond file extensions. Check the actual bytes to confirm the file is what it claims to be.
- **Multipart upload** --- Axum's `Multipart` extractor provides async field iteration with content type and file name metadata.
- **WASM-JS bridge** --- `#[wasm_bindgen(inline_js)]` for browser APIs where pure Rust bindings would be verbose.

If you run the app now and navigate to the exercise form, you should see a video upload section with URL and File mode tabs. Selecting a file shows the filename with a clear button. The actual upload fires when you save the exercise --- the parent form calls the JS bridge, which POSTs to `/api/v1/upload/video`, and the server validates, stores, and returns the URL.

The next chapter builds on the WASM bridge to implement PWA features: service worker registration, theme toggle with localStorage, and an install banner with iOS detection.

---

### DS Deep Dive

Your `Arc<StorageBackend>` shares one value across five handlers. But what IS Arc? How does atomic reference counting work? This deep dive builds Rc and Arc from scratch — raw pointers, atomic operations, and the Drop that frees memory.

**→ [Arc & Smart Pointers — "The Equipment Checkout System"](../ds-narratives/ch13-arc-smart-pointers.md)**
