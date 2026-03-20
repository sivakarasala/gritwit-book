# Chapter 7: User Authentication

Until now, GrindIt has been wide open. Anyone can create exercises, browse the library, and flip through pages. That is fine for a personal project on your laptop, but the moment you put a web app on the internet, you need to answer a fundamental question: **who is using this?**

This chapter builds a complete authentication system. Users will be able to sign in with their phone number (via a one-time code), with an email and password, or with their Google account. Once signed in, the server remembers them using sessions. And we will build guard functions that protect certain pages and actions --- only coaches can program workouts, and only logged-in users can record scores.

The spotlight concept is **enums and pattern matching** --- arguably the single most powerful feature Rust brings to the table compared to languages like JavaScript, Python, or Go. By the end of this chapter, you will see why Rust developers rave about `match`.

By the end of this chapter, you will have:

- A `UserRole` enum with `Athlete`, `Coach`, and `Admin` variants, complete with `rank()`, `Display`, and `From<String>` implementations
- `tower-sessions` with a PostgreSQL session store for persistent, server-side sessions
- `get_session()`, `get_current_user()`, `require_auth()`, and `require_role()` guard functions
- Google OAuth2 login (redirect flow with CSRF protection)
- Email/password registration and login with Argon2 hashing
- Phone OTP login with rate limiting and expiry
- A `LoginPage` component with tabbed authentication methods

---

## Spotlight: Enums & Pattern Matching

### The real-world problem

Imagine you are building a gym app. There are three kinds of users:

1. **Athletes** --- they work out and log scores
2. **Coaches** --- they program workouts and manage athletes
3. **Admins** --- they run the whole system

How would you represent this in code?

> **Programming Concept: What is an Enum?**
>
> An **enum** (short for "enumeration") is a type that can be one of several named options. Think of a traffic light: it can be Red, Yellow, or Green --- and nothing else. It cannot be "Purple" or "Half-Red." The set of options is fixed and known.
>
> In many languages, enums are just numbers with names. In Rust, enums are much more powerful --- each option can carry different data. Imagine a vending machine button: the "Soda" button might carry a flavor choice, the "Snack" button might carry a size, and the "Cancel" button carries nothing. Each button is a different kind of action with different associated information.
>
> Why does this matter? Because the compiler can check that you handle every possible option. If a traffic light adds a "Flashing Yellow" state tomorrow, the compiler will find every place in your code that checks the light's state and force you to handle the new one.

In JavaScript, you would probably use strings:

```javascript
const role = "coach"; // Could be "athlete", "coach", or "admin"
```

This works, but it has a sneaky problem. What stops someone from writing `"Coach"` (capital C), or `"coaches"` (plural), or `"couch"` (typo)? The answer is: nothing. Any string is a valid string, so the system has no way to prevent these mistakes.

### Rust enums: a closed set of options

Rust enums define exactly which values are allowed:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Athlete,
    Coach,
    Admin,
}
```

This means:

- There are exactly **three** possible roles. No more, no less.
- You cannot accidentally create a `UserRole::Couch` --- it does not exist, and the compiler will refuse to compile it.
- You cannot pass a `String` where a `UserRole` is expected. They are completely different types. It would be like trying to put a basketball into a keyhole --- the shapes simply do not fit.

### Adding behavior to enums

Enums in Rust are not just labels. You can add methods to them, just like you add methods to structs:

```rust
impl UserRole {
    pub fn rank(&self) -> u8 {
        match self {
            UserRole::Athlete => 0,
            UserRole::Coach => 1,
            UserRole::Admin => 2,
        }
    }
}
```

The `rank()` method assigns a number to each role. This is how GrindIt decides whether you have permission to do something: if you need Coach-level access, your rank must be at least 1. Athletes (rank 0) fail the check. Coaches (rank 1) pass. Admins (rank 2) pass too.

> **Programming Concept: What is Pattern Matching?**
>
> Pattern matching is looking at a value and doing different things depending on what it is. Think of a mail sorter at the post office: letters go to one slot, packages to another, and international mail to a third. The sorter looks at the shape and label and routes each item to the right place.
>
> In Rust, `match` is the mail sorter. You give it a value, and it checks the value against a list of patterns. The first pattern that matches wins, and the code for that pattern runs.
>
> The crucial difference from `if/else` chains: Rust's `match` is **exhaustive**. You must handle every possible case. If you forget one, the compiler will not let you build the program. This is like a mail sorter that refuses to start until every possible mail type has a designated slot.

### `match`: your new favorite keyword

The `match` expression is like a super-powered `switch` statement. Here is a simple example:

```rust
match user.role {
    UserRole::Athlete => "Can log workouts and view scores",
    UserRole::Coach   => "Can program WODs and manage athletes",
    UserRole::Admin   => "Full system access",
}
```

Try removing one of those lines. The compiler will immediately complain:

```
error[E0004]: non-exhaustive patterns: `UserRole::Admin` not covered
```

This is the magic. If you ever add a fourth role (say, `UserRole::Owner`), every single `match` on `UserRole` in your entire codebase will break at compile time. The compiler becomes your safety net --- no forgotten edge cases, no "oops, I forgot to handle the new role in the permissions check."

Compare this to JavaScript's `switch`, which happily falls through unmatched cases and does not care if you forget one.

### Match with guards: adding conditions

You can add extra conditions to match arms:

```rust
match user.role {
    UserRole::Admin => "Full access",
    UserRole::Coach if user.id == wod.created_by => "Can edit own WODs",
    UserRole::Coach => "Can view all WODs",
    UserRole::Athlete => "View scores only",
}
```

The `if user.id == wod.created_by` part is a **match guard**. It narrows the pattern: this arm only fires if the user is a Coach AND they created the WOD. If they are a Coach but not the owner, the next arm catches them.

Order matters --- Rust checks arms from top to bottom. The guarded arm must come before the un-guarded one.

### `Option<T>`: the "might not exist" type

Rust does not have `null` or `undefined`. Instead, it uses the `Option` enum:

```rust
// Option is defined like this (you don't write this yourself):
enum Option<T> {
    Some(T),  // There IS a value, and here it is
    None,     // There is NO value
}
```

When you have a value that might not exist --- like a user's email (some users sign up with phone only) --- you use `Option<String>`:

```rust
pub email: Option<String>,  // Some("alice@example.com") or None
```

To work with Options, you can use `match`, but Rust also provides convenient shortcuts:

```rust
// Full match --- handle both cases
match session.get("user_id").await? {
    Some(uid) => { /* use uid */ },
    None => { /* no session */ },
}

// if let --- when you only care about Some
if let Some(uid) = session.get("user_id").await? {
    // use uid --- this only runs if there IS a value
}

// let-else --- early return if None
let Some(uid) = session.get("user_id").await? else {
    return Ok(None);  // bail out early
};
// uid is available here, guaranteed to be Some
```

GrindIt uses `let-else` extensively. It reads like English: "let this be Some, otherwise return."

### `impl Display`: controlling how your type prints

The `Display` trait controls what happens when you use `{}` in a format string:

```rust
impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Athlete => write!(f, "athlete"),
            UserRole::Coach => write!(f, "coach"),
            UserRole::Admin => write!(f, "admin"),
        }
    }
}
```

Now `format!("Role: {}", user.role)` produces `"Role: coach"`. GrindIt also uses this when storing the role in the database --- the `Display` output matches the PostgreSQL enum values.

### Enums that carry data

Here is where Rust enums become truly different from other languages. Each variant can carry different data:

```rust
enum AuthMethod {
    Password { email: String, hash: String },
    Google { google_id: String, token: String },
    Phone { number: String, otp: String },
}
```

Each variant has its own fields. This is impossible in TypeScript enums (which are just numbers) and awkward in most other languages. In Rust, the compiler knows exactly which fields are available after you match on the variant:

```rust
match auth_method {
    AuthMethod::Password { email, hash } => {
        // email and hash are available here
    }
    AuthMethod::Google { google_id, token } => {
        // google_id and token are available here
    }
    AuthMethod::Phone { number, otp } => {
        // number and otp are available here
    }
}
```

No type guards. No runtime checks. The compiler guarantees you are accessing the right fields.

> **Coming from JS?**
>
> | Concept | TypeScript | Rust |
> |---------|-----------|------|
> | Simple enum | `enum Role { Athlete, Coach, Admin }` (numeric) | `enum UserRole { Athlete, Coach, Admin }` |
> | String enum | `enum Role { Athlete = "athlete" }` | `impl Display` for custom string output |
> | Union type | `type Result = { ok: true, data: T } \| { ok: false, error: E }` | `enum Result<T, E> { Ok(T), Err(E) }` |
> | Exhaustive check | Not enforced (needs `never` tricks) | Compiler error if a variant is missing |
> | Data in variants | Discriminated union with manual type guards | Native --- each variant holds typed fields |
>
> The biggest difference: TypeScript union types require runtime type guards (`if ('ok' in result)`). Rust enums are checked at compile time --- the compiler knows which variant you are in after a `match` arm, and you can access the inner data directly.

---

## Exercise 1: Define UserRole with rank(), Display, and From\<String\>

**Goal:** Build the `UserRole` enum, the `AuthUser` struct, and the helper functions that the rest of the auth system depends on.

### Step 1: Create the auth module

Create the directory structure:

```
src/
+-- auth/
|   +-- mod.rs
|   +-- session.rs
|   +-- oauth.rs
|   +-- password.rs
|   +-- otp.rs
|   +-- validation.rs
+-- lib.rs
+-- ...
```

> **Programming Concept: What is Authentication?**
>
> Authentication is proving who you are. Think of it like arriving at a gym:
>
> 1. **You show your membership card** (provide credentials)
> 2. **The front desk checks it** (the server verifies your credentials)
> 3. **They give you a wristband** (the server gives you a session)
> 4. **You show the wristband to enter any room** (the browser sends the session cookie with every request)
>
> The wristband is the key insight. The gym does not check your membership card every time you walk into a different room --- they checked it once at the front desk and gave you a wristband that proves you belong. Web sessions work exactly the same way.

### Step 2: Define the core types

`src/auth/mod.rs`:

```rust
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Strip internal prefixes from ServerFnError messages for user-friendly display.
pub fn clean_error(e: &ServerFnError) -> String {
    let raw = e.to_string();
    raw.strip_prefix("error running server function: ")
        .or_else(|| raw.strip_prefix("ServerFnError: "))
        .unwrap_or(&raw)
        .to_string()
}

#[cfg(feature = "ssr")]
pub mod oauth;
pub mod otp;
pub mod password;
#[cfg(feature = "ssr")]
pub mod session;
#[cfg(feature = "ssr")]
mod validation;
#[cfg(feature = "ssr")]
pub use validation::*;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OtpResult {
    NewAccount,
    Existing,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Athlete,
    Coach,
    Admin,
}

impl UserRole {
    pub fn rank(&self) -> u8 {
        match self {
            UserRole::Athlete => 0,
            UserRole::Coach => 1,
            UserRole::Admin => 2,
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Athlete => write!(f, "athlete"),
            UserRole::Coach => write!(f, "coach"),
            UserRole::Admin => write!(f, "admin"),
        }
    }
}
```

Let us walk through the module organization:

- **`oauth`, `session`, and `validation`** are gated behind `#[cfg(feature = "ssr")]`. These modules use server-only dependencies like `sqlx`, `tower_sessions`, and `argon2`. They do not exist in the browser build.
- **`otp` and `password`** are available in both features. They contain `#[server]` functions --- Leptos generates client-side stubs that make HTTP calls to the server. The actual logic runs on the server, but the function signature exists on the client.
- **`OtpResult`** is a simple enum with two variants. When a phone OTP is verified, the server tells the client whether this is a brand-new account or an existing one. The client uses this to decide where to redirect: new accounts go to `/profile` (to set their name), existing accounts go to `/`.

### Step 3: Define the AuthUser struct

Add to `src/auth/mod.rs`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub gender: Option<String>,
}

impl AuthUser {
    pub fn initials(&self) -> String {
        self.display_name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }

    /// Returns the best available user identifier (email, phone, or fallback).
    pub fn identifier(&self) -> &str {
        self.email
            .as_deref()
            .or(self.phone.as_deref())
            .unwrap_or("")
    }
}
```

Let us trace through the `identifier()` method step by step:

1. `self.email` is `Option<String>` --- might be `Some("alice@example.com")` or `None`
2. `.as_deref()` converts `Option<String>` to `Option<&str>` --- this avoids cloning the string
3. `.or(self.phone.as_deref())` --- if email is `None`, try the phone number instead
4. `.unwrap_or("")` --- if both are `None`, use an empty string as a last resort

This is Rust's alternative to `user.email || user.phone || ""` in JavaScript --- but type-safe, because you cannot accidentally treat `None` as a string.

### Step 4: Add the get_me server function

```rust
#[server]
pub async fn get_me() -> Result<Option<AuthUser>, ServerFnError> {
    let result = session::get_current_user().await;
    match &result {
        Ok(Some(u)) => tracing::info!("get_me: authenticated as {}", u.identifier()),
        Ok(None) => tracing::info!("get_me: no session found"),
        Err(e) => tracing::warn!("get_me: error: {}", e),
    }
    result
}
```

This server function is how the client checks authentication status. Notice the `match` on `result` --- it handles three cases:

1. `Ok(Some(u))` --- success, and there IS a user
2. `Ok(None)` --- success, but no one is logged in
3. `Err(e)` --- something went wrong

This is nested pattern matching: `Result` (Ok/Err) wrapping `Option` (Some/None). In JavaScript, you would need multiple `if` checks and null guards. In Rust, the compiler ensures you handle every combination.

### Step 5: Create the users migration

```sql
-- migrations/XXXXXX_create_users_table.sql
CREATE TYPE user_role AS ENUM ('athlete', 'coach', 'admin');

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    google_id TEXT UNIQUE,
    email TEXT UNIQUE,
    phone TEXT UNIQUE,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    password_hash TEXT,
    role user_role NOT NULL DEFAULT 'athlete',
    gender TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_google_id ON users (google_id);
CREATE INDEX idx_users_email ON users (email);
CREATE INDEX idx_users_phone ON users (phone);
```

The PostgreSQL `user_role` enum mirrors the Rust `UserRole` enum. You now have type safety on both sides --- Rust prevents invalid variants in code, PostgreSQL prevents invalid values in storage.

Why three indexes? Because users can sign in three ways (Google, email, phone), and each login method needs to look up the user quickly. Without an index, the database would scan every row.

<details>
<summary>Hint: If you see "type user_role does not exist"</summary>

The `CREATE TYPE` statement must run before the `CREATE TABLE` that references it. If you split the migration into multiple files, make sure the enum creation migration has an earlier timestamp in its filename. In GrindIt, both are in the same migration file to guarantee ordering.

</details>

---

## Exercise 2: Session Management with tower-sessions

**Goal:** Set up PostgreSQL-backed sessions and build the guard functions that protect server-side routes.

> **Programming Concept: What is a Session?**
>
> A session is the server remembering who you are between page loads. Think of it like a wristband at a music festival:
>
> 1. At the gate, you show your ticket (log in with credentials)
> 2. The gate gives you a wristband with a unique number (the server sets a session cookie)
> 3. Every time you enter a stage area, security scans your wristband (the browser sends the cookie)
> 4. They look up your wristband number in their system to see your access level (the server checks the session in the database)
>
> The wristband itself does not contain your personal information --- it is just a number. All the actual data stays in the festival's computer system. Web sessions work the same way: the cookie is just an ID, and all the data lives on the server.
>
> This is different from JWTs (JSON Web Tokens), where all the user information is stuffed into the cookie itself. JWTs are like laminated ID badges that contain your photo, name, and access level. They are convenient but harder to revoke --- if you lose the badge, anyone can use it until it expires.

### Step 1: Add dependencies

In `Cargo.toml`, add under `[dependencies]`:

```toml
[dependencies]
tower-sessions = { version = "0.14", optional = true }
tower-sessions-sqlx-store = { version = "0.14", features = ["postgres"], optional = true }
```

Add to the `ssr` feature:

```toml
[features]
ssr = [
    # ... existing deps ...
    "dep:tower-sessions",
    "dep:tower-sessions-sqlx-store",
]
```

### Step 2: Configure the session layer

In your `main.rs` (or startup code), add the session layer to Axum:

```rust
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::PostgresStore;

// After creating the PgPool:
let session_store = PostgresStore::new(pool.clone());
session_store.migrate().await.expect("Failed to migrate session store");

let session_layer = SessionManagerLayer::new(session_store)
    .with_same_site(tower_sessions::cookie::SameSite::Lax)
    .with_http_only(true)
    .with_secure(false); // Set to true in production with HTTPS

// Add to Axum router:
let app = Router::new()
    // ... routes ...
    .layer(session_layer);
```

Let us unpack each setting:

- **`PostgresStore`** stores sessions in a database table. It creates a `tower_sessions` table automatically when you call `.migrate()`.
- **`SameSite::Lax`** controls when the browser sends the cookie. "Lax" means: send it on normal navigation (clicking links, typing URLs) but not on cross-site requests from other websites. This prevents most CSRF attacks.
- **`http_only(true)`** means JavaScript cannot read the cookie. This protects against XSS attacks --- even if an attacker injects JavaScript into your page, they cannot steal the session cookie.
- **`secure(false)`** means the cookie works over HTTP (not just HTTPS). Set this to `true` in production, where you should always use HTTPS.

### Step 3: Build the session helper functions

`src/auth/session.rs`:

```rust
use super::{AuthUser, UserRole};
use leptos::prelude::*;
use tower_sessions::Session;

const USER_ID_KEY: &str = "user_id";

pub async fn get_session() -> Result<Session, ServerFnError> {
    let session: Session = leptos_axum::extract()
        .await
        .map_err(|e| ServerFnError::new(format!("Session extraction failed: {}", e)))?;
    Ok(session)
}

pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    let session = get_session().await?;
    let user_id: Option<String> = session
        .get(USER_ID_KEY)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let Some(uid) = user_id else {
        return Ok(None);
    };

    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = uid
        .parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    let user = match crate::db::get_user_by_id(&pool, user_uuid).await {
        Ok(u) => Some(u),
        Err(e) => {
            tracing::error!("get_user_by_id failed: {:?}", e);
            None
        }
    };
    Ok(user)
}

pub async fn require_auth() -> Result<AuthUser, ServerFnError> {
    get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))
}

pub async fn require_role(min_role: UserRole) -> Result<AuthUser, ServerFnError> {
    let user = require_auth().await?;
    if user.role.rank() >= min_role.rank() {
        Ok(user)
    } else {
        Err(ServerFnError::new("Insufficient permissions"))
    }
}

pub async fn set_user_id(session: &Session, user_id: &str) -> Result<(), ServerFnError> {
    session
        .insert(USER_ID_KEY, user_id.to_string())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

### Step 4: Understand the guard pattern

These functions form a hierarchy, like layers of security at a building:

```
get_current_user()  ->  Option<AuthUser>  (None = not logged in)
       |
require_auth()      ->  AuthUser          (Err = redirect to login)
       |
require_role(Coach) ->  AuthUser          (Err = insufficient permissions)
```

Think of it as three checkpoints:

1. **`get_current_user()`** --- "Is anyone there?" Returns `Some(user)` or `None`
2. **`require_auth()`** --- "You MUST be someone." Returns the user or an error
3. **`require_role(Coach)`** --- "You must be at least a Coach." Checks the role hierarchy

Every server function that needs protection calls one of these at the very top:

```rust
#[server]
pub async fn create_wod(title: String, ...) -> Result<String, ServerFnError> {
    // Only coaches and admins can create WODs
    let user = crate::auth::session::require_role(UserRole::Coach).await?;
    // ... rest of the function
}
```

The `?` operator is the key. If `require_role` returns `Err("Unauthorized")`, the `?` immediately returns that error to the client. The rest of the function never runs. This is a one-line security check --- clean, readable, and impossible to forget.

<details>
<summary>Hint: If leptos_axum::extract() fails</summary>

The `leptos_axum::extract()` function pulls Axum extractors from the request context. It only works inside a server function that is being called via an HTTP request. If you call it from a background task, it will fail.

Also verify that the `SessionManagerLayer` is added to your Axum router BEFORE the Leptos handler. Layers in Axum run in reverse order of how they are added. The session layer must process the request before Leptos tries to extract the session.

</details>

---

## Exercise 3: Build the OAuth2 Login Flow

**Goal:** Implement Google OAuth2 with CSRF protection, user upsert, and session creation.

### Step 1: Add OAuth2 dependencies

```toml
[dependencies]
oauth2 = { version = "5", optional = true }
reqwest = { version = "0.12", features = ["json"], optional = true }
secrecy = { version = "0.10", features = ["serde"] }

[features]
ssr = [
    # ... existing ...
    "dep:oauth2",
    "dep:reqwest",
]
```

### Step 2: Build the OAuth client

`src/auth/oauth.rs`:

```rust
use axum::{
    extract::{Query, State},
    response::Redirect,
};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use tower_sessions::Session;

const CSRF_STATE_KEY: &str = "oauth_csrf_state";

#[derive(Clone)]
pub struct OAuthState {
    pub client: oauth2::basic::BasicClient,
    pub pool: sqlx::PgPool,
}

pub fn build_oauth_client(config: &OAuthSettings) -> oauth2::basic::BasicClient {
    use secrecy::ExposeSecret;

    oauth2::basic::BasicClient::new(ClientId::new(
        config.google_client_id.expose_secret().clone(),
    ))
    .set_client_secret(ClientSecret::new(
        config.google_client_secret.expose_secret().clone(),
    ))
    .set_auth_uri(
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .expect("Invalid auth URL"),
    )
    .set_token_uri(
        TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .expect("Invalid token URL"),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.redirect_url.clone()).expect("Invalid redirect URL"),
    )
}
```

### Step 3: The login redirect

```rust
pub async fn google_login(
    State(state): State<OAuthState>,
    session: Session,
) -> Result<Redirect, axum::http::StatusCode> {
    let (auth_url, csrf_token) = state
        .client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .url();

    // Store CSRF token in session --- we verify it in the callback
    session
        .insert(CSRF_STATE_KEY, csrf_token.secret().clone())
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::temporary(auth_url.as_str()))
}
```

The OAuth2 flow works like checking into a hotel through a travel agent:

1. **You go to the travel agent (GrindIt):** "I want to check in at Hotel Google."
2. **The agent gives you a reference number (CSRF token)** and sends you to the hotel.
3. **At the hotel (Google), you prove your identity** and they give you a confirmation code.
4. **You bring the confirmation code back to the agent.** The agent checks that the reference number matches (CSRF validation), then exchanges the code for your room key (access token).
5. **The agent uses the room key to get your profile information** from the hotel and checks you in.

The CSRF token prevents a specific attack: without it, an attacker could craft a URL that logs you into THEIR Google account on your GrindIt instance.

### Step 4: The callback handler

```rust
#[derive(Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: String,
    name: String,
    picture: Option<String>,
}

pub async fn google_callback(
    State(state): State<OAuthState>,
    session: Session,
    Query(params): Query<CallbackParams>,
) -> Result<Redirect, axum::http::StatusCode> {
    // 1. Validate CSRF token
    let stored_state: Option<String> = session
        .get(CSRF_STATE_KEY)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(stored) = stored_state else {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    };

    if stored != params.state {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    session.remove::<String>(CSRF_STATE_KEY).await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // 2. Exchange authorization code for access token
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let token_result = state
        .client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&http_client)
        .await
        .map_err(|e| {
            tracing::error!("Token exchange failed: {:?}", e);
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // 3. Fetch user info from Google
    let userinfo: GoogleUserInfo = http_client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(token_result.access_token().secret())
        .send()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .json()
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // 4. Determine role --- first user becomes admin
    let role = crate::auth::default_role_for_new_user(&state.pool)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // 5. Upsert user
    let user_id: (uuid::Uuid,) = sqlx::query_as(
        r#"INSERT INTO users (google_id, email, display_name, avatar_url, role)
           VALUES ($1, $2, $3, $4, $5::user_role)
           ON CONFLICT (google_id) DO UPDATE SET
               email = EXCLUDED.email,
               display_name = EXCLUDED.display_name,
               avatar_url = EXCLUDED.avatar_url,
               updated_at = now()
           RETURNING id"#,
    )
    .bind(&userinfo.sub)
    .bind(&userinfo.email)
    .bind(&userinfo.name)
    .bind(&userinfo.picture)
    .bind(role)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // 6. Set session
    session
        .insert("user_id", user_id.0.to_string())
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::temporary("/"))
}
```

Key details:

- **Step 1 (CSRF validation):** We check that the `state` parameter from Google matches what we stored in the session. If it does not match, someone tampered with the request.
- **Step 5 (ON CONFLICT upsert):** If this user has logged in before, we update their profile instead of creating a duplicate. The `google_id` is the unique identifier.
- **Step 4 (First user becomes admin):** `default_role_for_new_user` checks `SELECT COUNT(*) FROM users`. If the table is empty, the first person gets admin powers. This bootstraps the system without needing a separate setup step.

### Step 5: Register the routes

In your `main.rs`, add the OAuth routes:

```rust
use crate::auth::oauth::{google_login, google_callback, logout, OAuthState};

let oauth_state = OAuthState {
    client: build_oauth_client(&config.oauth),
    pool: pool.clone(),
};

let app = Router::new()
    .route("/auth/google/login", get(google_login))
    .route("/auth/google/callback", get(google_callback))
    .route("/auth/logout", get(logout))
    .with_state(oauth_state)
    // ... other routes and layers
```

These are plain Axum routes, not Leptos server functions. OAuth requires server-side HTTP redirects (302 status codes), which server functions cannot do --- they always return JSON.

<details>
<summary>Hint: If the Google callback returns "BAD_REQUEST"</summary>

Check that:
1. The redirect URL in your Google Cloud Console matches exactly what `build_oauth_client` uses (including port and path)
2. The session store is working --- try logging `session.id()` in both handlers
3. Cookies are set to `SameSite::Lax`, not `Strict` --- the callback is a cross-site redirect from Google

</details>

---

## Exercise 4: Build the LoginPage Component

**Goal:** Build a login page with tabbed authentication methods: Phone OTP (primary), Email/Password, and Google OAuth.

### Step 1: The page structure

> **Programming Concept: What is Hashing?**
>
> Hashing turns a password into a scrambled string that cannot be reversed. Imagine a paper shredder: you feed in a document, and you get confetti. You can verify that two documents produce the same confetti pattern, but you cannot reconstruct the original document from the confetti.
>
> When you create an account with the password "hunter2", GrindIt does not store "hunter2" in the database. Instead, it feeds the password through a hashing algorithm (Argon2) and stores the resulting scrambled string: `$argon2id$v=19$m=19456,t=2,p=1$abc...$xyz...`
>
> When you log in, GrindIt hashes the password you type and compares it to the stored hash. If they match, you are in. If the database is ever leaked, attackers get the hashes but cannot reverse them back to passwords.
>
> The "salt" is a random string mixed in before hashing. It ensures that two users with the same password get different hashes. Without salt, an attacker could precompute hashes for common passwords (a "rainbow table") and look them up instantly.

`src/pages/login/mod.rs`:

```rust
use crate::auth::clean_error;
use crate::auth::otp::{SendOtp, VerifyOtp};
use crate::auth::password::{LoginWithPassword, RegisterWithPassword};
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    let login = ServerAction::<LoginWithPassword>::new();
    let register = ServerAction::<RegisterWithPassword>::new();
    let send_otp = ServerAction::<SendOtp>::new();
    let verify_otp = ServerAction::<VerifyOtp>::new();

    let (show_register, set_show_register) = signal(false);
    let toast: RwSignal<Option<ToastMsg>> = RwSignal::new(None);

    // OTP state
    let otp_phone = RwSignal::new(String::new());
    let otp_sent = RwSignal::new(false);
    let otp_code = RwSignal::new(String::new());

    // Which login method is active: "phone" or "email"
    let active_method = RwSignal::new("phone".to_string());

    let login_pending = login.pending();
    let register_pending = register.pending();
    let send_otp_pending = send_otp.pending();
    let verify_otp_pending = verify_otp.pending();

    let login_success = RwSignal::new(false);
    let register_success = RwSignal::new(false);
    let verify_success = RwSignal::new(false);

    // ... (effect handlers and view follow)
```

Four `ServerAction` types --- one for each server function. Each `ServerAction` provides reactive state: `.pending()` tells you if the request is in flight, `.value()` gives you the result when it arrives.

### Step 2: Effect handlers for action results

```rust
    // Handle email login result
    Effect::new(move |_| match login.value().get() {
        Some(Ok(_)) => {
            login_success.set(true);
            #[cfg(feature = "hydrate")]
            let _ = js_sys::eval("setTimeout(() => { window.location.href = '/'; }, 700)");
        }
        Some(Err(e)) => show_error(&e),
        None => {}
    });

    // Handle verify OTP result
    Effect::new(move |_| match verify_otp.value().get() {
        Some(Ok(result)) => {
            verify_success.set(true);
            #[cfg(feature = "hydrate")]
            {
                let url = if result == crate::auth::OtpResult::NewAccount {
                    "/profile"
                } else {
                    "/"
                };
                let _ = js_sys::eval(&format!(
                    "setTimeout(() => {{ window.location.href = '{}'; }}, 700)",
                    url
                ));
            }
        }
        Some(Err(e)) => show_error(&e),
        None => {}
    });
```

Notice the OTP result handler uses pattern matching on the `OtpResult` enum: new accounts go to `/profile` (to set their name), existing accounts go to `/`. The server returns structured data (not a string), and the client makes decisions based on the variant.

We use `window.location.href` instead of Leptos's `use_navigate()` for login redirects because we want a **full page reload**. The session cookie was just set by the server, and a full reload ensures the entire component tree re-evaluates with the new authentication state.

### Step 3: The Phone OTP tab

```rust
    <Show when=move || active_method.get() == "phone">
        <div class="auth-form">
            <Show when=move || !otp_sent.get()>
                <div class="phone-input-row">
                    <span class="country-code-label">"+91"</span>
                    <input
                        type="tel"
                        inputmode="numeric"
                        class="phone-number-input"
                        placeholder="98765 43210"
                        maxlength="10"
                        prop:value=move || otp_phone.get()
                        on:input=move |ev| {
                            let mut val = digits_only(&event_target_value(&ev));
                            val.truncate(10);
                            otp_phone.set(val);
                        }
                    />
                </div>
                <button
                    class="auth-submit"
                    disabled=move || send_otp_pending.get()
                    on:click=on_send_otp
                >"Send OTP"</button>
            </Show>

            <Show when=move || otp_sent.get()>
                <p class="otp-hint">
                    "Enter the 6-digit code sent to "
                    <strong>{move || format!("+91 {}", otp_phone.get())}</strong>
                </p>
                <input
                    type="text"
                    inputmode="numeric"
                    maxlength="6"
                    placeholder="000000"
                    class="otp-input"
                    prop:value=move || otp_code.get()
                    on:input=move |ev| {
                        let val = digits_only(&event_target_value(&ev));
                        otp_code.set(val);
                    }
                />
                <button
                    class="auth-submit"
                    class:auth-submit--success=move || verify_success.get()
                    disabled=move || verify_otp_pending.get() || verify_success.get()
                    on:click=on_verify_otp
                >
                    {move || if verify_success.get() {
                        "Signed in!"
                    } else if verify_otp_pending.get() {
                        "Verifying..."
                    } else {
                        "Verify & Sign In"
                    }}
                </button>
            </Show>
        </div>
    </Show>
```

The OTP flow is a two-step state machine: first enter your phone number, then enter the code. The `otp_sent` signal controls which step is visible. `<Show when=...>` conditionally renders its children --- when the condition is `true`, the children appear; when `false`, they disappear.

### Step 4: The Email/Password tab

```rust
    <Show when=move || active_method.get() == "email">
        <Show when=move || !show_register.get()>
            <div class="auth-form">
                <ActionForm action=login>
                    <input type="email" name="email" placeholder="Email" required />
                    <input type="password" name="password" placeholder="Password" required />
                    <button type="submit" class="auth-submit"
                        disabled=move || login_pending.get() || login_success.get()
                    >
                        {move || if login_success.get() {
                            "Signed in!"
                        } else if login_pending.get() {
                            "Signing in..."
                        } else {
                            "Sign in"
                        }}
                    </button>
                </ActionForm>
            </div>
            <p class="auth-switch">
                "No account? "
                <button class="auth-link"
                    on:click=move |_| set_show_register.set(true)
                >"Register"</button>
            </p>
        </Show>

        <Show when=move || show_register.get()>
            <div class="auth-form">
                <ActionForm action=register>
                    <input type="text" name="name" placeholder="Name" required />
                    <input type="email" name="email" placeholder="Email" required />
                    <input type="password" name="password"
                        placeholder="Password (min 8 chars)" required minlength="8" />
                    <button type="submit" class="auth-submit"
                        disabled=move || register_pending.get() || register_success.get()
                    >"Create account"</button>
                </ActionForm>
            </div>
        </Show>
    </Show>
```

`<ActionForm>` is a Leptos component that submits a form to a `ServerAction`. It reads form fields by their `name` attribute and passes them to the server function. This works with or without JavaScript --- if WASM has not loaded yet, the form submits as a traditional HTML POST. Once hydrated, it submits via a fetch call in the background.

### Step 5: The Google OAuth button

```rust
    <div class="auth-divider"><span>"or"</span></div>

    <a href="/auth/google/login" rel="external" class="google-btn">
        "Sign in with Google"
    </a>
```

The `rel="external"` attribute tells Leptos's client-side router to NOT intercept this link. We want a full page navigation to `/auth/google/login`, which is an Axum route that redirects to Google. Without `rel="external"`, the router would try to handle it as a client-side navigation and find no matching route.

### Step 6: The password server functions

`src/auth/password.rs`:

```rust
use leptos::prelude::*;

#[server]
pub async fn login_with_password(email: String, password: String) -> Result<(), ServerFnError> {
    use argon2::{Argon2, PasswordHash, PasswordVerifier};

    let pool = crate::db::db().await?;

    #[derive(sqlx::FromRow)]
    struct Row {
        id: uuid::Uuid,
        password_hash: Option<String>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT id, password_hash FROM users WHERE email = $1"
    )
    .bind(&email)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let row = row.ok_or_else(|| ServerFnError::new("Invalid email or password"))?;

    let hash = row.password_hash
        .ok_or_else(|| ServerFnError::new("Invalid email or password"))?;

    let parsed = PasswordHash::new(&hash)
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| ServerFnError::new("Invalid email or password"))?;

    let session = super::session::get_session().await?;
    super::session::set_user_id(&session, &row.id.to_string()).await?;

    Ok(())
}
```

Notice the deliberately vague error message: "Invalid email or password" --- the same message whether the email does not exist, the user has no password (Google-only account), or the password is wrong. This prevents attackers from discovering which email addresses have accounts.

### Step 7: Validation helpers

`src/auth/validation.rs`:

```rust
use leptos::prelude::ServerFnError;

pub fn validate_name(name: &str) -> Result<String, ServerFnError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(ServerFnError::new("Name is required"));
    }
    if name.len() > 100 {
        return Err(ServerFnError::new("Name is too long"));
    }
    Ok(name)
}

pub fn validate_email(email: &str) -> Result<Option<String>, ServerFnError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return Ok(None);
    }
    if !email.contains('@') || !email.contains('.') {
        return Err(ServerFnError::new("Invalid email address"));
    }
    Ok(Some(email))
}

pub fn validate_password(password: &str) -> Result<(), ServerFnError> {
    if password.len() < 8 {
        return Err(ServerFnError::new("Password must be at least 8 characters"));
    }
    if password.len() > 128 {
        return Err(ServerFnError::new("Password is too long"));
    }
    Ok(())
}

pub fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// First user becomes admin, everyone else is athlete.
pub async fn default_role_for_new_user(pool: &sqlx::PgPool) -> Result<&'static str, ServerFnError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(if count.0 == 0 { "admin" } else { "athlete" })
}
```

The `hash_password` function uses Argon2 --- the winner of the Password Hashing Competition. Argon2 is intentionally slow. While a regular hash function can compute billions of hashes per second, Argon2 deliberately takes about 100 milliseconds. This makes brute-force attacks impractical: trying all common passwords would take years instead of minutes.

<details>
<summary>Hint: If Argon2 hashing is very slow in debug builds</summary>

Add this to `Cargo.toml` to optimize Argon2 even in debug builds:

```toml
[profile.dev.package.argon2]
opt-level = 3
```

This makes password hashing take ~100ms instead of ~2s during development.

</details>

---

## Rust Gym

### Drill 1: Your First Enum

Define a `TrafficLight` enum with variants `Red`, `Yellow`, and `Green`. Add a method `can_go(&self) -> bool` that returns `true` only for `Green`.

```rust
// Define the enum and implement can_go()
```

<details>
<summary>Hint</summary>

Remember: `match` checks the value against each variant. Only one arm should return `true`.

</details>

<details>
<summary>Solution</summary>

```rust
enum TrafficLight {
    Red,
    Yellow,
    Green,
}

impl TrafficLight {
    fn can_go(&self) -> bool {
        match self {
            TrafficLight::Green => true,
            TrafficLight::Red => false,
            TrafficLight::Yellow => false,
        }
    }
}

// Or more concisely:
// matches!(self, TrafficLight::Green)
```

The `matches!` macro is a shortcut that returns `true` if the value matches the pattern. It is useful when you only care about one variant and want a boolean result.

</details>

### Drill 2: Match with Guards

Write a function `describe_access(role: &UserRole, is_owner: bool)` that returns:
- Admin: "Full access"
- Coach who is owner: "Can edit"
- Coach who is not owner: "Read only"
- Athlete: "View scores only"

```rust
fn describe_access(role: &UserRole, is_owner: bool) -> &'static str {
    // Your implementation
}
```

<details>
<summary>Hint</summary>

Put the guarded arm (Coach + is_owner) BEFORE the un-guarded Coach arm. Rust checks arms top to bottom.

</details>

<details>
<summary>Solution</summary>

```rust
fn describe_access(role: &UserRole, is_owner: bool) -> &'static str {
    match role {
        UserRole::Admin => "Full access",
        UserRole::Coach if is_owner => "Can edit",
        UserRole::Coach => "Read only",
        UserRole::Athlete => "View scores only",
    }
}
```

The match guard `if is_owner` only applies to the arm it is attached to. The second `Coach` arm catches all coaches where `is_owner` is false. Order matters --- Rust checks arms top to bottom.

</details>

### Drill 3: From\<String\> Conversion

Implement `From<String>` for `UserRole` so that `UserRole::from("coach".to_string())` returns `UserRole::Coach`, and unknown strings default to `UserRole::Athlete`.

```rust
impl From<String> for UserRole {
    fn from(s: String) -> Self {
        // Your implementation
    }
}
```

<details>
<summary>Hint</summary>

Use `.to_lowercase()` to handle case differences, then `match` on the result. The `_` pattern catches everything else.

</details>

<details>
<summary>Solution</summary>

```rust
impl From<String> for UserRole {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "admin" => UserRole::Admin,
            "coach" => UserRole::Coach,
            _ => UserRole::Athlete,
        }
    }
}

// Now you can write:
let role: UserRole = "coach".to_string().into();
// or:
let role = UserRole::from(db_role_string);
```

Implementing `From<String>` automatically gives you `Into<UserRole>` for `String` --- Rust provides the reverse direction for free. The `to_lowercase()` makes the conversion case-insensitive.

</details>

---

## DSA in Context: Hashing

Password storage is a real-world application of hash functions. Let us connect this to data structures you may have seen:

**HashMap:** You use `HashMap<String, String>` to look up a user by email --- O(1) average lookup. The hash function maps the email to a bucket index. Speed is the goal.

**Password hashing:** Argon2 maps a password to a fixed-length string. Unlike HashMap hashes (which aim for speed), password hashes aim for **slowness** --- making brute-force attacks impractical.

**Collision resistance:** A hash collision is when two different inputs produce the same output. For HashMaps, collisions degrade performance (O(n) worst case). For passwords, collisions would allow attackers to find a different string that matches. Argon2's collision resistance makes this computationally infeasible.

| Property | HashMap hash | Password hash (Argon2) |
|----------|-------------|----------------------|
| Speed | Fast (nanoseconds) | Slow (100ms+) |
| Deterministic | Yes | Yes (with same salt) |
| Collision resistance | Low priority | Critical |
| Reversible | No | No |
| Salt | Not used | Required |

---

## System Design Corner: Auth Architecture

### Sessions vs Tokens

| Aspect | Server sessions (GrindIt) | JWT tokens |
|--------|--------------------------|-----------|
| Storage | Server-side (PostgreSQL) | Client-side (cookie/localStorage) |
| Revocation | Delete the session row | Difficult --- must maintain a blocklist |
| Size | Small cookie (session ID) | Large cookie (encoded claims) |
| Stateless | No --- requires DB lookup | Yes --- self-contained |
| Best for | Web apps with SSR | APIs, microservices |

GrindIt uses server sessions because:
1. **Instant revocation** --- logging out is just `session.flush()`
2. **No secret leakage** --- the session data never leaves the server
3. **SSR compatibility** --- the session cookie is sent automatically with every request

### RBAC Hierarchy

Role-Based Access Control in GrindIt uses a simple numeric rank:

```
Admin (2) > Coach (1) > Athlete (0)
```

The `require_role(min_role)` check is: `user.role.rank() >= min_role.rank()`. This means:
- `require_role(Athlete)` --- any logged-in user passes
- `require_role(Coach)` --- coaches and admins pass
- `require_role(Admin)` --- only admins pass

---

## Design Insight: Information Hiding (Ousterhout)

The auth system is a perfect example of Ousterhout's principle from *A Philosophy of Software Design*: **the best modules are those whose interface is much simpler than their implementation.**

**The simple interface:** `require_auth()` returns `Result<AuthUser, ServerFnError>`. Call it at the top of any server function. That is the entire API.

**The hidden complexity:** Behind that one function call:
- Session extraction from the HTTP request
- Session store lookup in PostgreSQL
- UUID parsing and database query for the user record
- Error mapping from `sqlx::Error` to `ServerFnError`
- Role deserialization from the database enum type

A server function author does not need to know any of this. They write `let user = require_auth().await?;` and get an `AuthUser` or an error. The complexity is real --- but it is contained in one place.

---

## What You Built

In this chapter, you:

1. **Defined the `UserRole` enum** with `rank()`, `Display`, and data-carrying variants --- learning how Rust enums differ fundamentally from enums in other languages
2. **Built session management** with `tower-sessions` backed by PostgreSQL --- `get_session()`, `set_user_id()`, `get_current_user()`, `require_auth()`, `require_role()`
3. **Implemented Google OAuth2** with CSRF protection, token exchange, user upsert, and session creation
4. **Built the LoginPage** with tabbed Phone OTP and Email/Password forms, plus a Google sign-in button
5. **Practiced pattern matching** --- exhaustive `match`, match guards, `if let`, `let-else`, and nested `Option`/`Result` handling

GrindIt now has real authentication. Anonymous users can browse exercises and WODs. Logging workouts requires signing in. Programming WODs requires Coach or Admin role. The guard functions protect every server function with a single line of code.

In Chapter 8, we will build the WOD programming system --- where coaches create structured workouts with sections and movements, displayed on a weekly calendar.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a HashMap from scratch — hash function, buckets, collision chaining, and rehashing from scratch in Rust — no libraries, just std.

**→ [HashMap Auth](../ds-narratives/ch07-hashmap-auth.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/auth/mod.rs` | [`src/auth/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/auth/mod.rs) --- `UserRole`, `AuthUser`, `clean_error()`, `get_me()` |
| `src/auth/session.rs` | [`src/auth/session.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/auth/session.rs) --- `get_session`, `get_current_user`, `require_auth`, `require_role` |
| `src/auth/oauth.rs` | [`src/auth/oauth.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/auth/oauth.rs) --- `google_login`, `google_callback`, `OAuthState` |
| `src/auth/password.rs` | [`src/auth/password.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/auth/password.rs) --- `login_with_password`, `register_with_password` |
| `src/auth/otp.rs` | [`src/auth/otp.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/auth/otp.rs) --- `send_otp`, `verify_otp` |
| `src/auth/validation.rs` | [`src/auth/validation.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/auth/validation.rs) --- `validate_name`, `validate_email`, `hash_password`, `default_role_for_new_user` |
| `src/pages/login/mod.rs` | [`src/pages/login/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/login/mod.rs) --- `LoginPage` with tabbed methods |
