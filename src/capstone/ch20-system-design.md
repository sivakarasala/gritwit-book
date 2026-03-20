# Chapter 20: System Design Deep Dive

This chapter is the system design capstone. It takes six questions that commonly appear in system design interviews and answers each one using the GrindIt fitness tracker as the running example. Every section follows the same format you would use in a 35-45 minute interview: clarify requirements, estimate capacity, sketch a high-level design, deep-dive into critical components, discuss scaling, and call out tradeoffs.

Read each section as a standalone 15-minute interview walkthrough. Practice explaining them out loud to a whiteboard or a friend.

---

## 20.1 Design a Fitness Tracker

### The Interview Question

> "Design a fitness tracking platform like SugarWOD or Wodify. Users can program workouts, log scores, view leaderboards, and track exercise history."

### Requirements Gathering

Before drawing a single box, ask clarifying questions:

- **Users and roles.** Are there distinct roles (athlete, coach, admin)? Can a coach program workouts that athletes then log scores against? *Yes -- three roles with a strict hierarchy.*
- **Core features.** WOD (Workout of the Day) programming, score logging with multiple scoring types (time, rounds, weight), exercise library, workout history, leaderboards. *Confirmed.*
- **Multi-tenancy.** Do we need to support multiple gyms from day one, or can we start single-tenant? *Start single-tenant, design for multi-tenant later.*
- **Platform.** Web only or native mobile? *PWA -- installable web app that works offline for basic features.*
- **Non-functional.** Latency target for page loads? Availability requirement? *Sub-200ms server response for logged-in pages. 99.9% uptime.*

### Capacity Estimation

Start with a single gym and project forward:

| Metric | Single Gym | 100 Gyms | 1,000 Gyms |
|--------|-----------|----------|------------|
| Registered users | 200 | 20,000 | 200,000 |
| Daily active users | 60 (30%) | 6,000 | 60,000 |
| WODs programmed/day | 1-2 | 100-200 | 1,000-2,000 |
| Score logs/day | 60 | 6,000 | 60,000 |
| Peak requests/sec | 2-3 | 200-300 | 2,000-3,000 |
| Storage (1 year) | 500 MB | 50 GB | 500 GB |

At single-gym scale, a single PostgreSQL instance handles everything comfortably. The architecture should not over-engineer for scale it does not yet need, but should not paint itself into a corner either.

### High-Level Design

```
┌──────────────┐       ┌──────────────────────────────┐
│   Browser    │──────▶│  Leptos SSR + Axum Server    │
│   (PWA)      │◀──────│  (Hydration + Server Fns)    │
└──────────────┘       └──────────┬───────────────────┘
                                  │
                       ┌──────────▼───────────────────┐
                       │  PostgreSQL                   │
                       │  (exercises, users, wods,     │
                       │   workout_logs, sessions)     │
                       └──────────────────────────────┘
```

The stack is deliberately simple: one binary serves both the HTML (SSR) and the API. Leptos server functions and REST endpoints both call the same `db.rs` query layer -- zero business logic duplication. The service worker provides offline caching and PWA installability.

Key components:

1. **Leptos SSR + Hydration.** Server renders the initial HTML for fast first paint; WASM hydrates on the client for interactivity. Routes are defined once in `app.rs` and shared between server and client.
2. **Axum HTTP layer.** Middleware stack: request ID generation, tracing, session management (tower-sessions with PostgreSQL-backed store), then Leptos routes. REST API is nested under `/api/v1`.
3. **PostgreSQL.** Single source of truth. SQLx compile-time query checking via `.sqlx/` offline data. Migrations in `migrations/` directory.
4. **Service worker.** Network-first for navigation, stale-while-revalidate for static assets. Versioned cache with automatic cleanup on activation.

### Deep Dives

**1. The "Two Doors, One Database" Pattern**

GrindIt serves two client types through the same binary: the Leptos UI (server functions) and third-party consumers (REST API). Both call identical `db.rs` functions:

```
Server Function (Leptos UI)          REST Handler (API clients)
        │                                      │
        └──────────┐          ┌────────────────┘
                   ▼          ▼
              db::list_exercises_db(&pool)
                       │
                       ▼
                   PostgreSQL
```

In `main.rs`, this is achieved by nesting the API router and merging Leptos routes into a single Axum `Router`. The `db()` function in `db.rs` resolves the pool from either Leptos context (inside `leptos_routes_with_context`) or the global `OnceLock<PgPool>` (for standalone server function calls).

**2. Authentication Architecture**

Three auth methods, one session system:

- **Google OAuth** (`auth/oauth.rs`): OAuth2 authorization code flow. On callback, upserts user in PostgreSQL, sets session.
- **Email/Password** (`auth/password.rs`): Argon2id hashing via the `argon2` crate. Timing-safe comparison. Same error message for wrong email and wrong password (prevents user enumeration).
- **Phone OTP** (`auth/otp.rs`): SMS-delivered one-time password. Verified server-side.

All three methods converge at `session::set_user_id()`, which stores the user UUID in a `tower-sessions` PostgreSQL-backed session. Every subsequent request resolves the user via `session::get_current_user()`. Role-based access uses `require_role(UserRole::Coach)` which checks `user.role.rank() >= min_role.rank()`.

**3. Data Model Evolution**

The migration history tells the story of iterative schema design. The initial schema had three tables (exercises, workout_logs, workout_exercises). Over 21 migrations, it grew to support:

- Users with multi-method auth (Google ID, email/password, phone/OTP)
- WODs with sections and movements (three-level hierarchy)
- Section-level scoring with multiple scoring types
- Movement-level logging with sets (weight, reps, distance, calories)
- Soft delete with ownership tracking

Each migration is a forward-only SQL file, applied by `sqlx::migrate!()` at startup. No down migrations -- rollbacks are achieved with new forward migrations.

### Scaling Discussion

| Scale | Architecture Change |
|-------|-------------------|
| **1x** (200 users) | Single server, single PostgreSQL. Current GrindIt. |
| **10x** (2,000 users) | Add connection pooling (PgBouncer), read replicas for leaderboard queries. CDN for static assets. |
| **100x** (20,000 users) | Horizontal app servers behind a load balancer. Redis for session store (replace PostgreSQL sessions). Background job queue for heavy operations. |
| **1000x** (200,000 users) | Multi-tenant schema (see Section 20.5). Sharding by gym. Dedicated leaderboard service with Redis sorted sets. |

### Tradeoffs

| Decision | Chosen | Alternative | Why |
|----------|--------|-------------|-----|
| SSR + Hydration | Leptos SSR | SPA (client-only) | Faster first paint, SEO, works without JS initially |
| Single binary | Axum serves everything | Separate API + frontend | Simpler deployment, shared types, one process to monitor |
| PostgreSQL sessions | tower-sessions-sqlx | Redis sessions | Fewer moving parts at small scale; swap later |
| Forward-only migrations | sqlx::migrate!() | Reversible migrations | Simpler, safer in production; rollback via new migration |
| OnceLock for pool | Global static | Dependency injection | Less boilerplate for server functions; testable via Leptos context override |

### Talking Points

- "I would start with the simplest architecture that meets latency targets -- a single Rust binary serving SSR HTML, hydrating on the client, backed by PostgreSQL."
- "The two-doors pattern means we get a REST API for free without duplicating business logic."
- "Auth converges at a single session layer regardless of login method. Adding a new method (e.g., Apple Sign-In) means writing one handler that calls `set_user_id` at the end."
- "I designed for single-tenant first but the schema supports multi-tenant evolution via a `gym_id` foreign key on every table."
- "The PWA service worker gives us offline page caching without the complexity of a native app."

---

## 20.2 Auth at Scale

### The Interview Question

> "Design a multi-method authentication system for a platform with 1 million users. Support social login (Google, Apple), email/password, and phone OTP. The system must handle 1,000 login attempts per second at peak."

### Requirements Gathering

- **Auth methods.** Google OAuth, Apple Sign-In, email/password, phone OTP. Users may link multiple methods to one account.
- **Security requirements.** Passwords hashed with a memory-hard algorithm. Rate limiting on login endpoints. Token/session rotation on privilege escalation. CSRF protection.
- **Session management.** How long do sessions last? *24 hours of inactivity. Absolute max 7 days.*
- **Account linking.** If a user signs up with email, then later logs in with Google using the same email, should accounts merge? *Yes, auto-link by verified email.*

### Capacity Estimation

| Metric | Value |
|--------|-------|
| Total users | 1,000,000 |
| Daily active users | 300,000 (30%) |
| Peak login attempts/sec | 1,000 |
| Active sessions | ~300,000 |
| Session store size | 300K x 512 bytes = 150 MB |
| Password hashes stored | ~600K (60% use email/password) |
| Argon2 hash time | ~250ms per attempt |
| Argon2 throughput (8 cores) | ~32 hashes/sec per core = 256/sec |

The Argon2 throughput number is critical. At 1,000 login attempts/sec, you need either 4 servers dedicated to password verification or you offload to a dedicated auth microservice. This is why social login and OTP are important -- they bypass the expensive hashing step.

### High-Level Design

```
┌─────────┐     ┌──────────────────┐     ┌──────────────┐
│ Client  │────▶│  API Gateway /   │────▶│  Auth Service │
│         │◀────│  Rate Limiter    │◀────│              │
└─────────┘     └──────────────────┘     └──────┬───────┘
                                                │
                        ┌───────────────────────┼───────────────┐
                        │                       │               │
                  ┌─────▼─────┐          ┌──────▼──────┐  ┌────▼─────┐
                  │ OAuth     │          │ Password    │  │ OTP      │
                  │ Providers │          │ Verifier    │  │ Service  │
                  │ (Google,  │          │ (Argon2)    │  │ (SMS)    │
                  │  Apple)   │          │             │  │          │
                  └───────────┘          └─────────────┘  └──────────┘
                                                │
                                         ┌──────▼──────┐
                                         │  User DB    │
                                         │  (PostgreSQL)│
                                         └──────┬──────┘
                                                │
                                         ┌──────▼──────┐
                                         │ Session     │
                                         │ Store       │
                                         │ (Redis)     │
                                         └─────────────┘
```

### Deep Dives

**1. Rate Limiting Strategy**

Three tiers of rate limiting:

- **IP-level.** 20 login attempts per minute per IP. Implemented at the API gateway (or Axum middleware with a token bucket). Blocks credential-stuffing attacks.
- **Account-level.** 5 failed attempts per account per 15 minutes. After 5 failures, require CAPTCHA. After 10, lock for 30 minutes. Stored in Redis with TTL.
- **Global.** Circuit breaker at 5,000 attempts/sec. If exceeded, return 503 and alert.

GrindIt's current implementation does not include rate limiting (single-user scale). At scale, you would add Tower middleware:

```rust
// Conceptual -- rate limit layer in the middleware stack
let rate_limit = tower::limit::RateLimitLayer::new(1000, Duration::from_secs(1));
let app = Router::new()
    .nest("/auth", auth_routes)
    .layer(rate_limit);
```

**2. Token Rotation and Session Security**

Current GrindIt uses `tower-sessions` with PostgreSQL storage. At scale:

- **Session ID rotation.** After login, call `session.cycle_id()` to prevent session fixation. GrindIt does this implicitly via the `tower-sessions` layer.
- **Absolute expiry.** Sessions expire after 7 days regardless of activity. Inactivity timeout at 24 hours.
- **Privilege escalation.** When a user changes their password or modifies security settings, invalidate all other sessions. Query: `DELETE FROM sessions WHERE user_id = $1 AND id != $2`.
- **At 300K active sessions.** Move from PostgreSQL to Redis. A single Redis instance handles millions of sessions with sub-millisecond reads.

**3. Account Linking**

The user table uses a design where `email` is the canonical linking key:

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    google_id TEXT UNIQUE,       -- NULL if not linked
    email TEXT UNIQUE,           -- canonical identifier
    phone TEXT UNIQUE,           -- NULL if not linked
    password_hash TEXT,          -- NULL if social-only
    role user_role DEFAULT 'athlete',
    ...
);
```

When a Google OAuth callback returns an email that already exists in the database, the system links the Google ID to the existing account rather than creating a duplicate. This is handled in the OAuth callback handler in `auth/oauth.rs`.

### Scaling Discussion

| Scale | Change |
|-------|--------|
| **10K users** | PostgreSQL sessions work fine. Argon2 on the app server. |
| **100K users** | Redis for sessions. Dedicated auth worker pool for Argon2. |
| **1M users** | Auth microservice. Rate limiting at CDN edge (Cloudflare). CAPTCHA integration. |
| **10M users** | Federated identity service. Passwordless push toward OAuth/WebAuthn. Geo-distributed session stores. |

### Tradeoffs

| Decision | Chosen | Alternative | Why |
|----------|--------|-------------|-----|
| Argon2id | Memory-hard, GPU-resistant | bcrypt | Better resistance to ASIC/GPU attacks; tunable memory cost |
| Session-based auth | Server-side sessions | JWT | Revocable, no token size issues, simpler security model |
| PostgreSQL sessions (small) | Simpler stack | Redis from day one | Fewer components to operate; swap is mechanical |
| Same error for wrong email/password | "Invalid email or password" | Specific error | Prevents user enumeration attacks |
| Auto-link by email | Merge accounts | Separate accounts per method | Better UX; email is verified by OAuth provider |

### Talking Points

- "Argon2 is intentionally slow -- 250ms per hash. That is a feature, not a bug. But it means password verification is CPU-bound and must be sized separately from general request handling."
- "I converge all auth methods to a single session layer. Adding Apple Sign-In is one new handler, not a new auth system."
- "Rate limiting happens at three levels: IP (gateway), account (Redis counter), and global (circuit breaker). Each catches a different attack pattern."
- "Sessions over JWTs because I need immediate revocation -- when a user changes their password, all other sessions die instantly."

---

## 20.3 Real-time Leaderboard

### The Interview Question

> "Design a live leaderboard for a fitness platform. 10,000 concurrent users are watching their gym's leaderboard update in real time as athletes log scores."

### Requirements Gathering

- **Update frequency.** How quickly must a new score appear? *Under 2 seconds from submission to display.*
- **Ranking logic.** Is it simple "highest score wins"? *No. Multiple scoring types: ForTime (lowest wins), AMRAP (highest wins), Strength (heaviest wins). Rx scores always rank above Scaled.*
- **Scope.** Per-WOD leaderboard? Per-gym? Global? *Per-WOD per gym initially. Global leaderboards later.*
- **Historical.** Do we need historical snapshots of leaderboard state? *No. Current ranking only.*
- **Ties.** How are ties broken? *Earlier submission wins.*

### Capacity Estimation

| Metric | Value |
|--------|-------|
| Concurrent viewers | 10,000 |
| Score submissions/sec (peak) | 50 (class of 20 finishes a WOD within 5 minutes) |
| Leaderboard reads/sec | 10,000 (each viewer polls or receives push) |
| Entries per leaderboard | 20-200 (gym members who did that WOD) |
| Leaderboard payload size | ~5 KB (200 entries x 25 bytes each) |
| Bandwidth (push to all) | 50 MB/sec peak (10K clients x 5 KB) |

The read:write ratio is 200:1. This is a classic read-heavy system -- perfect for caching.

### High-Level Design

Three evolutionary stages:

**Stage 1: Database-backed (current GrindIt)**

```
Client ──▶ Server Function ──▶ PostgreSQL
                                   │
                          ORDER BY is_rx DESC,
                                   score ASC/DESC,
                                   submitted_at ASC
```

The leaderboard is a query. Every page load runs:

```sql
SELECT u.display_name, sl.score_value, wl.is_rx, wl.created_at
FROM section_logs sl
JOIN workout_logs wl ON sl.workout_log_id = wl.id
JOIN users u ON wl.user_id = u.id
WHERE wl.wod_id = $1 AND sl.section_id = $2
ORDER BY wl.is_rx DESC, sl.score_value ASC, wl.created_at ASC
```

This works for up to ~1,000 concurrent users with proper indexing.

**Stage 2: Redis sorted sets**

```
Score submitted ──▶ Write to PostgreSQL
                         │
                         ▼
                  Publish to Redis ──▶ ZADD leaderboard:{wod_id} score user_id
                         │
Client polls ──▶ ZRANGEBYSCORE ──▶ Return top N
```

Redis sorted sets give O(log N) insertion and O(log N + M) range queries. For a 200-member leaderboard, both operations complete in microseconds.

The scoring key must encode Rx status and scoring type into the sort score:

```
score = (rx_flag * 1_000_000_000) + normalized_score
```

For ForTime, `normalized_score = MAX_TIME - actual_time` (invert so higher is better in the sorted set). For AMRAP and Strength, use the raw value.

**Stage 3: WebSocket push**

```
Score submitted ──▶ PostgreSQL + Redis ZADD
                         │
                         ▼
                  Redis PUBLISH leaderboard:{wod_id}
                         │
                         ▼
              WebSocket Hub ──▶ Push to all connected clients
```

Each client opens a WebSocket connection for the WOD they are viewing. When a score is submitted, the server publishes to a Redis channel. The WebSocket hub subscribes to that channel and fans out the updated leaderboard to all connected clients.

### Deep Dives

**1. Custom Sort Ordering**

GrindIt's leaderboard sorts differently depending on scoring type. In `db.rs`, the leaderboard query uses `ORDER BY wl.is_rx DESC` to always put Rx athletes above Scaled. Within each group, ForTime sorts ascending (faster is better) while AMRAP sorts descending (more rounds is better).

At the Redis layer, encode this into a single numeric score:

```
Rx ForTime 8:30  → 1_000_000_000 + (3600 - 510) = 1_000_003_090
Rx ForTime 9:00  → 1_000_000_000 + (3600 - 540) = 1_000_003_060
Scaled AMRAP 12  → 0_000_000_000 + 12            = 12
```

This single number preserves the full ordering in one `ZREVRANGEBYSCORE` call.

**2. Fan-out Architecture**

With 10,000 concurrent viewers of a single leaderboard, naive per-client database queries (polling) would generate 10,000 queries/sec. Instead:

- **Shared subscription.** One Redis subscription per WOD per server instance. The server maintains a `HashMap<WodId, Vec<WebSocketSender>>` mapping.
- **Broadcast channel.** Use `tokio::sync::broadcast` to fan out within a single server process. One Redis message triggers one broadcast, which wakes all connected WebSocket tasks.
- **Payload compression.** The full leaderboard (200 entries, ~5 KB) is small enough to send as a complete snapshot on each update. Delta encoding adds complexity without meaningful bandwidth savings at this size.

**3. Consistency Model**

The leaderboard is eventually consistent with a target of under 2 seconds staleness:

1. Score written to PostgreSQL (source of truth).
2. After successful write, `ZADD` to Redis and `PUBLISH` to the channel.
3. If Redis is unavailable, the leaderboard falls back to direct PostgreSQL queries.

This means a reader might briefly see stale data, but never incorrect data. The PostgreSQL write is the commit point -- if it fails, the score is not recorded.

### Scaling Discussion

| Scale | Architecture |
|-------|-------------|
| **100 concurrent** | Direct PostgreSQL query per page load. No caching. |
| **1,000 concurrent** | PostgreSQL with an indexed query. Optional Redis cache with 5-second TTL. |
| **10,000 concurrent** | Redis sorted sets + WebSocket push. One subscription per WOD per server. |
| **100,000 concurrent** | Multiple WebSocket servers behind a load balancer. Redis Cluster for sorted sets. Consider per-gym partitioning. |

### Tradeoffs

| Decision | Chosen | Alternative | Why |
|----------|--------|-------------|-----|
| Full snapshot per update | Send entire leaderboard | Delta/diff updates | Simpler client logic; 5 KB is tiny |
| Redis sorted sets | Single data structure for ranking | Application-level sort | O(log N) insert + range query; battle-tested |
| Eventually consistent | 2-second staleness ok | Strong consistency | Leaderboard does not need ACID; availability matters more |
| WebSocket | Persistent connection | SSE or long polling | Bidirectional (future chat), lower overhead than polling |

### Talking Points

- "The read:write ratio of 200:1 tells me this is a caching problem, not a database problem."
- "I encode Rx status and scoring type into a single numeric score so Redis sorted sets handle the full ordering in one operation."
- "I use tokio broadcast channels for in-process fan-out so one Redis message serves thousands of WebSocket clients."
- "The fallback path is always PostgreSQL. Redis is an optimization, not a requirement."

---

## 20.4 File Upload Pipeline

### The Interview Question

> "Design a video upload system for a fitness app. Users upload exercise demonstration videos (30-120 seconds, up to 100 MB). The system must validate, store, optionally transcode, and serve videos through a CDN."

### Requirements Gathering

- **File types.** MP4, WebM, MOV, AVI. *Confirmed.*
- **Size limits.** 100 MB max per upload. *Confirmed.*
- **Processing.** Do we need to generate thumbnails? Transcode to multiple resolutions? *Thumbnails yes, transcoding nice-to-have.*
- **Access control.** Are videos public or per-user? *Public once uploaded (exercise demonstrations).*
- **Latency.** How fast must the upload feel? *Upload completes in under 30 seconds on a reasonable connection. Transcoding can be async.*

### Capacity Estimation

| Metric | Value |
|--------|-------|
| Videos uploaded/day | 20 (coaches uploading demos) |
| Average video size | 30 MB |
| Daily upload volume | 600 MB |
| Annual storage | ~220 GB |
| Storage cost (R2) | ~$3.30/month at $0.015/GB |
| CDN egress | ~50 GB/month (1,000 views/day x 50 MB avg) |
| CDN cost (R2) | $0 (Cloudflare R2 has free egress) |

Video upload is a low-volume, high-size operation. The cost driver is storage, not compute.

### High-Level Design

```
┌──────────┐   multipart    ┌──────────────────────────────────┐
│  Client  │ ──────────────▶│  Upload Handler (Axum)           │
│          │                │  1. Auth check                   │
│          │                │  2. Extension allowlist           │
│          │                │  3. Size check (100 MB)          │
│          │                │  4. Magic byte validation         │
│          │                │  5. StorageBackend.upload()       │
└──────────┘                └──────────┬───────────────────────┘
                                       │
                        ┌──────────────┴──────────────┐
                        │                             │
                  ┌─────▼──────┐              ┌───────▼────────┐
                  │ Local FS   │              │ Cloudflare R2  │
                  │ (dev)      │              │ (production)   │
                  └────────────┘              └───────┬────────┘
                                                      │
                                              ┌───────▼────────┐
                                              │ Cloudflare CDN │
                                              │ (free egress)  │
                                              └────────────────┘
```

### Deep Dives

**1. The StorageBackend Enum (GrindIt's Implementation)**

GrindIt uses an enum-based strategy pattern in `storage.rs`:

```rust
pub enum StorageBackend {
    Local,
    R2 { bucket: Box<s3::Bucket>, public_url: String },
}
```

One `upload()` method, two implementations. The caller (the upload handler in `routes/upload.rs`) does not know or care which backend is active. The backend is selected at startup from configuration:

```rust
let storage = Arc::new(StorageBackend::from_config(&app_config.storage));
```

`Arc<StorageBackend>` is shared across all Axum handlers via `State(UploadState { storage, pool })`. This is a textbook deep module -- simple interface, complex internals hidden.

**2. Multi-Layer Validation**

GrindIt validates uploads at four levels (see `routes/upload.rs`):

1. **Content-Type header.** Must start with `video/`. Rejects obviously wrong MIME types.
2. **Extension allowlist.** Only `mp4`, `webm`, `mov`, `avi`, `m4v`. Blocks `.exe` renamed to `.mp4`.
3. **Size check.** `data.len() > MAX_UPLOAD_BYTES` (100 MB). Also enforced at the Axum layer with `DefaultBodyLimit::max(100 * 1024 * 1024)`.
4. **Magic byte validation.** `is_valid_video_magic()` checks the first 12 bytes for known container signatures: `ftyp` (MP4/MOV), EBML header (WebM), `RIFF....AVI` (AVI). This catches files with a spoofed extension.

This defense-in-depth approach means an attacker must bypass four independent checks to upload a malicious file.

**3. Scaling to a Full Pipeline**

At scale, the upload handler becomes the first stage of a pipeline:

```
Upload ──▶ Object Storage ──▶ Transcode Queue ──▶ Transcode Worker
                                                        │
                                               ┌────────┴────────┐
                                               │                 │
                                          720p MP4          Thumbnail
                                               │                 │
                                               ▼                 ▼
                                         Object Storage    Object Storage
                                               │
                                               ▼
                                         CDN (serve)
```

- **Presigned uploads.** At high volume, upload directly to R2 via a presigned URL. The server generates the URL, the client uploads without proxying through the app server. Eliminates the 100 MB memory footprint per concurrent upload.
- **Transcoding.** Use FFmpeg in a background worker (triggered by a message queue). Generate 720p and 360p variants. Store alongside the original.
- **Thumbnails.** Extract a frame at the 2-second mark during transcoding. Serve as the poster image.
- **Virus scanning.** For platforms accepting arbitrary files, add ClamAV scanning between upload and making the file publicly accessible.

### Scaling Discussion

| Scale | Architecture |
|-------|-------------|
| **20 uploads/day** | Proxy through app server to R2. No transcoding. Current GrindIt. |
| **200 uploads/day** | Presigned URLs for direct-to-R2 upload. Background thumbnail generation. |
| **2,000 uploads/day** | Dedicated upload service. Message queue (SQS/RabbitMQ) for transcoding jobs. Multiple FFmpeg workers. |
| **20,000 uploads/day** | Chunked/resumable uploads (tus protocol). Geo-distributed upload endpoints. Dedicated transcoding cluster. |

### Tradeoffs

| Decision | Chosen | Alternative | Why |
|----------|--------|-------------|-----|
| Proxy upload through server | Simpler auth, validation before storage | Presigned URL (direct to S3) | At 20 uploads/day, proxy overhead is negligible; simplifies validation |
| Enum dispatch | `StorageBackend` enum | Trait object (`Box<dyn Storage>`) | Enum is zero-cost at runtime; only two variants; no need for runtime polymorphism |
| R2 over S3 | Free egress | S3 (pay per GB egress) | Video serving is egress-heavy; R2 saves ~$0.09/GB |
| Magic byte check | Server-side validation | Trust Content-Type header | Headers are trivially spoofable; magic bytes are authoritative |

### Talking Points

- "I validate at four layers: MIME type, extension, size, and magic bytes. Each catches a different class of invalid upload."
- "The `StorageBackend` enum is a compile-time strategy pattern. Adding a third backend (e.g., Azure Blob) means adding one enum variant and one match arm."
- "At low volume, proxying through the server is simpler and lets me validate before storing. At high volume, I switch to presigned URLs and validate asynchronously."
- "I chose Cloudflare R2 specifically because video serving is egress-heavy and R2 has zero egress fees."

---

## 20.5 Multi-tenant Gym Platform

### The Interview Question

> "You have a single-gym fitness tracker. Extend it to support 1,000 gyms, each with their own members, workouts, and leaderboards. Ensure tenant isolation -- Gym A must never see Gym B's data."

### Requirements Gathering

- **Isolation level.** Do gyms need separate databases, or is schema-level isolation sufficient? *Schema-level (shared database, tenant column) is fine for 1,000 gyms. Regulatory/compliance isolation not required.*
- **Cross-gym features.** Can athletes belong to multiple gyms? Global leaderboards? *Athletes can be members of multiple gyms. Global leaderboards are a future feature.*
- **Admin hierarchy.** Gym-level admin vs platform-level super-admin? *Both. Gym admins manage their gym. Super-admins manage the platform.*
- **Data migration.** How do we migrate existing single-gym data? *Add `gym_id` column with a default value for the existing gym.*

### Capacity Estimation

| Metric | Value |
|--------|-------|
| Gyms | 1,000 |
| Users per gym (avg) | 150 |
| Total users | 150,000 |
| WODs per gym per day | 1-2 |
| Total WODs/day | 1,500 |
| Workout logs/day | 45,000 |
| Database size (1 year) | ~20 GB |
| Largest gym | 500 members, 50 WODs/week |

### High-Level Design

**Schema Evolution: Three Stages**

**Stage 1: Shared tables with `gym_id` column**

```sql
-- Migration: add gym_id to all tenant-scoped tables
ALTER TABLE exercises ADD COLUMN gym_id UUID REFERENCES gyms(id);
ALTER TABLE wods ADD COLUMN gym_id UUID REFERENCES gyms(id);
ALTER TABLE workout_logs ADD COLUMN gym_id UUID REFERENCES gyms(id);

-- Junction table for multi-gym membership
CREATE TABLE gym_memberships (
    user_id UUID REFERENCES users(id),
    gym_id UUID REFERENCES gyms(id),
    role gym_role NOT NULL DEFAULT 'athlete',
    PRIMARY KEY (user_id, gym_id)
);
```

Every query adds `WHERE gym_id = $1`. This is the simplest approach and works for 1,000 gyms on a single PostgreSQL instance.

**Stage 2: Row-Level Security (RLS)**

```sql
-- Enable RLS on exercises table
ALTER TABLE exercises ENABLE ROW LEVEL SECURITY;

CREATE POLICY gym_isolation ON exercises
    USING (gym_id = current_setting('app.current_gym_id')::uuid);
```

With RLS, even if application code forgets a `WHERE gym_id = $1` clause, PostgreSQL enforces isolation. The application sets `app.current_gym_id` at the start of each request:

```rust
sqlx::query("SET LOCAL app.current_gym_id = $1")
    .bind(gym_id)
    .execute(&pool)
    .await?;
```

**Stage 3: Schema-per-tenant (if needed)**

For gyms with strict data isolation requirements (e.g., military, healthcare-adjacent):

```
gym_001.exercises, gym_001.wods, gym_001.workout_logs
gym_002.exercises, gym_002.wods, gym_002.workout_logs
```

Each gym gets a PostgreSQL schema. Queries use `SET search_path = gym_001`. Migration tooling must iterate all schemas.

### Deep Dives

**1. Tenant Resolution**

How does the system know which gym a request belongs to? Three common approaches:

- **Subdomain.** `crossfit-oakland.grindit.app` -- parse the subdomain, resolve to `gym_id`. Best for branding.
- **Path prefix.** `grindit.app/g/crossfit-oakland/wod` -- resolve from the URL path. Simpler DNS.
- **Header/cookie.** After selecting a gym in the UI, store `gym_id` in the session. Simplest to implement.

GrindIt's architecture supports the session-based approach naturally. Add a `current_gym_id` field to the session, set it when the user selects a gym, and use it in all queries.

**2. Adapting GrindIt's Current Schema**

The existing migration for exercises:

```sql
CREATE TABLE exercises (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    ...
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX idx_exercises_name ON exercises (LOWER(name));
```

For multi-tenancy, the unique index must include `gym_id`:

```sql
-- Drop old index
DROP INDEX idx_exercises_name;
-- New composite unique index
CREATE UNIQUE INDEX idx_exercises_name_per_gym ON exercises (gym_id, LOWER(name));
```

Similarly, `list_exercises_db()` in `db.rs` changes from:

```sql
SELECT ... FROM exercises WHERE deleted_at IS NULL ORDER BY name
```

to:

```sql
SELECT ... FROM exercises WHERE deleted_at IS NULL AND gym_id = $1 ORDER BY name
```

With RLS enabled, even the original query (without `AND gym_id = $1`) would be safe -- but explicit filtering is clearer and allows the query planner to use the composite index.

**3. Cross-Gym Features**

Some data is naturally global (exercise definitions, movement standards), while other data is tenant-scoped (WODs, scores, memberships). The schema should distinguish:

```
Global tables:          Tenant-scoped tables:
  - exercise_catalog      - exercises (gym-specific demos/modifications)
  - movement_standards    - wods
  - platform_users        - workout_logs
                          - section_logs
                          - gym_memberships
```

A gym's exercise library inherits from the global catalog but can add custom exercises visible only to their members.

### Scaling Discussion

| Scale | Architecture |
|-------|-------------|
| **10 gyms** | Shared table + `gym_id` column. No RLS needed. |
| **100 gyms** | Enable RLS as defense-in-depth. Composite indexes. |
| **1,000 gyms** | Connection pooling (PgBouncer). Read replicas for leaderboards. Consider partitioning large tables by `gym_id`. |
| **10,000 gyms** | Shard by gym_id across multiple PostgreSQL instances. Dedicated database for the largest gyms. |

### Tradeoffs

| Decision | Chosen | Alternative | Why |
|----------|--------|-------------|-----|
| Shared database + `gym_id` | Simplest migration path | Database-per-tenant | 1,000 gyms is well within one PostgreSQL's capacity; avoids operational overhead of 1,000 databases |
| RLS | Defense-in-depth | Application-only filtering | Prevents data leaks from forgotten WHERE clauses; small performance cost (~5%) |
| Session-based tenant resolution | Simple, works with existing session infra | Subdomain routing | No DNS wildcard setup; works immediately |
| Multi-gym membership | Junction table | Duplicate user rows per gym | One user identity, many gym memberships; cleaner auth |

### Talking Points

- "I would start with the simplest multi-tenant approach: a `gym_id` column on every tenant-scoped table. At 1,000 gyms, a single PostgreSQL instance handles this easily."
- "RLS is my safety net. Even if a developer forgets `WHERE gym_id = $1`, PostgreSQL enforces isolation at the query planner level."
- "The unique exercise name index must become a composite index including `gym_id` -- otherwise Gym A creating 'Back Squat' would block Gym B from doing the same."
- "I distinguish global data (exercise catalog) from tenant data (gym-specific WODs). Gyms inherit from the catalog but own their customizations."

---

## 20.6 Offline-first PWA

### The Interview Question

> "Design offline sync for a fitness tracking PWA. Users should be able to log workouts when they have no internet connection (e.g., in a basement gym). When connectivity returns, changes must sync to the server without data loss."

### Requirements Gathering

- **Offline capabilities.** Which features must work offline? *Workout logging (primary), exercise library (read-only), workout history (read-only).*
- **Conflict resolution.** What if the same workout is edited on two devices while offline? *Last-write-wins for scores. Server is the source of truth for leaderboards.*
- **Offline duration.** How long might a user be offline? *Up to 4 hours (a long competition day in a venue with no signal).*
- **Data volume.** How much data do we need to cache locally? *Exercise library (~500 entries), recent WODs (last 7 days), user's own history (last 30 days).*

### Capacity Estimation

| Metric | Value |
|--------|-------|
| Exercise library | 500 entries x 200 bytes = 100 KB |
| Recent WODs (7 days) | 14 WODs x 2 KB = 28 KB |
| Workout history (30 days) | 30 entries x 500 bytes = 15 KB |
| Pending offline logs | 1-5 entries x 1 KB = 5 KB |
| Total IndexedDB usage | ~150 KB |
| IndexedDB limit (browser) | 50 MB - 2 GB (varies) |

The data is tiny. We are well within browser storage limits.

### High-Level Design

```
┌─────────────────────────────────────────────────────┐
│  Browser                                            │
│                                                     │
│  ┌──────────────┐    ┌───────────────────────────┐  │
│  │  Leptos UI   │◀──▶│  IndexedDB                │  │
│  │  (WASM)      │    │  - exercises (read cache)  │  │
│  │              │    │  - wods (read cache)        │  │
│  │              │    │  - history (read cache)     │  │
│  │              │    │  - pending_logs (write queue)│ │
│  └──────┬───────┘    └───────────────────────────┘  │
│         │                                           │
│  ┌──────▼───────┐                                   │
│  │ Service      │    ┌─────────────────────────┐    │
│  │ Worker       │───▶│  Sync Manager           │    │
│  │ (sw.js)      │    │  - Online? Push pending  │    │
│  │              │    │  - Offline? Queue writes  │    │
│  └──────────────┘    └─────────────────────────┘    │
└─────────────────────────┬───────────────────────────┘
                          │ (when online)
                          ▼
                  ┌───────────────┐
                  │  Server       │
                  │  (Axum +      │
                  │   PostgreSQL) │
                  └───────────────┘
```

### Deep Dives

**1. GrindIt's Current Service Worker**

The existing `sw.js` in `public/sw.js` implements two caching strategies:

- **Navigation requests (HTML pages).** Network-first with cache fallback. Fetches from the server; if the response is OK, caches it. If the network fails, serves the cached version.
- **Static assets (JS, CSS, images).** Stale-while-revalidate. Serves the cached version immediately while fetching an updated version in the background.

Cache versioning is handled via a `CACHE_VERSION` constant. On activation, the worker deletes old caches:

```javascript
const CACHE_VERSION = "v6";
// On activate: delete caches where key !== current STATIC_CACHE
```

This is a solid foundation but does not yet handle offline data mutations.

**2. Offline Write Queue**

The key addition for offline-first is a write queue in IndexedDB:

```
IndexedDB Schema:
  pending_logs (object store)
    - id: UUID (generated client-side)
    - type: "workout_log" | "score_update"
    - payload: { wod_id, sections, scores, ... }
    - created_at: timestamp
    - sync_status: "pending" | "syncing" | "synced" | "failed"
    - retry_count: 0
```

The flow:

1. **User logs a workout offline.** The Leptos UI detects `!navigator.onLine` (or the server function fails with a network error). Instead of showing an error, it writes to IndexedDB's `pending_logs` store and shows a "Saved locally -- will sync when online" toast.
2. **Connectivity returns.** The `online` event fires. The sync manager reads all `pending` entries from IndexedDB and submits them to the server one by one.
3. **Server confirms.** Each successful sync updates the entry's `sync_status` to `"synced"`. The UI shows a "Synced!" indicator.
4. **Server rejects.** If the server returns a validation error (e.g., duplicate log), mark as `"failed"` and surface the error to the user.

**3. Conflict Resolution**

For a fitness tracker, conflicts are rare and the resolution strategy is straightforward:

- **Last-write-wins by timestamp.** Each workout log has a `client_timestamp` (when the user pressed "Submit") and a `server_timestamp` (when the server received it). If two offline devices submit a log for the same WOD, both are stored -- they are separate workout log entries, not conflicting edits of the same record.
- **Idempotency via client-generated UUID.** The client generates a UUID for each workout log before writing to IndexedDB. The server uses `INSERT ... ON CONFLICT (id) DO NOTHING`. If the sync manager retries a failed submission, the second attempt is silently ignored.
- **Leaderboard recalculation.** When offline logs sync, the leaderboard query naturally includes them on the next read. No special reconciliation needed.

The one true conflict case is if a user edits the *same* workout log from two devices while both are offline. Resolution: compare `client_timestamp`, keep the newer edit, discard the older one. The server's `updated_at` column tracks this:

```sql
INSERT INTO workout_logs (id, ..., updated_at)
VALUES ($1, ..., $2)
ON CONFLICT (id) DO UPDATE SET
    notes = EXCLUDED.notes,
    updated_at = EXCLUDED.updated_at
WHERE workout_logs.updated_at < EXCLUDED.updated_at;
```

### Scaling Discussion

| Scale | Architecture |
|-------|-------------|
| **Basic offline** | Service worker caches pages. No offline data mutation. Current GrindIt. |
| **Offline logging** | IndexedDB write queue + sync manager. Client-generated UUIDs. |
| **Full offline-first** | IndexedDB mirrors server data. Reads never hit the network. Background sync keeps local store fresh. |
| **Multi-device sync** | Server-sent events notify other devices when data changes. CRDTs for collaborative editing (overkill for a fitness tracker). |

### Tradeoffs

| Decision | Chosen | Alternative | Why |
|----------|--------|-------------|-----|
| Network-first for pages | Fresh content when online | Cache-first (offline-first) | Fitness data changes frequently; stale WODs are worse than a brief loading state |
| IndexedDB for offline data | Structured storage, indexes, transactions | localStorage | localStorage has 5-10 MB limit and no indexing; IndexedDB handles structured data better |
| Client-generated UUIDs | Idempotent retries, no server roundtrip for ID | Server-generated IDs | Offline clients cannot ask the server for an ID; UUIDv4 collisions are astronomically unlikely |
| Last-write-wins | Simple, predictable | CRDTs, operational transforms | Fitness logs are rarely concurrently edited; complexity of CRDTs is not justified |
| Stale-while-revalidate for assets | Instant loads, background refresh | Cache-first with manual invalidation | Best UX tradeoff; user gets instant response while fresh content loads |

### Talking Points

- "The service worker gives us the foundation -- cached pages work offline. IndexedDB adds the write side: a queue of pending mutations that sync when connectivity returns."
- "Client-generated UUIDs are essential for offline-first. They make submissions idempotent -- the sync manager can safely retry without creating duplicates."
- "I use network-first for navigation because fitness data (today's WOD, leaderboard) changes frequently. Stale-while-revalidate for static assets gives instant loads."
- "Conflict resolution is simple for this domain: workout logs are append-only events. Two logs for the same WOD from different devices are both valid -- they are separate workout sessions, not conflicting edits."
- "The total offline data footprint is under 200 KB. We are nowhere near browser storage limits, so there is no need for complex eviction policies."

---

## Summary: Patterns Across All Six Designs

Looking across these six system design problems, several patterns recur:

**Start simple, scale deliberately.** Every design starts with the simplest architecture that meets requirements. PostgreSQL handles sessions, leaderboards, and multi-tenancy at small scale. Redis, WebSockets, and sharding are introduced only when specific bottlenecks emerge.

**Defense in depth.** Auth has three rate-limiting tiers. Video upload has four validation layers. Multi-tenancy has both application-level filtering and database-level RLS. No single layer is trusted to be the only line of defense.

**Enum-based abstraction.** GrindIt uses Rust enums as a zero-cost strategy pattern: `StorageBackend` (Local vs R2), `UserRole` (Athlete/Coach/Admin), `Environment` (Local/Production). Each provides a simple interface and hides variant-specific complexity behind `match`.

**The "two doors" principle.** Server functions and REST endpoints share the same database layer. Offline clients and online clients converge at the same sync endpoint. Multiple auth methods converge at the same session layer. Every system has a single source of truth with multiple access paths.

**Tradeoffs are explicit.** Every design section includes a tradeoff table. In an interview, naming the tradeoff you considered (even if you chose the simpler option) signals maturity. "I chose PostgreSQL sessions over Redis because at this scale, fewer moving parts outweighs the latency benefit" is a stronger answer than "I used Redis for sessions."

Use these six designs as templates. In an interview, you will not have time for this level of detail -- but you should be able to hit the requirements, capacity estimation, high-level design, one deep dive, and two tradeoffs within 35 minutes. Practice until that flow is automatic.
