# Chapter 5: Database Persistence

Reload the page and your exercises vanish. Every create, edit, and delete from Chapter 4 lived in a `RwSignal<Vec<Exercise>>` --- memory that evaporates the instant the browser refreshes. It is like writing a shopping list on a foggy window --- the moment you close it, everything is gone.

This chapter makes your data permanent. You will connect GrindIt to PostgreSQL, a real database, and write the code that saves exercises there. After this chapter, you can create an exercise, close your browser, come back tomorrow, and it will still be there.

By the end of this chapter, you will have:

- PostgreSQL running in Docker with a proper initialization script
- SQL migrations that create the `exercises` table
- A global connection pool using `OnceLock<PgPool>`
- `#[server]` functions that bridge client and server
- Exercises that survive page reloads

---

## Spotlight: Async/Await & SQLx

### Why do we need something called "async"?

> **Programming Concept: What is a Database?**
>
> A **database** is a structured way to store data that survives when you close the program. Think of the difference between sticky notes and a filing cabinet:
>
> - **Memory (sticky notes):** Quick to read and write, but temporary. Close the program and everything is lost.
> - **Database (filing cabinet):** Organized, permanent, and searchable. Your data is safe even if the power goes out.
>
> PostgreSQL is one of the most popular databases in the world. It stores data in tables (like spreadsheets), and you talk to it using a language called SQL (Structured Query Language).

When your program asks the database for data, something interesting happens. The database is a separate program, often running on a separate computer. Your program sends a request over the network, the database processes it, and sends back a response. This takes *time* --- usually a few milliseconds, but that is an eternity for a CPU that can execute billions of instructions per second.

The question is: what should your program do while waiting?

> **Programming Concept: What is Async/Await?**
>
> Imagine you are at a restaurant. You place your order with the waiter. Now you have two options:
>
> 1. **Stand at the kitchen door and wait** (synchronous). You cannot do anything else until your food arrives. If ten people order at the same time, nine of them are just standing there, blocked.
>
> 2. **Sit at your table and chat with friends** (asynchronous). The waiter brings your food when it is ready. While waiting, you are free to do other things. Multiple orders can be processed at the same time.
>
> Async/await in Rust works like option 2. When your code asks the database for data, it does not block the entire program. Instead, it says "I will wait for this result" and frees up the thread to handle other requests. When the database responds, the code picks up where it left off.

Here is what async looks like in Rust:

```rust
// Synchronous --- blocks the thread while waiting
fn list_exercises_sync(pool: &PgPool) -> Vec<Exercise> {
    // Thread sits idle while PostgreSQL runs the query
    sqlx::query_as("SELECT * FROM exercises").fetch_all(pool)
}

// Asynchronous --- frees the thread while waiting
async fn list_exercises(pool: &PgPool) -> Vec<Exercise> {
    // Thread is free to handle other requests during the query
    sqlx::query_as("SELECT * FROM exercises").fetch_all(pool).await
}
```

Two keywords make the magic happen:

- **`async`** on the function declaration says "this function can pause and resume." It returns a `Future` --- a promise of a value that will arrive later.
- **`.await`** on an expression says "pause here until the result is ready."

Here is a crucial difference from JavaScript that trips up many developers: **Rust futures are lazy.** Nothing happens until you `.await` them.

```rust
// Rust --- the Future does nothing until awaited
let future = fetch_exercises();    // NO database query yet --- just creates a Future
let data = future.await;           // NOW the query runs AND we wait for the result
```

```javascript
// JavaScript --- the Promise starts immediately
const promise = fetch("/api/exercises"); // network request fires NOW
const data = await promise;              // just waits for completion
```

This laziness is deliberate. It means you can build up work without accidentally starting it. When you are ready, `.await` kicks everything off.

### The Tokio runtime

Rust does not have a built-in system for running async code. You need an external **runtime** --- a system that schedules and runs your futures. The standard choice is **Tokio**.

In `main.rs`, the `#[tokio::main]` attribute sets up the runtime:

```rust
#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    // Everything in here runs on the Tokio runtime
    // .await works here because Tokio is driving the futures
}
```

Think of Tokio as the restaurant manager who coordinates all the waiters. Without Tokio, your async functions would be recipes that nobody ever cooks.

### SQLx: compile-time verified queries

> **Programming Concept: What is SQLx?**
>
> SQLx is a Rust library for talking to databases. It has a superpower that most database libraries in other languages lack: it checks your SQL queries at compile time.
>
> This means if you write `SELECT naem FROM exercises` (typo: "naem" instead of "name"), the Rust compiler catches the mistake *before you even run the program*. In most other languages, you would only find this bug at runtime --- possibly in production, at 2 AM.

```rust
// This query is checked at compile time against the actual database schema:
let exercises = sqlx::query_as::<_, Exercise>(
    "SELECT id::text, name, category, scoring_type FROM exercises WHERE deleted_at IS NULL"
)
.fetch_all(&pool)
.await?;
```

If the `exercises` table does not have a `category` column, or if `name` is `INTEGER` instead of `TEXT`, the compiler catches it. This is one of Rust's strongest selling points for web development.

### The `FromRow` derive

SQLx needs to know how to convert database rows into Rust structs. The `#[derive(sqlx::FromRow)]` macro generates this conversion automatically:

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

The `#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]` line is important. It says: "only derive `FromRow` when compiling for the server." The WASM client does not have a database connection and does not need (or even have access to) `sqlx::FromRow`. Without this gate, the WASM build would fail.

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

If you do not have Docker installed, now is the time. Docker lets you run PostgreSQL in a container --- an isolated environment that does not interfere with anything else on your machine. Think of it as a virtual machine, but much lighter.

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

This looks like a lot, but it does three things:

1. **Launches PostgreSQL in a Docker container** --- like starting a new, clean database server
2. **Creates an application user** --- a user with limited permissions (not the all-powerful `postgres` superuser). This is a security best practice.
3. **Runs SQLx migrations** --- creates the tables your app needs

Make it executable:

```bash
chmod +x scripts/init_db.sh
```

### Step 2: Install the SQLx CLI

The SQLx command-line tool manages your database schema. Install it:

```bash
cargo install --version='~0.8' sqlx-cli \
    --no-default-features --features rustls,postgres
```

This takes a few minutes to compile. It is a one-time setup.

### Step 3: Create the migration

> **Programming Concept: What is a Migration?**
>
> A **migration** is a set of instructions for changing your database structure. Think of it as blueprints for renovating a building:
>
> - Migration 1: "Build a room called `exercises`" (CREATE TABLE)
> - Migration 2: "Add a window to the `exercises` room" (ALTER TABLE ADD COLUMN)
> - Migration 3: "Build a room called `workouts`" (CREATE TABLE)
>
> Each migration is numbered so they run in order. The database remembers which migrations have been applied, so running migrations twice is safe --- it only applies new ones.

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

Let us understand each piece:

- **`id UUID PRIMARY KEY DEFAULT gen_random_uuid()`** --- every row gets a unique ID, automatically generated. UUID stands for Universally Unique Identifier --- a random string like `550e8400-e29b-41d4-a716-446655440000` that is virtually guaranteed to be unique across all computers everywhere.
- **`name TEXT NOT NULL`** --- the exercise name is required (cannot be empty/null).
- **`TEXT[] DEFAULT '{}'`** --- PostgreSQL supports arrays. `muscle_groups` stores a list of strings in a single column.
- **`deleted_at TIMESTAMPTZ`** --- the soft delete column from Chapter 4, now in the database. `NULL` means active, non-null means deleted.
- **`CREATE UNIQUE INDEX ... WHERE deleted_at IS NULL`** --- a partial unique index. Two exercises can have the same name *only if* one of them is deleted. This prevents duplicates while allowing soft-deleted exercises to be "replaced."

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

Notice that `sqlx` and `tokio` are **optional**. They only exist when the `ssr` feature is active (server build). The WASM client never talks to the database directly --- it talks to the server, which talks to the database.

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

Let us break down each pattern you see in this file:

**`sqlx::query_as::<_, Exercise>`** runs a SQL query and maps each result row to the `Exercise` struct. The `_` lets Rust infer the database backend (PostgreSQL in our case). Each field in the struct must match a column name in the query result. We write `id::text` and `created_by::text` because the database stores UUIDs, but our struct uses `String` --- the `::text` cast converts them.

**`.bind(name)`** fills in a `$1`, `$2`, ... placeholder in the SQL query. This is called a **parameterized query**, and it is critically important for security. Never write SQL like this:

```rust
// DANGEROUS --- SQL injection vulnerability!
let query = format!("INSERT INTO exercises (name) VALUES ('{}')", name);
```

If someone enters `'; DROP TABLE exercises; --` as the name, that would delete your entire table. Parameterized queries with `.bind()` make this impossible because the database treats the value as data, never as SQL code.

**`.fetch_all(pool).await`** runs the query and collects all rows into a `Vec<Exercise>`. The `.await` yields the thread while PostgreSQL processes the query --- this is the async pattern we discussed.

**`#[cfg(feature = "ssr")]`** means these functions only exist on the server. The WASM client never calls the database directly. In the next exercise, we will create server functions that the client *can* call.

**The `?` operator with `sqlx::Error`** --- `.execute(pool).await?` returns `Result<PgQueryResult, sqlx::Error>`. The `?` unwraps success or propagates the error. This is the same `?` from Chapter 4, now appearing in async code.

<details>
<summary>Hint: If you see "the trait FromRow is not implemented"</summary>

Make sure `sqlx` is in your dependencies with the `postgres` feature, and that it is gated behind the `ssr` feature. Also verify that the `#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]` line is above the struct, not inside it.

</details>

---

## Exercise 3: Create the Global Pool with `OnceLock<PgPool>`

**Goal:** Set up a global database connection pool that is initialized once in `main()` and accessible from any server function.

### Step 1: Understand connection pools

> **Programming Concept: What is a Connection Pool?**
>
> A database connection is like a phone call. Dialing takes time (TCP handshake, authentication, protocol negotiation). If you hang up after every sentence and redial, you spend more time dialing than talking.
>
> A **connection pool** keeps several phone lines open at all times. When your code needs to talk to the database, it picks up an already-connected line. When done, it puts the line back (but does not hang up). The next request reuses the same open line.
>
> This is much faster than opening a new connection for every query.

Here is a visual:

```
Your Application
                          Connection Pool
  task 1 --borrow-->     [conn1] [conn2]
  task 2 --borrow-->     [conn3] [conn4]  ---->  PostgreSQL
  task 3 --wait--->      (all busy, wait)
  task 1 --return-->     [conn1 free!]
  task 3 --borrow-->     [conn1]
```

SQLx's `PgPool` implements this. You create it once and share it everywhere.

### Step 2: Add the global pool

Add this to the top of `src/db.rs`, above the model definitions:

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

Let us understand `OnceLock`:

- **`OnceLock`** is a container that can be set exactly once and then read by any thread. It is part of Rust's standard library.
- **`POOL.set(pool)`** stores the connection pool. If you accidentally call this twice, `.expect()` panics --- which is appropriate because double-initialization is a programmer error.
- **`POOL.get()`** returns `Option<&PgPool>`. It returns `None` if the pool has not been set yet.
- **`.cloned()`** creates a cheap copy. `PgPool` uses reference counting internally, so cloning does not create new database connections --- it just increments a counter.
- **`.ok_or_else()`** converts `None` to an error message. If somehow a server function runs before the pool is initialized, it gets a clear error instead of a crash.

Now look at the `db()` function from the caller's perspective:

```rust
let pool = db().await?;
```

One line. The caller does not know about `OnceLock`, does not worry about whether the pool is initialized, does not manage connection state. All of that complexity is hidden inside `db()`. This is good design --- a simple interface hiding the messy details.

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

The startup sequence tells a clear story:

1. **Load environment variables** --- `dotenvy::dotenv().ok()` reads the `.env` file
2. **Create the connection pool** --- `PgPoolOptions::new().max_connections(5).connect(...)` opens up to 5 connections to PostgreSQL
3. **Run migrations** --- `sqlx::migrate!()` checks which migrations have been applied and runs any new ones. The `!` means it is a macro that embeds your migration files into the binary at compile time.
4. **Store the pool globally** --- `init_pool(pool)` puts it in the `OnceLock`
5. **Start the web server** --- Axum starts listening for HTTP requests

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

If you want to skip compile-time verification temporarily, you can set the `SQLX_OFFLINE=true` environment variable, but you lose the safety guarantee.

</details>

---

## Exercise 4: Write Server Functions and Connect to the UI

**Goal:** Create `#[server]` functions that call the database and connect them to the exercises page using `Resource::new`.

### Step 1: Understand server functions

> **Programming Concept: What is a Server Function?**
>
> Imagine a restaurant where you order by phone. You (the browser) call the kitchen (the server) and say "I want a list of exercises." The kitchen looks in the fridge (database), prepares the answer, and sends it back over the phone.
>
> A **server function** is code that runs on the server but can be called from the browser. The Leptos `#[server]` macro generates two versions of the function:
>
> - **On the server:** The actual code runs, accessing the database, file system, secrets, etc.
> - **In the browser:** The function body is *replaced* with an HTTP request to the server. The arguments are sent over the network, and the response is received.
>
> The caller does not know the difference. Browser code calls `list_exercises().await` and gets back a `Vec<Exercise>`, whether it came from a direct database query or an HTTP round trip.

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

When compiled for the server (`feature = "ssr"`): the function body runs as-is.

When compiled for the browser (`feature = "hydrate"`): the function body is replaced with an HTTP POST request to `/api/list_exercises`. The `Vec<Exercise>` result is serialized on the server and deserialized in the browser.

This is why all server function arguments and return types must be `Serialize + Deserialize` --- they need to travel across the network.

### Step 2: Create the server functions

Add these to `src/app.rs` (or create a separate `src/server_fns.rs` --- we will reorganize in Chapter 6):

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

Notice how the `?` and `map_err` patterns from Chapter 4 appear everywhere:

- `crate::db::db().await?` --- get the pool or return a `ServerFnError`
- `.map_err(|e| ServerFnError::new(e.to_string()))` --- convert `sqlx::Error` into `ServerFnError` (because server functions must return `ServerFnError`, not arbitrary error types)
- `id.parse().map_err(...)?` --- convert a `String` to a UUID, handling the parse error

Every `?` is a potential early return. If the pool is not initialized, the function stops at line 1. If the database query fails, it stops at the `.map_err()?` line. The caller gets a clear error message.

### Step 3: Connect to the UI with `Resource::new`

A `Resource` in Leptos is a reactive wrapper around an async operation. It fetches data and integrates with the signal system:

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

This is where everything comes together. Let us trace what happens when a user creates a new exercise:

```
User types "Pistol Squat" and clicks "Add Exercise"
  -> create_action.dispatch(CreateExercise { name: "Pistol Squat", ... })
    -> Browser sends HTTP POST to the server
      -> create_exercise() runs on the server
        -> create_exercise_db() inserts into PostgreSQL
          -> returns Ok(())
    -> create_action.version() increments (0 -> 1)
      -> Resource sees its dependency changed
        -> Resource re-fetches: calls list_exercises()
          -> list_exercises_db() queries PostgreSQL
            -> returns Vec<Exercise> (now includes Pistol Squat)
              -> UI re-renders with the updated list
```

Let us understand each piece:

**`ServerAction::<CreateExercise>::new()`** creates an action tied to the `create_exercise` server function. The `CreateExercise` type is generated automatically by the `#[server]` macro --- it is a struct with fields matching the function's parameters.

**`.version().get()`** returns a counter that increments each time the action completes. By including this in the `Resource`'s dependency closure, the resource automatically re-fetches whenever an action finishes. Create an exercise? Counter goes from 0 to 1. Delete an exercise? Counter goes from 0 to 1. Either way, the list refreshes.

**`<Suspense>`** shows a loading fallback while the resource is fetching. On first load, the server renders the exercises directly (SSR) --- the user sees content immediately. On subsequent fetches, the existing content stays visible until the new data arrives.

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

Save everything and test. Create an exercise, reload the page --- it persists. Delete an exercise, reload --- it is gone (soft-deleted). The data lives in PostgreSQL now. This is a huge milestone.

<details>
<summary>Hint: If you see "ServerFnError: Database pool not initialized"</summary>

This means `init_pool()` was not called before the first server function ran. Check that your `main.rs` calls `gritwit::db::init_pool(pool.clone())` before starting the Axum server. Also verify that the `DATABASE_URL` is correct and PostgreSQL is running.

</details>

---

## Rust Gym

### Drill 1: Predicting Async Behavior

What does this code print, and in what order? Write your prediction before checking.

```rust
async fn greet(name: &str) -> String {
    println!("  preparing greeting for {}...", name);
    format!("Hello, {}!", name)
}

#[tokio::main]
async fn main() {
    println!("1. start");
    let future_a = greet("Alice");
    let future_b = greet("Bob");
    println!("2. futures created");
    let a = future_a.await;
    println!("3. got: {}", a);
    let b = future_b.await;
    println!("4. got: {}", b);
}
```

<details>
<summary>Hint</summary>

Remember: Rust futures are lazy. Creating a future does *not* execute the function body. The body only runs when you `.await` it.

</details>

<details>
<summary>Solution</summary>

```
1. start
2. futures created
  preparing greeting for Alice...
3. got: Hello, Alice!
  preparing greeting for Bob...
4. got: Hello, Bob!
```

Key insight: "preparing greeting for Alice..." does NOT print at step 1 when `greet("Alice")` is called. It prints at step 3 when `.await` is called. This proves that Rust futures are lazy.

If you wanted both to run concurrently:

```rust
let (a, b) = tokio::join!(greet("Alice"), greet("Bob"));
```

This starts both and waits for both to complete.

</details>

### Drill 2: `Result` in Async Functions

This function uses `.unwrap()` in two places. Replace each with `?` and `map_err`:

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
<summary>Hint</summary>

The return type needs to change from `String` to `Result<String, ServerFnError>`. Each `.unwrap()` becomes `.map_err(|e| ServerFnError::new(e.to_string()))?`.

</details>

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

Two `.unwrap()` calls became two `.map_err(...)?` calls. The return type changed from `String` to `Result<String, ServerFnError>`. The function can now fail gracefully instead of crashing.

</details>

### Drill 3: Understanding `Option` to `Result` Conversion

Convert each `Option` to a `Result` using the appropriate method:

```rust
// 1. Convert None to a specific error message
let name: Option<String> = None;
// Goal: Result<String, String> = Err("Name not found")

// 2. Convert None to a default value (no Result needed)
let count: Option<i32> = None;
// Goal: i32 = 0

// 3. Convert Some to Ok, None to Err, then use ?
fn get_first(items: &[String]) -> Result<String, String> {
    let first = items.first(); // returns Option<&String>
    // Goal: return the first item or Err("List is empty")
    todo!()
}
```

<details>
<summary>Solution</summary>

```rust
// 1. ok_or / ok_or_else converts Option to Result
let result: Result<String, String> = name.ok_or_else(|| "Name not found".to_string());

// 2. unwrap_or provides a default value
let value: i32 = count.unwrap_or(0);

// 3. Combine ok_or_else with ? for clean error propagation
fn get_first(items: &[String]) -> Result<String, String> {
    let first = items
        .first()
        .ok_or_else(|| "List is empty".to_string())?;
    Ok(first.clone())
}
```

The pattern: `.ok_or_else()` converts `Option` to `Result`, and `?` propagates the `Err`. This two-step combo is extremely common in Rust.

</details>

---

## What You Built

In this chapter, you:

1. **Set up PostgreSQL with Docker** --- init script, Docker container, application user, database creation
2. **Wrote SQL migrations** --- `CREATE TABLE exercises` with UUID primary key, array columns, partial unique index, soft delete column
3. **Built async database functions** --- `list_exercises_db`, `create_exercise_db`, `delete_exercise_db` with `sqlx::query_as` and parameterized queries
4. **Created a global pool** --- `OnceLock<PgPool>` with `init_pool()` and `db()` accessor
5. **Connected client to server** --- `#[server]` functions, `ServerAction`, `Resource::new`, `<Suspense>` for loading states

Your exercises now persist in PostgreSQL. Create one, reload the page, and it is still there. That is a fundamental milestone in any application --- the difference between a demo and a real product.

In Chapter 6, we will add multi-page routing. The Exercises, Home, History, and Log pages each get their own URL, with a bottom nav that highlights the active tab.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Binary Search Tree that organizes exercises as you insert them — and teaches you about Box and ownership from scratch in Rust — no libraries, just std.

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
