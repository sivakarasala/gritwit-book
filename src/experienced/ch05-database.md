# Chapter 5: Database Persistence

Reload the page and your exercises vanish. Every create, edit, and delete from Chapter 4 lived in a `RwSignal<Vec<Exercise>>` — memory that evaporates the instant the browser refreshes. This chapter makes it permanent. You will connect GrindIt to PostgreSQL, write SQL migrations, and build the async pipeline that moves data between your Leptos components and the database.

The spotlight concept is **async/await** — how Rust handles operations that take time (database queries, HTTP requests, file I/O) without blocking the thread. This is the same idea as JavaScript's `async/await`, but the mechanics are fundamentally different.

By the end of this chapter, you will have:

- PostgreSQL running in Docker with a proper initialization script
- SQL migrations that create the `exercises` table
- A global connection pool using `OnceLock<PgPool>`
- `#[server]` functions that bridge client and server
- Exercises that survive page reloads

---

## Spotlight: Async/Await & SQLx

### Why async?

A database query takes milliseconds. That is an eternity for a CPU that executes billions of instructions per second. If you wait synchronously, the thread sits idle — it cannot serve other requests while the database does its work.

Async code solves this by *yielding* the thread during the wait. The runtime can use that thread to handle other requests, then resume your code when the database responds.

```rust
// Synchronous — blocks the thread
fn list_exercises_sync(pool: &PgPool) -> Vec<Exercise> {
    // Thread sits idle while PostgreSQL runs the query
    sqlx::query_as("SELECT * FROM exercises").fetch_all(pool)
}

// Asynchronous — yields the thread
async fn list_exercises(pool: &PgPool) -> Vec<Exercise> {
    // Thread is free to handle other requests during the query
    sqlx::query_as("SELECT * FROM exercises").fetch_all(pool).await
}
```

The `async` keyword on a function means it returns a `Future` — a value that represents work that has not happened yet. The `.await` keyword yields the current task until the Future completes. Nothing runs until `.await` is called — Rust's futures are *lazy*.

### Futures vs Promises

If you know JavaScript `async/await`, the syntax looks identical. The semantics are not.

```javascript
// JavaScript — the Promise starts immediately
const promise = fetch("/api/exercises"); // network request fires NOW
const data = await promise;              // just waits for completion
```

```rust
// Rust — the Future does nothing until awaited
let future = fetch_exercises();    // NO network request yet — just a Future
let data = future.await;           // NOW the request fires AND we wait
```

This laziness is deliberate. It means you can build up complex Future chains without accidentally triggering side effects. It also means the Rust compiler can optimize the state machine that represents the async operation.

### The Tokio runtime

Rust does not have a built-in async runtime. You choose one. The overwhelming standard is **Tokio**, which provides:

- A multi-threaded task scheduler
- Async I/O (TCP, UDP, file system)
- Timers, channels, synchronization primitives

In `main.rs`, the `#[tokio::main]` attribute sets up the runtime:

```rust
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    // Everything in here runs on the Tokio runtime
    // .await works here because Tokio is driving the futures
}
```

Without a runtime, `.await` would compile but never complete — there would be nothing to poll the future.

### SQLx: compile-time verified queries

SQLx is a Rust database library with a unique superpower: it checks your SQL queries against the database schema at compile time. A typo in a column name or a type mismatch between Rust and PostgreSQL produces a compile error, not a runtime crash.

```rust
// This query is checked at compile time against the actual database schema:
let exercises = sqlx::query_as::<_, Exercise>(
    "SELECT id::text, name, category, scoring_type FROM exercises WHERE deleted_at IS NULL"
)
.fetch_all(&pool)
.await?;
```

If the `exercises` table does not have a `category` column, or if `name` is `INTEGER` instead of `TEXT`, the compiler catches it. This is one of Rust's strongest selling points for web development — entire classes of "wrong SQL" bugs become impossible.

### The `FromRow` derive

SQLx needs to know how to convert database rows into Rust structs. The `#[derive(sqlx::FromRow)]` macro generates this conversion automatically, matching struct fields to column names:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub category: String,
    pub scoring_type: String,
    pub created_by: Option<String>,
}
```

The `#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]` line is important. It says: "only derive `FromRow` when compiling for the server." The WASM client does not have a database connection and does not need (or even have access to) `sqlx::FromRow`. Without this gate, the WASM build would fail trying to find SQLx.

> **Coming from JS?**
>
> | Concept | JavaScript | Rust |
> |---------|-----------|------|
> | Async function | `async function f() {}` | `async fn f() {}` |
> | Await a result | `const x = await f();` | `let x = f().await;` |
> | When work starts | Immediately on call | Only when `.await`ed (lazy) |
> | Runtime | Built into V8/Node.js | External: Tokio (you add it) |
> | Query library | Prisma, Drizzle, raw `pg` | SQLx (compile-time checked) |
> | ORM mapping | Prisma schema, TypeORM decorators | `#[derive(FromRow)]` |
>
> The biggest difference: JavaScript Promises are eager (they start executing immediately). Rust Futures are lazy (nothing happens until polled). This means `let x = async_fn()` in Rust creates a future but does NOT start the operation. You must write `let x = async_fn().await` to actually run it.

---

## Exercise 1: Set Up PostgreSQL with Docker

**Goal:** Get a PostgreSQL database running locally, create the application user and database, and run the first migration.

### Step 1: Create the initialization script

Create `scripts/init_db.sh`:

```bash
#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v sqlx)" ]; then
  echo >&2 "Error: sqlx is not installed."
  echo >&2 "Use:"
  echo >&2 "    cargo install --version='~0.8' sqlx-cli \
--no-default-features --features rustls,postgres"
  echo >&2 "to install it."
  exit 1
fi

DB_PORT="${DB_PORT:=5432}"
SUPERUSER="${SUPERUSER:=postgres}"
SUPERUSER_PWD="${SUPERUSER_PWD:=password}"
APP_USER="${APP_USER:=app}"
APP_USER_PWD="${APP_USER_PWD:=secret}"
APP_DB_NAME="${APP_DB_NAME:=gritwit}"

if [[ -z "${SKIP_DOCKER}" ]]; then
  RUNNING_POSTGRES_CONTAINER=$(docker ps --filter 'name=postgres' --format '{{.ID}}')
  if [[ -n $RUNNING_POSTGRES_CONTAINER ]]; then
    echo >&2 "there is a postgres container already running, kill it with"
    echo >&2 "    docker kill ${RUNNING_POSTGRES_CONTAINER}"
    exit 1
  fi

  CONTAINER_NAME="postgres_$(date '+%s')"
  docker run \
      --env POSTGRES_USER=${SUPERUSER} \
      --env POSTGRES_PASSWORD=${SUPERUSER_PWD} \
      --health-cmd="pg_isready -U ${SUPERUSER} || exit 1" \
      --health-interval=1s \
      --health-timeout=5s \
      --health-retries=5 \
      --publish "${DB_PORT}":5432 \
      --detach \
      --name "${CONTAINER_NAME}" \
      postgres -N 1000

  until [ \
    "$(docker inspect -f "{{.State.Health.Status}}" ${CONTAINER_NAME})" == \
    "healthy" \
  ]; do
    >&2 echo "Postgres is still unavailable - sleeping"
    sleep 1
  done

  CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
  docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${CREATE_QUERY}"

  GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
  docker exec -it "${CONTAINER_NAME}" psql -U "${SUPERUSER}" -c "${GRANT_QUERY}"
fi

>&2 echo "Postgres is up and running on port ${DB_PORT} - running migrations now!"

DATABASE_URL=postgres://${APP_USER}:${APP_USER_PWD}@localhost:${DB_PORT}/${APP_DB_NAME}
export DATABASE_URL
sqlx database create
sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"
```

Make it executable:

```bash
chmod +x scripts/init_db.sh
```

This script does three things:
1. Launches PostgreSQL in a Docker container
2. Creates an application user with limited privileges
3. Runs SQLx migrations to create the schema

### Step 2: Install the SQLx CLI

```bash
cargo install --version='~0.8' sqlx-cli \
    --no-default-features --features rustls,postgres
```

### Step 3: Create the migration

```bash
mkdir -p migrations
```

Create `migrations/20260101120000_create_exercises_table.sql`:

```sql
CREATE TABLE exercises (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    movement_type TEXT,
    muscle_groups TEXT[] DEFAULT '{}',
    description TEXT,
    demo_video_url TEXT,
    scoring_type TEXT NOT NULL DEFAULT 'weight_and_reps',
    created_by UUID,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_exercises_name
    ON exercises (LOWER(name))
    WHERE deleted_at IS NULL;
```

A few things to note about this schema:

- **`UUID PRIMARY KEY DEFAULT gen_random_uuid()`** — PostgreSQL generates a random UUID for each new row. No auto-incrementing integer that leaks insertion order.
- **`TEXT[] DEFAULT '{}'`** — PostgreSQL supports array columns natively. `muscle_groups` stores a list of strings in a single column.
- **`deleted_at TIMESTAMPTZ`** — the soft delete column from Chapter 4, now in the database. `NULL` means active, non-null means deleted.
- **`CREATE UNIQUE INDEX ... WHERE deleted_at IS NULL`** — a partial unique index. Two exercises can have the same name only if one of them is deleted. This is a common pattern for soft-delete systems.

### Step 4: Run the script

```bash
./scripts/init_db.sh
```

If everything works, you will see:

```
Postgres has been migrated, ready to go!
```

### Step 5: Set the database URL

Create a `.env` file in your project root (add `.env` to `.gitignore`):

```
DATABASE_URL=postgres://app:secret@localhost:5432/gritwit
```

SQLx reads this at compile time (for query verification) and at runtime (for connecting).

<details>
<summary>Hint: If the Docker container fails to start</summary>

The most common issue is port 5432 already being in use (perhaps by a local PostgreSQL installation). Either stop the local PostgreSQL service or use a different port:

```bash
DB_PORT=5433 ./scripts/init_db.sh
```

Then update your `.env` to use port 5433.

</details>

---

## Exercise 2: Add `FromRow` and Write Database Functions

**Goal:** Update the Exercise struct with `FromRow` and write async functions to list and create exercises.

### Step 1: Add dependencies

Add these to `Cargo.toml` under `[dependencies]`:

```toml
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid"], optional = true }
uuid = { version = "1", features = ["v4"] }
tokio = { version = "1", features = ["full"], optional = true }
serde = { version = "1", features = ["derive"] }
```

And update the `ssr` feature:

```toml
[features]
ssr = ["leptos/ssr", "leptos_meta/ssr", "dep:sqlx", "dep:tokio"]
hydrate = ["leptos/hydrate", "dep:console_error_panic_hook", "dep:wasm-bindgen"]
```

### Step 2: Create the database module

Create `src/db.rs`:

```rust
use serde::{Deserialize, Serialize};

// ---- Models ----

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub category: String,
    pub scoring_type: String,
    pub created_by: Option<String>,
}

// ---- Database functions (server-only) ----

#[cfg(feature = "ssr")]
pub async fn list_exercises_db(
    pool: &sqlx::PgPool,
) -> Result<Vec<Exercise>, sqlx::Error> {
    sqlx::query_as::<_, Exercise>(
        r#"SELECT
            id::text, name, category, scoring_type, created_by::text
        FROM exercises
        WHERE deleted_at IS NULL
        ORDER BY name"#,
    )
    .fetch_all(pool)
    .await
}

#[cfg(feature = "ssr")]
pub async fn create_exercise_db(
    pool: &sqlx::PgPool,
    name: &str,
    category: &str,
    scoring_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO exercises (name, category, scoring_type)
        VALUES ($1, $2, $3)"#,
    )
    .bind(name)
    .bind(category)
    .bind(scoring_type)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn delete_exercise_db(
    pool: &sqlx::PgPool,
    id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE exercises SET deleted_at = NOW() WHERE id = $1"
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
```

### Step 3: Understand the patterns

**`sqlx::query_as::<_, Exercise>`** — runs a SQL query and maps each row to the `Exercise` struct. The `_` lets Rust infer the database backend (PostgreSQL). Each struct field must match a column name in the query result. We cast `id::text` and `created_by::text` because the database stores UUIDs, but our struct uses `String`.

**`.bind(name)`** — binds a Rust value to a `$1`, `$2`, ... placeholder. This is a parameterized query — it prevents SQL injection. Never use `format!("SELECT ... WHERE name = '{}'", name)` — that is how injection attacks happen.

**`.fetch_all(pool).await`** — runs the query and collects all rows into a `Vec<Exercise>`. The `.await` yields the thread while PostgreSQL processes the query.

**`#[cfg(feature = "ssr")]`** — these functions only exist on the server. The WASM client never calls the database directly — it calls server functions, which we will create in Exercise 4.

**The `?` operator with `sqlx::Error`** — `.execute(pool).await?` returns `Result<PgQueryResult, sqlx::Error>`. The `?` unwraps the success case or propagates the error. Every database operation uses this pattern.

<details>
<summary>Hint: If you see "the trait FromRow is not implemented"</summary>

Make sure `sqlx` is in your dependencies with the `postgres` feature, and that it is gated behind the `ssr` feature. Also verify that the `#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]` line is above the struct, not inside it. The derive macro must see all the fields at once.

</details>

---

## Exercise 3: Create the Global Pool with `OnceLock<PgPool>`

**Goal:** Set up a global database connection pool that is initialized once in `main()` and accessible from any server function.

### Step 1: Understand connection pools

A database connection is expensive to create — it involves a TCP handshake, authentication, and protocol negotiation. Creating a new connection for every query would be slow.

A **connection pool** maintains a set of open connections. When your code needs to query the database, it borrows a connection from the pool. When done, it returns the connection (not closes it). The next query reuses the same connection.

```
┌───────────────────────┐
│   Your Application    │
│                       │
│   task 1 ──borrow──>  │
│   task 2 ──borrow──>  │     ┌──────────────────┐
│   task 3 ──wait───>   │◄───►│  Connection Pool  │
│                       │     │  [conn1] [conn2]  │──► PostgreSQL
│   task 1 ──return──>  │     │  [conn3] [conn4]  │
│   task 3 ──borrow──>  │     └──────────────────┘
└───────────────────────┘
```

SQLx's `PgPool` implements this. You create it once and share it everywhere.

### Step 2: Add the global pool

Add this to the top of `src/db.rs`:

```rust
#[cfg(feature = "ssr")]
static POOL: std::sync::OnceLock<sqlx::PgPool> = std::sync::OnceLock::new();

/// Call once from main() to make the pool globally available.
#[cfg(feature = "ssr")]
pub fn init_pool(pool: sqlx::PgPool) {
    POOL.set(pool).expect("Pool already initialized");
}

/// Get a handle to the database pool.
#[cfg(feature = "ssr")]
pub async fn db() -> Result<sqlx::PgPool, leptos::prelude::ServerFnError> {
    POOL.get()
        .cloned()
        .ok_or_else(|| leptos::prelude::ServerFnError::new("Database pool not initialized"))
}
```

**`OnceLock`** is a synchronization primitive from the standard library. It holds a value that can be set exactly once and then read by any thread. It is the idiomatic way to store global state in Rust when the value is set at startup and never changes.

- `POOL.set(pool)` — sets the value. Returns `Err` if already set (which our `.expect()` turns into a panic — appropriate because double-initialization is a programmer error, not a runtime condition).
- `POOL.get()` — returns `Option<&PgPool>`. `None` if not yet set.
- `.cloned()` — `PgPool` implements `Clone` cheaply (it is internally reference-counted). Cloning a pool does not create new connections — it just increments a counter.

### Step 3: Initialize in `main.rs`

Update `src/main.rs`:

```rust
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use gritwit::app::*;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use sqlx::postgres::PgPoolOptions;

    dotenvy::dotenv().ok();

    // Create the connection pool
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Store in the global OnceLock
    gritwit::db::init_pool(pool.clone());

    // Leptos setup
    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);

    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    println!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
```

The startup sequence is:
1. Load environment variables (`.env` file)
2. Create the connection pool (`PgPoolOptions::new()...`)
3. Run migrations (`sqlx::migrate!()`)
4. Store the pool globally (`init_pool(pool)`)
5. Start the web server

**`sqlx::migrate!()`** is a macro that embeds your migration SQL files into the binary at compile time. At runtime, it checks which migrations have already been applied and runs any new ones. This means deploying a new version of your app automatically updates the database schema.

**`PgPoolOptions::new().max_connections(5)`** — limits the pool to 5 simultaneous connections. For a development server, this is plenty. In production, you would tune this based on your PostgreSQL `max_connections` setting and the number of application instances.

### Step 4: Add new dependencies

Update `Cargo.toml`:

```toml
[dependencies]
dotenvy = "0.15"
# ... existing dependencies
```

<details>
<summary>Hint: If sqlx::migrate!() fails with "error communicating with database"</summary>

The `sqlx::migrate!()` macro connects to the database at compile time to verify the migration files. Make sure:
1. PostgreSQL is running (`docker ps` to check)
2. The `DATABASE_URL` environment variable is set (either in `.env` or exported in your shell)
3. The database exists (`sqlx database create`)

If you want to skip compile-time verification temporarily, you can use `sqlx::migrate!().run(&pool)` with the `SQLX_OFFLINE=true` environment variable, but you lose the safety guarantee.

</details>

---

## Exercise 4: Write Server Functions and Connect to the UI

**Goal:** Create `#[server]` functions that call the database and connect them to the exercises page using `Resource::new`.

### Step 1: Understand server functions

A server function is a regular async function annotated with `#[server]`. The macro does something remarkable: it generates *two different versions* of the function depending on the compilation target.

```rust
#[server]
pub async fn list_exercises() -> Result<Vec<Exercise>, ServerFnError> {
    // This code only runs on the server
    let pool = crate::db::db().await?;
    crate::db::list_exercises_db(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

When compiled with `feature = "ssr"` (server):
- The function body runs as-is on the server
- It has access to the database, file system, secrets — everything

When compiled with `feature = "hydrate"` (WASM client):
- The function body is replaced with an HTTP POST request to the server
- The arguments are serialized and sent as the request body
- The return value is deserialized from the response

The caller does not know the difference. Client-side code calls `list_exercises().await` and gets back a `Vec<Exercise>`, whether it came from a direct database query or an HTTP round trip.

### Step 2: Create the server functions

Add these to `src/app.rs` (or create a separate `src/server_fns.rs`):

```rust
use crate::db::Exercise;
use leptos::prelude::*;

#[server]
pub async fn list_exercises() -> Result<Vec<Exercise>, ServerFnError> {
    let pool = crate::db::db().await?;
    crate::db::list_exercises_db(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn create_exercise(
    name: String,
    category: String,
    scoring_type: String,
) -> Result<(), ServerFnError> {
    if name.trim().is_empty() {
        return Err(ServerFnError::new("Name cannot be empty"));
    }
    let pool = crate::db::db().await?;
    crate::db::create_exercise_db(&pool, &name, &category, &scoring_type)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn delete_exercise(id: String) -> Result<(), ServerFnError> {
    let pool = crate::db::db().await?;
    let uuid: uuid::Uuid = id
        .parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
    crate::db::delete_exercise_db(&pool, uuid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

Notice the `?` and `map_err` patterns from Chapter 4 appearing everywhere:
- `crate::db::db().await?` — get the pool or return a `ServerFnError`
- `.map_err(|e| ServerFnError::new(e.to_string()))` — convert `sqlx::Error` into `ServerFnError`
- `id.parse().map_err(...)` — convert a string to a UUID, handling the parse error

### Step 3: Connect to the UI with `Resource::new`

A `Resource` in Leptos is a reactive wrapper around an async operation. It fetches data from the server and integrates with the signal system:

```rust
#[component]
fn ExercisesPage() -> impl IntoView {
    let create_action = ServerAction::<CreateExercise>::new();
    let delete_action = ServerAction::<DeleteExercise>::new();

    // Resource re-fetches whenever an action completes
    let exercises = Resource::new(
        move || (
            create_action.version().get(),
            delete_action.version().get(),
        ),
        |_| list_exercises(),
    );

    let search = RwSignal::new(String::new());

    view! {
        <div class="exercises-page">
            // ... FAB button and form (now dispatching create_action) ...

            <div class="exercises-search">
                <input
                    type="text"
                    class="exercises-search-input"
                    placeholder="Search exercises..."
                    prop:value=move || search.get()
                    on:input=move |ev| search.set(event_target_value(&ev))
                />
            </div>

            <Suspense fallback=|| view! { <p class="loading">"Loading exercises..."</p> }>
                {move || {
                    let q = search.get().to_lowercase();
                    exercises.get().map(|result| {
                        match result {
                            Ok(list) => {
                                let filtered: Vec<Exercise> = list
                                    .into_iter()
                                    .filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q))
                                    .collect();

                                if filtered.is_empty() {
                                    view! {
                                        <p class="empty-state">"No exercises found."</p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="exercises-list">
                                            {filtered.into_iter().map(|ex| {
                                                let id = ex.id.clone();
                                                view! {
                                                    <div class="exercise-card">
                                                        <div class="exercise-card__name">{ex.name}</div>
                                                        <div class="exercise-card__meta">
                                                            <span class="exercise-card__category">{&ex.category}</span>
                                                            <span class="exercise-card__scoring">{ex.scoring_type}</span>
                                                        </div>
                                                        <button
                                                            class="exercise-delete"
                                                            on:click=move |_| {
                                                                delete_action.dispatch(DeleteExercise {
                                                                    id: id.clone(),
                                                                });
                                                            }
                                                        >"Delete"</button>
                                                    </div>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any()
                                }
                            }
                            Err(e) => {
                                view! {
                                    <p class="error">{format!("Error: {}", e)}</p>
                                }.into_any()
                            }
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
```

### Step 4: Understand the reactive data flow

The data flow is:

```
User creates exercise
  -> create_action.dispatch(CreateExercise { ... })
    -> HTTP POST to server
      -> create_exercise() runs on server
        -> create_exercise_db() inserts into PostgreSQL
          -> returns Ok(())
    -> create_action.version() increments
      -> Resource dependency changes
        -> Resource re-fetches: list_exercises()
          -> list_exercises_db() queries PostgreSQL
            -> returns Vec<Exercise> (now includes the new one)
              -> UI re-renders with the updated list
```

**`ServerAction::<CreateExercise>::new()`** creates an action tied to a server function. When you call `.dispatch()`, it serializes the arguments, sends them to the server, and tracks the pending state.

**`.version().get()`** returns a counter that increments each time the action completes. By including this in the `Resource`'s dependency closure, the resource automatically re-fetches when any action finishes.

**`<Suspense>`** shows a loading fallback while the resource is fetching. On first load, the server renders the exercises directly (SSR). On subsequent fetches (after creates/deletes), the suspense boundary shows the existing content until the new data arrives.

### Step 5: Update the create form

Replace the in-memory `Callback` approach from Chapter 4 with a server action dispatch:

```rust
let create_action = ServerAction::<CreateExercise>::new();

// In the form's on_submit handler:
let on_submit = move |ev: leptos::ev::SubmitEvent| {
    ev.prevent_default();
    let name = name_input.get_untracked();
    if name.trim().is_empty() { return; }

    create_action.dispatch(CreateExercise {
        name,
        category: category_input.get_untracked(),
        scoring_type: scoring_input.get_untracked(),
    });
    show_form.set(false);
};
```

Save everything and test. Create an exercise, reload the page — it persists. Delete an exercise, reload — it is gone (soft-deleted). The data lives in PostgreSQL now.

<details>
<summary>Hint: If you see "ServerFnError: Database pool not initialized"</summary>

This means `init_pool()` was not called before the first server function ran. Check that your `main.rs` calls `gritwit::db::init_pool(pool.clone())` before starting the Axum server. Also verify that the `DATABASE_URL` is correct and PostgreSQL is running.

</details>

---

## Rust Gym

### Drill 1: Async Function Basics

What does this code print, and in what order? Predict first, then verify.

```rust
async fn fetch_name() -> String {
    println!("  fetching name...");
    "Back Squat".to_string()
}

async fn fetch_category() -> String {
    println!("  fetching category...");
    "weightlifting".to_string()
}

#[tokio::main]
async fn main() {
    println!("1. before creating futures");
    let name_future = fetch_name();
    let category_future = fetch_category();
    println!("2. futures created, not yet awaited");
    let name = name_future.await;
    println!("3. name fetched: {}", name);
    let category = category_future.await;
    println!("4. category fetched: {}", category);
}
```

<details>
<summary>Solution</summary>

```
1. before creating futures
2. futures created, not yet awaited
  fetching name...
3. name fetched: Back Squat
  fetching category...
4. category fetched: weightlifting
```

Key insight: "fetching name..." does NOT print at step 1 when `fetch_name()` is called. It prints at step 3 when `.await` is called. This proves that Rust futures are lazy — creating a future does not execute it.

If you wanted both to run concurrently, you would use `tokio::join!`:

```rust
let (name, category) = tokio::join!(fetch_name(), fetch_category());
```

This starts both futures and waits for both to complete, allowing them to run concurrently on the same thread.

</details>

### Drill 2: Error Handling in Async

This function uses `.unwrap()` in three places. Replace each with proper error handling using `?` and `map_err`:

```rust
async fn get_exercise_name(pool: &PgPool, id_str: &str) -> String {
    let uuid: Uuid = id_str.parse().unwrap();
    let row = sqlx::query_as::<_, Exercise>(
        "SELECT id::text, name, category, scoring_type, created_by::text
         FROM exercises WHERE id = $1"
    )
    .bind(uuid)
    .fetch_one(pool)
    .await
    .unwrap();
    row.name
}
```

<details>
<summary>Solution</summary>

```rust
async fn get_exercise_name(
    pool: &PgPool,
    id_str: &str,
) -> Result<String, ServerFnError> {
    let uuid: Uuid = id_str
        .parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    let row = sqlx::query_as::<_, Exercise>(
        "SELECT id::text, name, category, scoring_type, created_by::text
         FROM exercises WHERE id = $1"
    )
    .bind(uuid)
    .fetch_one(pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(row.name)
}
```

Three `.unwrap()` calls became three `map_err(...)?` calls. The return type changed from `String` to `Result<String, ServerFnError>`. The function can now fail gracefully instead of panicking.

</details>

### Drill 3: `.await` Chaining

Rewrite this function to use method chaining with `.await` and `?` instead of separate variable bindings:

```rust
async fn exercise_exists(pool: &PgPool, name: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "SELECT 1 FROM exercises WHERE LOWER(name) = LOWER($1) AND deleted_at IS NULL"
    )
    .bind(name)
    .fetch_optional(pool)
    .await;

    match result {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(e),
    }
}
```

<details>
<summary>Solution</summary>

```rust
async fn exercise_exists(pool: &PgPool, name: &str) -> Result<bool, sqlx::Error> {
    sqlx::query(
        "SELECT 1 FROM exercises WHERE LOWER(name) = LOWER($1) AND deleted_at IS NULL"
    )
    .bind(name)
    .fetch_optional(pool)
    .await
    .map(|row| row.is_some())
}
```

The entire `match` block collapses into `.map(|row| row.is_some())`. The `?` is not even needed here because `.map()` transforms the `Ok` value while preserving the `Err`. `fetch_optional` returns `Result<Option<Row>, Error>`, and `.map(|row| row.is_some())` converts it to `Result<bool, Error>`.

</details>

---

## DSA in Context: B-Tree Indexes

When you wrote `CREATE UNIQUE INDEX idx_exercises_name ON exercises (LOWER(name))`, PostgreSQL created a **B-tree index**. Understanding B-trees helps you write efficient queries and explain performance in interviews.

A B-tree is a self-balancing tree where each node can have many children (not just two like a binary tree). For a database index:

```
                    [M]
                /         \
          [D, H]           [R, W]
         /  |  \          /  |  \
      [A,B] [E,F] [I,K]  [N,P] [S,T] [X,Z]
```

**Lookup:** To find "Pull-ups", start at root [M], go right (P > M), go to [R, W], go left (P < R), find [N, P] — found. This is O(log n) — the same as a binary tree, but with much better cache locality because each node holds many keys.

**Without an index:** PostgreSQL must scan every row in the table — O(n). For 14 exercises, this is fine. For 14,000 exercises, it matters.

**When to index:** Index columns that appear in `WHERE`, `ORDER BY`, `JOIN ON`, and `UNIQUE` constraints. Our `LOWER(name)` index makes case-insensitive name lookups and uniqueness checks fast.

**The cost:** Indexes speed up reads but slow down writes (every INSERT/UPDATE must also update the index). For our app, this is a good trade-off — we read the exercise list far more often than we create new exercises.

---

## System Design Corner: Connection Pooling

Connection pooling is a common system design interview topic.

**Why pools?** A TCP connection to PostgreSQL costs ~1-5ms to establish (DNS + TCP handshake + TLS + authentication). If each request creates and destroys a connection, you add that overhead to every query. A pool keeps connections alive and reuses them.

**Pool sizing:** The PostgreSQL wiki recommends: `pool_size = (core_count * 2) + effective_spindle_count`. For a typical cloud VM with 2 cores and SSD storage, that is about 5-10 connections. Too few connections and requests queue up. Too many and PostgreSQL spends more time context-switching between connections than doing useful work.

**Lazy vs eager initialization:** `PgPoolOptions::new().connect(...)` establishes connections eagerly (connects immediately). `connect_lazy(...)` defers connection creation until the first query. GrindIt's reference implementation uses `connect_lazy_with(...)` — the app starts instantly and connections are created on demand.

```
Eager:   App starts -> creates 5 connections -> first request uses existing connection
Lazy:    App starts -> first request -> creates connection -> uses it
```

Lazy is better for development (fast startup) and serverless (pay only for what you use). Eager is better for production (first request is fast, and startup failures are detected early).

> **Interview talking point:** *"We use SQLx's PgPool with a max of 10 connections, stored in a global OnceLock. Server functions call db().await? to borrow a connection from the pool. We use lazy initialization in development for fast startup, and eager in production so we detect connection failures at deploy time rather than on the first user request."*

---

## Design Insight: Pull Complexity Downward

Ousterhout's principle: *"It is more important for a module to have a simple interface than a simple implementation."* He calls this "pulling complexity downward" — the module author absorbs complexity so that callers do not have to.

Look at the `db()` function:

```rust
pub async fn db() -> Result<sqlx::PgPool, ServerFnError> {
    POOL.get()
        .cloned()
        .ok_or_else(|| ServerFnError::new("Database pool not initialized"))
}
```

From the caller's perspective:

```rust
let pool = db().await?;
```

One line. The caller does not know about `OnceLock`, does not worry about whether the pool is initialized, does not manage connection state. All of that complexity is pulled down into the `db()` function and the `init_pool()` call in `main.rs`.

Compare this to the alternative where every server function manages its own connection:

```rust
// Bad: complexity pushed up to every caller
#[server]
pub async fn list_exercises() -> Result<Vec<Exercise>, ServerFnError> {
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| ServerFnError::new("DATABASE_URL not set"))?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    // ... finally query ...
}
```

This is "pushing complexity upward" — every caller repeats the same connection logic. A change to the connection strategy (switching from `connect` to `connect_lazy`, adding TLS, changing the pool size) would require updating every server function.

The `db()` function is a **deep module** — a simple interface hiding significant implementation. The best code in any codebase consists of deep modules.

---

## What You Built

In this chapter, you:

1. **Set up PostgreSQL with Docker** — init script, Docker container, application user, database creation
2. **Wrote SQL migrations** — `CREATE TABLE exercises` with UUID primary key, array columns, partial unique index, soft delete column
3. **Built async database functions** — `list_exercises_db`, `create_exercise_db`, `delete_exercise_db` with `sqlx::query_as` and parameterized queries
4. **Created a global pool** — `OnceLock<PgPool>` with `init_pool()` and `db()` accessor
5. **Connected client to server** — `#[server]` functions, `ServerAction`, `Resource::new`, `<Suspense>` for loading states
6. **Practiced async/await** — lazy futures, `.await` chaining, Tokio runtime, error handling in async contexts

Your exercises now persist in PostgreSQL. Create one, reload the page, and it is still there. In Chapter 6, we will add multi-page routing — the Exercises, Home, History, and Log pages each get their own URL, with a bottom nav that highlights the active tab.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Binary Search Tree that organizes exercises as you insert them — and teaches you about Box and ownership.

**→ [BST Exercise Lookup](../ds-narratives/ch05-bst-exercise-lookup.md)**

You've been writing `.await` all chapter — now find out what it actually does under the hood. Build a Future from scratch, implement a mini executor, and see the state machine the compiler generates.

**→ [Async & Futures Deep Dive — "The Front Desk Manager"](../ds-narratives/ch05-async-futures.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `scripts/init_db.sh` | [`scripts/init_db.sh`](https://github.com/sivakarasala/gritwit/blob/main/scripts/init_db.sh) |
| `migrations/..._create_exercises_table.sql` | [`migrations/20260311120000_create_exercises_table.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120000_create_exercises_table.sql) |
| `src/db.rs` (models + queries) | [`src/db.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/db.rs) |
| `src/main.rs` (pool init) | [`src/main.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/main.rs) |
| Server functions | [`src/pages/exercises/server_fns.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/server_fns.rs) |
| Resource + Suspense in UI | [`src/pages/exercises/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/mod.rs) |
