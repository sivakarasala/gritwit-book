# Chapter 18: CI/CD

Code that works on your machine is not code that works. It is code that has not been tested in a clean environment, with a fresh database, by an impartial judge. This chapter builds the automation layer that catches problems before they reach production: GitHub Actions workflows for formatting, linting, auditing, and testing — plus pre-commit hooks that run the same checks locally. By the end, every push to `main` triggers a pipeline that formats, lints, audits, and tests your code against a real PostgreSQL database.

The spotlight concept is **quality automation and Rust tooling** — the tools that enforce code quality at the language level. `cargo fmt` is not just a formatter; it is a social contract that eliminates style debates. `cargo clippy` is not just a linter; it catches real bugs that compile but behave incorrectly. `cargo deny` is not just an auditor; it prevents licensing violations and known vulnerabilities from entering your dependency tree. These tools exist because Rust's philosophy extends beyond the compiler: correctness is a spectrum, and the toolchain covers as much of it as possible.

By the end of this chapter, you will have:

- A `general.yml` GitHub Actions workflow with three parallel jobs: test, fmt, and clippy
- A PostgreSQL service container that provides a real database for integration tests
- An `audit.yml` workflow that scans dependencies daily and on every `Cargo.toml`/`Cargo.lock` change
- A `deny.toml` configuration with license allowlists, advisory ignores, and ban policies
- A `scripts/pre-commit` hook that runs `cargo fmt --check` and `cargo clippy` before every commit
- A `scripts/setup-hooks.sh` script that installs the pre-commit hook

---

## Spotlight: Quality Automation & Rust Tooling

### cargo fmt: the formatting enforcer

Rust has an official formatter, `rustfmt`, invoked via `cargo fmt`. It is not configurable by default — it enforces a single style across the entire Rust ecosystem. This is intentional. When every Rust project looks the same, you can read unfamiliar code without adjusting to a new style.

In CI, `cargo fmt` runs with `--check`:

```bash
cargo fmt --check
```

The `--check` flag prints a diff of formatting violations and exits with a non-zero status code if any exist. It does not modify files. If the check fails, the developer runs `cargo fmt` locally to fix the formatting and commits the result.

> **Coming from JS?** Prettier serves the same role for JavaScript — opinionated formatting that eliminates debates. The difference: Prettier has configuration options (semicolons, quotes, tab width). rustfmt has almost none by default. A `rustfmt.toml` file can override some settings, but the Rust community convention is to use the defaults. This means `cargo fmt` on any Rust project produces identical style.

> **Coming from Go?** `go fmt` (and `gofmt`) is the direct inspiration for `cargo fmt`. Both are canonical formatters with minimal configuration. Go was the first major language to ship an official formatter, and Rust followed the same philosophy: formatting is a solved problem, not a team decision.

### cargo clippy: the lint collection

Clippy is Rust's official linter — a collection of over 700 lints that catch common mistakes, suggest improvements, and enforce idioms. Some examples:

```rust
// Clippy warns: this `if let` can be replaced with `matches!`
if let Some(_) = value {
    true
} else {
    false
}
// Suggestion: matches!(value, Some(_))

// Clippy warns: redundant clone
let s = String::from("hello");
let t = s.clone(); // clippy: `s` is not used after this clone, so `clone()` is unnecessary

// Clippy warns: manual implementation of `map`
match result {
    Ok(val) => Some(val),
    Err(_) => None,
}
// Suggestion: result.ok()
```

In CI, clippy runs with `-D warnings` to treat warnings as errors:

```bash
cargo clippy --features ssr -- -D warnings
```

The `--features ssr` flag ensures clippy checks the server-side code (which is conditionally compiled). Without it, `#[cfg(feature = "ssr")]` blocks are skipped. The `-- -D warnings` passes the `-D warnings` flag to clippy itself (not to Cargo), promoting all warnings to errors so the CI job fails on any lint violation.

> **Coming from JS?** ESLint is the JavaScript equivalent, but with a crucial difference: ESLint requires extensive configuration (which rules, which parser, which plugins). Clippy works out of the box with sane defaults. You can suppress individual lints with `#[allow(clippy::lint_name)]` on specific items, but the default set catches real bugs without configuration.

### cargo deny: the dependency auditor

`cargo deny` checks your dependency tree against multiple databases:

- **Advisories** — known security vulnerabilities from the RustSec Advisory Database
- **Licenses** — ensures all dependencies use approved licenses
- **Bans** — prevents specific crates or duplicate versions
- **Sources** — ensures all crates come from approved registries

The configuration lives in `deny.toml`:

```toml
[advisories]
ignore = [
    { id = "RUSTSEC-2024-0436", reason = "paste is a transitive dep from Leptos, no action available" },
]

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "OpenSSL",
    "Zlib",
    "MPL-2.0",
    "CC0-1.0",
    "BSL-1.0",
]

[bans]
multiple-versions = "warn"
wildcards = "allow"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

Several important sections:

- **`[advisories].ignore`** — some advisories have no fix (the vulnerability is in a transitive dependency and no patched version exists). You acknowledge these explicitly with a reason string. This is better than disabling advisory checks entirely — you document the risk and revisit it when a fix becomes available.

- **`[licenses].allow`** — a whitelist of approved licenses. If a dependency uses a license not on this list (GPL, AGPL, proprietary), `cargo deny` fails. This prevents accidental license violations — importing a GPL crate into a commercial project could have legal consequences.

- **`[bans].multiple-versions`** — warns when two versions of the same crate are compiled (e.g., `serde 1.0.200` and `serde 1.0.197`). This increases binary size and compile time. The `"warn"` level alerts without failing; `"deny"` would fail the build.

- **`[sources]`** — restricts where crates can come from. `allow-registry` limits to the official crates.io index. `allow-git = []` means no git dependencies are allowed (they bypass the crates.io publishing process and its minimum quality checks).

> **Coming from JS?** `npm audit` checks for known vulnerabilities but has a high false-positive rate and no license checking. `cargo deny` is more comprehensive — it covers vulnerabilities, licenses, duplicate versions, and untrusted sources in a single tool. The `deny.toml` configuration is checked into version control, making the policy explicit and reviewable.

### Pre-commit hooks: local quality gate

The CI pipeline catches problems after you push. Pre-commit hooks catch them before you commit — faster feedback, fewer wasted CI minutes. GrindIt's pre-commit hook runs the same checks as the CI pipeline:

```sh
#!/bin/sh
set -e

echo "Running cargo fmt check..."
cargo fmt --check
if [ $? -ne 0 ]; then
    echo "Formatting check failed. Run 'cargo fmt' to fix."
    exit 1
fi

echo "Running cargo clippy..."
cargo clippy --features ssr -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy found warnings. Fix them before committing."
    exit 1
fi

echo "Pre-commit checks passed."
```

The `set -e` flag makes the script exit on the first error. Each check prints its purpose, runs the command, and provides a fix suggestion if it fails. The hook does not run tests (too slow for a pre-commit hook) or `cargo deny` (too slow and requires network access for advisory database updates).

The hook is installed by `scripts/setup-hooks.sh`:

```sh
#!/bin/sh
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cp "$SCRIPT_DIR/pre-commit" "$REPO_ROOT/.git/hooks/pre-commit"
chmod +x "$REPO_ROOT/.git/hooks/pre-commit"

echo "Git hooks installed."
```

The script copies the pre-commit hook from `scripts/` to `.git/hooks/` and makes it executable. `.git/hooks/` is not tracked by git, so the hook must be installed after cloning. The `scripts/` directory is tracked, so the hook definition is version-controlled and reviewable.

---

## The GitHub Actions Workflows

### general.yml: the main pipeline

The main workflow runs on every push to `main` and every pull request. It has three parallel jobs: test, fmt, and clippy.

```yaml
name: Rust

on:
  push:
    branches:
      - main
  pull_request:
    types: [opened, synchronize, reopened]
    branches:
      - main

env:
  CARGO_TERM_COLOR: always
  SQLX_VERSION: 0.8.0
  SQLX_FEATURES: "rustls,postgres"
  APP_USER: app
  APP_USER_PWD: secret
  APP_DB_NAME: gritwit
```

The `on` section triggers the workflow on pushes to `main` and on pull request events (opened, updated, reopened) targeting `main`. This means every PR gets tested before merging, and the `main` branch is tested after merging.

The `env` section defines variables shared across jobs. `CARGO_TERM_COLOR: always` enables colored output in CI logs (cargo suppresses color when stdout is not a terminal). The SQLx and database variables are used by the test job.

### The test job with PostgreSQL service container

```yaml
jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:17
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: password
          POSTGRES_DB: postgres
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v4

      - name: Install the Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly

      - name: Install sqlx-cli
        run: cargo install sqlx-cli
          --version=${{ env.SQLX_VERSION }}
          --features ${{ env.SQLX_FEATURES }}
          --no-default-features
          --locked

      - name: Create app user in Postgres
        run: |
          sudo apt-get install -y postgresql-client

          CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
          PGPASSWORD="password" psql -U "postgres" -h "localhost" -c "${CREATE_QUERY}"

          GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
          PGPASSWORD="password" psql -U "postgres" -h "localhost" -c "${GRANT_QUERY}"

      - name: Migrate database
        run: SKIP_DOCKER=true ./scripts/init_db.sh

      - name: Run tests
        run: cargo test --features ssr
```

The `services` section starts a PostgreSQL 17 container alongside the job runner. GitHub Actions maps the container's port 5432 to the host's port 5432, making it accessible at `localhost:5432`. The database is initialized with the `postgres` superuser.

The test job then:

1. **Installs sqlx-cli** — needed to run migrations. The `--locked` flag ensures the exact versions from `Cargo.lock` are used. The `--features rustls,postgres` flag builds sqlx-cli with TLS support (rustls, not OpenSSL) and PostgreSQL support.

2. **Creates the application user** — production code connects as a non-superuser (`app`). The CI pipeline creates this user with `CREATE USER` and grants `CREATEDB` permission (needed by `init_db.sh` to create the database).

3. **Runs migrations** — `SKIP_DOCKER=true ./scripts/init_db.sh` runs the initialization script without starting a Docker container (the PostgreSQL container is already running as a service). The script creates the database and runs all SQL migrations.

4. **Runs tests** — `cargo test --features ssr` compiles and runs all tests with the SSR feature enabled. The tests can access the real PostgreSQL database, making them integration tests rather than unit tests with mocks.

### The fmt and clippy jobs

```yaml
  fmt:
    name: Rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install the Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly
          components: rustfmt
      - name: Enforce formatting
        run: cargo fmt --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install the Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly
          components: clippy
      - name: Linting
        run: cargo clippy --features ssr -- -D warnings
```

These jobs are simple — install the toolchain with the required component, run the check. They run in parallel with the test job (not sequentially), so a formatting failure does not block the test run. All three must pass for the workflow to succeed.

The `components` field in `setup-rust-toolchain` installs `rustfmt` and `clippy` as part of the toolchain. These are optional components — a bare Rust installation does not include them.

### audit.yml: the security scanner

```yaml
name: Security audit

on:
  schedule:
    - cron: "0 0 * * *"
  push:
    paths:
      - "**/Cargo.toml"
      - "**/Cargo.lock"

jobs:
  security_audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/install-action@cargo-deny
      - name: Scan for vulnerabilities
        run: cargo deny check advisories
```

The audit workflow runs on two triggers:

1. **`schedule: cron: "0 0 * * *"`** — runs daily at midnight UTC. New vulnerabilities are discovered constantly; a daily scan catches them even when the code has not changed.

2. **`push: paths: ["**/Cargo.toml", "**/Cargo.lock"]`** — runs when dependency files change. This catches vulnerabilities introduced by new or updated dependencies.

The `taiki-e/install-action@cargo-deny` action installs cargo-deny as a pre-compiled binary (faster than `cargo install`). The `cargo deny check advisories` command checks only the advisories section — license and ban checks are part of a separate step or the main workflow.

---

## Exercises

### Exercise 1: Write the general.yml workflow with test, fmt, and clippy jobs

**Goal:** Create `.github/workflows/general.yml` with three parallel jobs that test, format-check, and lint the codebase.

**Instructions:**
1. Define the workflow trigger: push to `main` and pull requests targeting `main`
2. Create the `test` job with a PostgreSQL service container, sqlx-cli installation, database migration, and `cargo test`
3. Create the `fmt` job with `cargo fmt --check`
4. Create the `clippy` job with `cargo clippy --features ssr -- -D warnings`

<details>
<summary>Hint 1</summary>

The PostgreSQL service container needs `image: postgres:17`, environment variables for user/password/database, and a port mapping (`5432:5432`). GitHub Actions uses the `services` key at the job level.
</details>

<details>
<summary>Hint 2</summary>

The test job needs `postgresql-client` installed via `apt-get` to run `psql` commands for creating the app user. Use `PGPASSWORD="password" psql -U "postgres" -h "localhost"` to connect to the service container.
</details>

<details>
<summary>Hint 3</summary>

The fmt and clippy jobs need the `components` field in the toolchain setup action. `rustfmt` for the fmt job, `clippy` for the clippy job. These are separate from the toolchain itself.
</details>

<details>
<summary>Solution</summary>

```yaml
name: Rust

on:
  push:
    branches:
      - main
  pull_request:
    types: [opened, synchronize, reopened]
    branches:
      - main

env:
  CARGO_TERM_COLOR: always
  SQLX_VERSION: 0.8.0
  SQLX_FEATURES: "rustls,postgres"
  APP_USER: app
  APP_USER_PWD: secret
  APP_DB_NAME: gritwit

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:17
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: password
          POSTGRES_DB: postgres
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v4

      - name: Install the Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly

      - name: Install sqlx-cli
        run: cargo install sqlx-cli
          --version=${{ env.SQLX_VERSION }}
          --features ${{ env.SQLX_FEATURES }}
          --no-default-features
          --locked

      - name: Create app user in Postgres
        run: |
          sudo apt-get install -y postgresql-client

          CREATE_QUERY="CREATE USER ${APP_USER} WITH PASSWORD '${APP_USER_PWD}';"
          PGPASSWORD="password" psql -U "postgres" -h "localhost" -c "${CREATE_QUERY}"

          GRANT_QUERY="ALTER USER ${APP_USER} CREATEDB;"
          PGPASSWORD="password" psql -U "postgres" -h "localhost" -c "${GRANT_QUERY}"

      - name: Migrate database
        run: SKIP_DOCKER=true ./scripts/init_db.sh

      - name: Run tests
        run: cargo test --features ssr

  fmt:
    name: Rustfmt
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install the Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly
          components: rustfmt
      - name: Enforce formatting
        run: cargo fmt --check

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install the Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly
          components: clippy
      - name: Linting
        run: cargo clippy --features ssr -- -D warnings
```
</details>

### Exercise 2: Write the audit.yml workflow with cargo deny

**Goal:** Create `.github/workflows/audit.yml` that scans dependencies for known vulnerabilities on a daily schedule and when dependency files change.

**Instructions:**
1. Trigger on a daily cron schedule and on pushes that modify `Cargo.toml` or `Cargo.lock`
2. Install `cargo-deny` using the `taiki-e/install-action`
3. Run `cargo deny check advisories`

<details>
<summary>Hint 1</summary>

The cron syntax `"0 0 * * *"` means midnight UTC daily. The `schedule` trigger uses an array of cron expressions under the `schedule` key.
</details>

<details>
<summary>Hint 2</summary>

The `push.paths` filter uses glob patterns. `"**/Cargo.toml"` matches `Cargo.toml` at any depth. This catches changes to workspace member Cargo.toml files too.
</details>

<details>
<summary>Solution</summary>

```yaml
name: Security audit

on:
  schedule:
    - cron: "0 0 * * *"
  push:
    paths:
      - "**/Cargo.toml"
      - "**/Cargo.lock"

jobs:
  security_audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: taiki-e/install-action@cargo-deny
      - name: Scan for vulnerabilities
        run: cargo deny check advisories
```

The `taiki-e/install-action` installs pre-compiled binaries, avoiding the multi-minute `cargo install` compilation. For `cargo-deny`, this reduces setup time from ~3 minutes to ~5 seconds.
</details>

### Exercise 3: Configure deny.toml with license allowlist and advisory ignores

**Goal:** Create `deny.toml` with a license allowlist, an advisory ignore for a known transitive dependency issue, and source restrictions.

**Instructions:**
1. Add an `[advisories]` section with an ignore entry for `RUSTSEC-2024-0436` (a known issue in the `paste` crate, a transitive dependency of Leptos)
2. Add a `[licenses]` section with an allowlist of common open-source licenses
3. Add a `[bans]` section that warns on multiple versions and allows wildcards
4. Add a `[sources]` section that restricts to crates.io and disallows git dependencies

<details>
<summary>Hint 1</summary>

Advisory ignores use `{ id = "RUSTSEC-YYYY-NNNN", reason = "..." }` syntax. The reason string documents why you are ignoring the advisory — this is essential for code review and future reconsideration.
</details>

<details>
<summary>Hint 2</summary>

Common open-source licenses to allow: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Unicode-DFS-2016, OpenSSL, Zlib, MPL-2.0, CC0-1.0, BSL-1.0. If you are building a commercial product, you typically cannot allow GPL, AGPL, or SSPL.
</details>

<details>
<summary>Solution</summary>

```toml
[advisories]
ignore = [
    { id = "RUSTSEC-2024-0436", reason = "paste is a transitive dep from Leptos, no action available" },
]

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "OpenSSL",
    "Zlib",
    "MPL-2.0",
    "CC0-1.0",
    "BSL-1.0",
]

[bans]
multiple-versions = "warn"
wildcards = "allow"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

Run `cargo deny check` locally to verify the configuration. The first run may produce warnings about multiple versions of common crates (like `syn` or `proc-macro2`) — these are normal in projects with many dependencies and are safe to leave as warnings.
</details>

### Exercise 4: Write the pre-commit hook and setup script

**Goal:** Create `scripts/pre-commit` with formatting and linting checks, and `scripts/setup-hooks.sh` to install it.

**Instructions:**
1. Write a shell script that runs `cargo fmt --check` and `cargo clippy --features ssr -- -D warnings`
2. Each check should print what it is doing, run the command, and print a helpful fix suggestion if it fails
3. Write `setup-hooks.sh` that copies the pre-commit script to `.git/hooks/` and makes it executable
4. Test by running `./scripts/setup-hooks.sh` and then attempting a commit with a formatting violation

<details>
<summary>Hint 1</summary>

Start the script with `#!/bin/sh` and `set -e`. The `set -e` flag causes the script to exit immediately if any command fails, which is the correct behavior for a pre-commit hook — any failure should abort the commit.
</details>

<details>
<summary>Hint 2</summary>

For the setup script, use `$(cd "$(dirname "$0")" && pwd)` to find the script's directory regardless of where it is called from. Navigate to the repo root with `$(cd "$SCRIPT_DIR/.." && pwd)`.
</details>

<details>
<summary>Solution</summary>

```sh
#!/bin/sh
# scripts/pre-commit
# Pre-commit hook: runs the same checks as CI pipeline

set -e

echo "Running cargo fmt check..."
cargo fmt --check
if [ $? -ne 0 ]; then
    echo "Formatting check failed. Run 'cargo fmt' to fix."
    exit 1
fi

echo "Running cargo clippy..."
cargo clippy --features ssr -- -D warnings
if [ $? -ne 0 ]; then
    echo "Clippy found warnings. Fix them before committing."
    exit 1
fi

echo "Pre-commit checks passed."
```

```sh
#!/bin/sh
# scripts/setup-hooks.sh
# Set up git hooks from the scripts/ directory
# Run once after cloning: ./scripts/setup-hooks.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cp "$SCRIPT_DIR/pre-commit" "$REPO_ROOT/.git/hooks/pre-commit"
chmod +x "$REPO_ROOT/.git/hooks/pre-commit"

echo "Git hooks installed."
```

Test the hook:
```bash
./scripts/setup-hooks.sh
# Add a deliberate formatting violation (extra spaces)
echo "fn  main()  {}" >> src/lib.rs
git add src/lib.rs
git commit -m "test"
# Should fail with: "Formatting check failed. Run 'cargo fmt' to fix."
git checkout -- src/lib.rs
```

Note: `set -e` makes the check after each command (`if [ $? -ne 0 ]`) technically redundant — the script would already exit on failure. However, the explicit check provides a human-readable error message. Without it, the developer would see cargo's error output but not the "Run 'cargo fmt' to fix" hint.
</details>

---

## Rust Gym: Quality Tooling Drills

### Drill 1: Fix clippy warnings

<details>
<summary>Exercise</summary>

Fix the following code to pass `cargo clippy -- -D warnings`:

```rust
fn process_scores(scores: &Vec<i32>) -> Option<f64> {
    if scores.len() == 0 {
        return None;
    }

    let mut sum = 0;
    for i in 0..scores.len() {
        sum = sum + scores[i];
    }

    let avg = sum as f64 / scores.len() as f64;
    return Some(avg);
}
```

Clippy warnings:
1. `&Vec<i32>` should be `&[i32]` (clippy::ptr_arg)
2. `.len() == 0` should be `.is_empty()` (clippy::len_zero)
3. `for i in 0..scores.len()` should use an iterator (clippy::needless_range_loop)
4. `sum = sum + scores[i]` should use `+=` (clippy::assign_op_pattern)
5. `return Some(avg)` — explicit return at end of function is unnecessary (clippy::needless_return)

Fixed:

```rust
fn process_scores(scores: &[i32]) -> Option<f64> {
    if scores.is_empty() {
        return None;
    }

    let sum: i32 = scores.iter().sum();
    let avg = f64::from(sum) / scores.len() as f64;
    Some(avg)
}
```

Each fix makes the code more idiomatic. `&[i32]` accepts both `&Vec<i32>` and `&[i32]` — it is strictly more general. `is_empty()` communicates intent better than a length comparison. The iterator sum eliminates indexing (which could panic on out-of-bounds access in less controlled code). The implicit return is Rust convention for the final expression.
</details>

### Drill 2: Configure deny.toml for a new project

<details>
<summary>Exercise</summary>

You are starting a new commercial project. Write a `deny.toml` that:

1. Denies all advisories (no ignores — fix them or remove the dependency)
2. Allows only permissive licenses (MIT, Apache-2.0, BSD variants, ISC)
3. Denies multiple versions of the same crate (forces clean dependency tree)
4. Allows only crates.io as a source (no git dependencies)

```toml
[advisories]
# No ignores — every advisory must be addressed
ignore = []

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
]
# Deny anything not in the allow list
unlicensed = "deny"

[bans]
multiple-versions = "deny"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

This is a strict configuration. In practice, `multiple-versions = "deny"` may be too strict for projects with many dependencies — popular crates like `syn` often have multiple versions in the tree due to proc-macro dependencies. Start with `"warn"` and tighten to `"deny"` when your dependency tree is clean.
</details>

### Drill 3: Write a GitHub Actions job with caching

<details>
<summary>Exercise</summary>

Add Cargo caching to the clippy job to speed up CI runs:

```yaml
  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install the Rust toolchain
        uses: actions-rust-lang/setup-rust-toolchain@v1
        with:
          toolchain: nightly
          components: clippy

      - name: Cache cargo registry and build artifacts
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-clippy-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-clippy-

      - name: Linting
        run: cargo clippy --features ssr -- -D warnings
```

The cache key includes `hashFiles('**/Cargo.lock')` — when dependencies change, a new cache is created. The `restore-keys` fallback allows using a previous cache even when the exact key does not match, which is faster than a clean build.

Note: `actions-rust-lang/setup-rust-toolchain@v1` already includes some caching by default. The explicit cache step is shown here to demonstrate the pattern — in production, check whether the toolchain action's built-in caching is sufficient for your needs.
</details>

---

## DSA in Context: Pipeline as Directed Acyclic Graph

The CI workflow you built has three parallel jobs that feed into an implicit merge gate:

```
         ┌──── fmt ────┐
push ────┼── clippy ───┼──── all pass? → merge allowed
         └──── test ───┘
```

This is a directed acyclic graph (DAG). Each node is a job, and edges represent dependencies. The three jobs have no dependencies on each other (they run in parallel), but the merge gate depends on all three.

**Interview version:** CI/CD pipelines are DAG scheduling problems. More complex pipelines have explicit dependencies:

```yaml
jobs:
  build:
    # ...
  test:
    needs: build
  deploy-staging:
    needs: test
  integration-test:
    needs: deploy-staging
  deploy-production:
    needs: integration-test
```

This is a linear chain (degenerate DAG). A more realistic pipeline has diamond dependencies:

```
build ──┬── unit-test ───┬── deploy-staging
        └── lint ────────┘
```

The scheduling algorithm is **topological sort** — process nodes in an order where every node runs after all its dependencies. GitHub Actions handles this automatically with the `needs` keyword. Implementing it yourself requires the same algorithm as Chapter 8's movement prerequisites (Kahn's algorithm or DFS-based topological sort).

**Bonus challenge:** How would you model a pipeline where `deploy-production` requires manual approval? This is a DAG with a human-in-the-loop node — the node blocks until an external event (approval) resolves it. GitHub Actions implements this with environments and protection rules.

---

## System Design Corner: CI/CD Pipeline Design

**Interview question:** "Design a CI/CD pipeline for a team of 20 engineers deploying to production multiple times per day."

**What we just built:** A three-job parallel pipeline with database integration testing, formatting enforcement, linting, and dependency auditing.

**Talking points:**

- **Test pyramid** — unit tests (fast, many) at the base, integration tests (slower, fewer) in the middle, end-to-end tests (slowest, fewest) at the top. GrindIt's pipeline runs integration tests against a real PostgreSQL database (middle of the pyramid). Unit tests run within `cargo test`. End-to-end tests (browser automation) would be a separate job with a deployed staging environment.

- **Parallelism** — the fmt, clippy, and test jobs run in parallel. Formatting failures (which take 10 seconds to check) are reported at the same time as test failures (which take minutes). This reduces the total feedback time from sequential (sum of all jobs) to parallel (max of all jobs).

- **Fail fast** — formatting and linting checks are cheap. Running them in parallel with tests means developers get fast feedback on style issues without waiting for the full test suite. Some teams add a "quick check" job that runs `cargo check` (type checking without codegen) as the fastest possible feedback — it catches compile errors in seconds.

- **Database testing strategy** — GrindIt uses a service container (PostgreSQL in Docker, managed by GitHub Actions). Alternatives: SQLite for tests (faster but misses PostgreSQL-specific behavior), shared staging database (state leaks between test runs), or ephemeral databases per test (using `sqlx::test` macro). The service container approach balances realism with isolation.

- **Deployment strategies** — after the pipeline passes, deployment options include: direct push (risky), blue-green deployment (run two identical environments, switch traffic), canary deployment (route a percentage of traffic to the new version), or rolling update (replace instances one at a time). Container orchestrators like Kubernetes support all of these natively.

- **Rollback plans** — every deployment should have a rollback path. With Docker images, rollback means redeploying the previous image tag. With database migrations, rollback requires writing "down" migrations (which SQLx supports). The CI pipeline should test both the migration and the rollback path.

- **Secret management** — CI pipelines need access to secrets (database passwords, API keys). GitHub Actions provides encrypted secrets (`${{ secrets.DATABASE_PASSWORD }}`) that are masked in logs. Never hardcode secrets in workflow files. GrindIt's pipeline uses hardcoded test credentials (`password`, `secret`) because the CI database is ephemeral — production secrets are injected via environment variables at deployment time.

---

> **Design Insight: Strategic Programming** (Ousterhout, Ch. 3)
>
> Setting up CI/CD, pre-commit hooks, and dependency auditing is a **strategic investment**. It costs time upfront and produces no visible features. But it pays dividends in every future chapter: formatting debates disappear, bug categories are eliminated before code review, vulnerable dependencies are caught before deployment, and production deployments become a button press instead of a ceremony. Tactical programmers skip CI setup and pay for it later with production incidents. Strategic programmers invest in automation early and compound the returns.

---

## Summary

This chapter built the automation layer that enforces code quality across the entire development lifecycle:

- **`cargo fmt --check`** — enforces a single code style. No configuration, no debates. The formatter is the final authority on whitespace, brace placement, and import ordering.
- **`cargo clippy -- -D warnings`** — catches over 700 categories of mistakes, from unnecessary clones to manual reimplementations of standard library methods. Treating warnings as errors ensures the codebase stays clean.
- **`cargo deny check`** — audits the dependency tree for vulnerabilities, license violations, duplicate versions, and untrusted sources. The `deny.toml` configuration makes the policy explicit and version-controlled.
- **PostgreSQL service container** — provides a real database for integration tests in CI. The test job creates an application user, runs migrations, and executes tests against real SQL queries.
- **Pre-commit hooks** — run formatting and linting checks before every commit. Faster feedback than CI, fewer wasted pipeline minutes.
- **Parallel CI jobs** — fmt, clippy, and test run simultaneously. The total pipeline time is the duration of the slowest job, not the sum of all jobs.

Together, these systems form a quality ratchet — code quality can only go up. Every commit is formatted. Every push is linted, audited, and tested. Every merge to `main` is verified against a real database. The automation handles the repetitive enforcement so code reviewers can focus on design, architecture, and business logic.

This completes the feature chapters of GrindIt. The next chapter steps back from code to reflect on software design principles applied throughout the project.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Directed Acyclic Graph with topological sort for CI pipeline job ordering.

**→ [DAG Pipeline](../ds-narratives/ch18-dag-pipeline.md)**

---
