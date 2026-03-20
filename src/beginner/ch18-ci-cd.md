# Chapter 18: CI/CD

Code that works on your machine is not code that works. It is code that has not been tested in a clean environment, with a fresh database, by an impartial judge. This chapter builds the automation layer that catches problems before they reach production: GitHub Actions workflows for formatting, linting, auditing, and testing --- plus pre-commit hooks that run the same checks locally. By the end, every push to `main` triggers a pipeline that formats, lints, audits, and tests your code against a real PostgreSQL database.

The spotlight concept is **quality automation and Rust tooling** --- the tools that enforce code quality at the language level. `cargo fmt` is not just a formatter; it is a social contract that eliminates style debates. `cargo clippy` is not just a linter; it catches real bugs that compile but behave incorrectly. `cargo deny` is not just an auditor; it prevents licensing violations and known vulnerabilities from entering your dependency tree. These tools exist because Rust's philosophy extends beyond the compiler: correctness is a spectrum, and the toolchain covers as much of it as possible.

By the end of this chapter, you will have:

- A `general.yml` GitHub Actions workflow with three parallel jobs: test, fmt, and clippy
- A PostgreSQL service container that provides a real database for integration tests
- An `audit.yml` workflow that scans dependencies daily and on every `Cargo.toml`/`Cargo.lock` change
- A `deny.toml` configuration with license allowlists, advisory ignores, and ban policies
- A `scripts/pre-commit` hook that runs `cargo fmt --check` and `cargo clippy` before every commit
- A `scripts/setup-hooks.sh` script that installs the pre-commit hook

---

## Spotlight: Quality Automation & Rust Tooling

> **Programming Concept: What is Continuous Integration and Continuous Deployment?**
>
> Imagine you are writing a book with five co-authors. Each person writes chapters on their laptop. Without a system, you have no idea if Alice's Chapter 3 is consistent with Bob's Chapter 7. Eventually, you try to combine everything and discover conflicting assumptions, formatting inconsistencies, and broken references.
>
> **Continuous Integration (CI)** solves this by automatically checking everyone's work every time they submit changes. In software, this means:
>
> 1. A developer pushes code to a shared repository (like GitHub)
> 2. An automated system (like GitHub Actions) runs checks: does the code compile? Do tests pass? Is the formatting correct? Are there known security vulnerabilities in the dependencies?
> 3. The results are reported back --- green checkmark (all pass) or red X (something failed)
>
> **Continuous Deployment (CD)** goes one step further: if all checks pass, the code is automatically deployed to production. No manual "deploy" button.
>
> The CI/CD pipeline is like a factory quality control line. Every product (code change) passes through inspection stations (format check, linting, testing, security audit) before it can leave the factory (be deployed). A defect caught at the factory is cheap to fix. A defect caught by a customer is expensive.
>
> Key terms:
> - **Pipeline** --- the sequence of automated steps that run on each code change
> - **Job** --- a single step in the pipeline (e.g., "run tests")
> - **Workflow** --- a collection of jobs triggered by an event (e.g., "on push to main")
> - **Green build** --- all jobs passed. The code is safe to merge or deploy.

### cargo fmt: the formatting enforcer

Rust has an official formatter, `rustfmt`, invoked via `cargo fmt`. It is not configurable by default --- it enforces a single style across the entire Rust ecosystem. This is intentional. When every Rust project looks the same, you can read unfamiliar code without adjusting to a new style.

> **Programming Concept: Why Automatic Code Formatting?**
>
> In team projects, developers often disagree about code style: tabs vs spaces, where to put curly braces, how to break long lines. These debates waste time and create noisy code reviews where half the comments are about style rather than logic.
>
> An automatic formatter ends these debates permanently. The formatter's style is the style. Period. No one chooses it, no one argues about it, and everyone's code looks the same. The result:
>
> - Code reviews focus on logic and design, not whitespace
> - Reading unfamiliar code is easier because the style is predictable
> - Merge conflicts due to formatting differences disappear
>
> Rust's `cargo fmt` follows this philosophy more strictly than most formatters. While tools in other languages offer extensive configuration (semicolons yes/no, single vs double quotes), `cargo fmt` has almost no options. One project, one style. This is a feature, not a limitation.

In CI, `cargo fmt` runs with `--check`:

```bash
cargo fmt --check
```

The `--check` flag prints a diff of formatting violations and exits with a non-zero status code if any exist. It does not modify files. If the check fails, the developer runs `cargo fmt` locally to fix the formatting and commits the result.

### cargo clippy: the lint collection

Clippy is Rust's official linter --- a collection of over 700 lints that catch common mistakes, suggest improvements, and enforce idioms. Some examples:

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

> **Programming Concept: What is a Linter?**
>
> A linter is a tool that analyzes your code for potential problems without running it. The name comes from the small fibers (lint) that a clothes dryer catches --- a linter catches the small issues that the compiler misses.
>
> The Rust compiler already catches many bugs (type errors, borrow checker violations, missing match arms). But some code compiles perfectly yet is still problematic:
>
> - Using `.len() == 0` instead of the clearer `.is_empty()`
> - Cloning a value unnecessarily (wastes memory)
> - Writing `return value;` at the end of a function instead of just `value` (not idiomatic Rust)
>
> Clippy catches these patterns and suggests the idiomatic alternative. It is not just about style --- some lints catch real bugs, like accidentally comparing a floating-point number with `==` (which can fail due to rounding errors).

In CI, clippy runs with `-D warnings` to treat warnings as errors:

```bash
cargo clippy --features ssr -- -D warnings
```

The `--features ssr` flag ensures clippy checks the server-side code (which is conditionally compiled). Without it, `#[cfg(feature = "ssr")]` blocks are skipped. The `-- -D warnings` passes the `-D warnings` flag to clippy itself (not to Cargo), promoting all warnings to errors so the CI job fails on any lint violation.

### cargo deny: the dependency auditor

`cargo deny` checks your dependency tree against multiple databases:

- **Advisories** --- known security vulnerabilities from the RustSec Advisory Database
- **Licenses** --- ensures all dependencies use approved licenses
- **Bans** --- prevents specific crates or duplicate versions
- **Sources** --- ensures all crates come from approved registries

> **Programming Concept: Why Audit Dependencies?**
>
> Modern software relies on hundreds of third-party libraries (called "dependencies"). GrindIt uses over 300 crates. Each crate is written by someone else and could contain:
>
> - **Security vulnerabilities** --- a bug that attackers could exploit. Security researchers find these and publish advisories (like `RUSTSEC-2024-0436`).
> - **License violations** --- some libraries use licenses (like GPL) that require your entire project to be open-source. Using such a library in a commercial project could have legal consequences.
> - **Supply chain attacks** --- a malicious actor publishes a crate that looks useful but contains harmful code.
>
> `cargo deny` checks all of these automatically. It reads a `deny.toml` configuration file that defines your project's policies: which licenses are acceptable, which advisories you have reviewed and accepted, and which crate registries are trusted.

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

- **`[advisories].ignore`** --- some advisories have no fix (the vulnerability is in a transitive dependency and no patched version exists). You acknowledge these explicitly with a reason string. This is better than disabling advisory checks entirely --- you document the risk and revisit it when a fix becomes available.

- **`[licenses].allow`** --- a whitelist of approved licenses. If a dependency uses a license not on this list (GPL, AGPL, proprietary), `cargo deny` fails. This prevents accidental license violations --- importing a GPL crate into a commercial project could have legal consequences.

- **`[bans].multiple-versions`** --- warns when two versions of the same crate are compiled (e.g., `serde 1.0.200` and `serde 1.0.197`). This increases binary size and compile time. The `"warn"` level alerts without failing; `"deny"` would fail the build.

- **`[sources]`** --- restricts where crates can come from. `allow-registry` limits to the official crates.io index. `allow-git = []` means no git dependencies are allowed (they bypass the crates.io publishing process and its minimum quality checks).

### Pre-commit hooks: local quality gate

> **Programming Concept: What is a Git Hook?**
>
> Git hooks are scripts that run automatically at specific points in the Git workflow. A **pre-commit hook** runs before every commit. If the hook script exits with an error, the commit is aborted.
>
> Think of it as a bouncer at a nightclub door. Before your changes can enter the repository (the nightclub), the bouncer (pre-commit hook) checks that they meet the dress code (formatting) and are not causing trouble (linting). If you fail the check, you are turned away and must fix the issue before trying again.
>
> Pre-commit hooks catch problems before they leave your machine. This is faster feedback than waiting for CI (which runs after you push). It also saves CI minutes --- if the hook catches a formatting error locally, you fix it before pushing, and the CI pipeline does not have to run (and fail) on the broken commit.

The CI pipeline catches problems after you push. Pre-commit hooks catch them before you commit --- faster feedback, fewer wasted CI minutes. GrindIt's pre-commit hook runs the same checks as the CI pipeline:

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

> **Programming Concept: What is GitHub Actions?**
>
> GitHub Actions is GitHub's built-in CI/CD service. It lets you define automated workflows that run in response to events in your repository (like pushing code or opening a pull request). The workflows run on GitHub's servers --- you do not need to set up your own CI server.
>
> A workflow is defined in a YAML file inside `.github/workflows/`. When an event matches the workflow's trigger, GitHub spins up a virtual machine, checks out your code, and runs the steps you defined. The results are shown as green checkmarks or red Xs on the pull request.
>
> Key concepts:
> - **Trigger** (`on`) --- what event starts the workflow (push, pull request, schedule)
> - **Job** --- a group of steps that run on the same machine
> - **Step** --- a single command or action
> - **Action** --- a reusable step published by the community (e.g., `actions/checkout@v4` checks out your code)
> - **Service container** --- a Docker container that runs alongside your job (e.g., a PostgreSQL database for tests)

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

> **Programming Concept: What is a Service Container?**
>
> Tests that interact with a database need an actual database running. You could mock the database (pretend it is there), but that misses real bugs --- your SQL might have a typo that the mock would not catch.
>
> A **service container** is a Docker container that GitHub Actions starts alongside your test job. It runs a real PostgreSQL database that your tests can connect to. When the job finishes, the container is destroyed --- there is no cleanup needed.
>
> The `services` section in the workflow YAML configures this:
>
> ```yaml
> services:
>   postgres:
>     image: postgres:17     # Use PostgreSQL version 17
>     env:
>       POSTGRES_USER: postgres
>       POSTGRES_PASSWORD: password
>     ports:
>       - 5432:5432          # Map the database port to localhost
> ```
>
> Your test code connects to `localhost:5432` --- the same as local development. The database is fresh and empty for every workflow run, ensuring tests are isolated and reproducible.

The test job then:

1. **Installs sqlx-cli** --- needed to run migrations. The `--locked` flag ensures the exact versions from `Cargo.lock` are used. The `--features rustls,postgres` flag builds sqlx-cli with TLS support (rustls, not OpenSSL) and PostgreSQL support.

2. **Creates the application user** --- production code connects as a non-superuser (`app`). The CI pipeline creates this user with `CREATE USER` and grants `CREATEDB` permission (needed by `init_db.sh` to create the database). This mirrors the production setup.

3. **Runs migrations** --- `SKIP_DOCKER=true ./scripts/init_db.sh` runs the initialization script without starting a Docker container (the PostgreSQL container is already running as a service). The script creates the database and runs all SQL migrations from the `migrations/` directory.

4. **Runs tests** --- `cargo test --features ssr` compiles and runs all tests with the SSR feature enabled. The tests can access the real PostgreSQL database, making them integration tests rather than unit tests with mocks.

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

These jobs are simple --- install the toolchain with the required component, run the check. They run in parallel with the test job (not sequentially), so a formatting failure does not block the test run. All three must pass for the workflow to succeed.

The `components` field in `setup-rust-toolchain` installs `rustfmt` and `clippy` as part of the toolchain. These are optional components --- a bare Rust installation does not include them.

> **Programming Concept: Why Run Jobs in Parallel?**
>
> The three jobs (test, fmt, clippy) have no dependencies on each other --- the formatting check does not need test results, and tests do not need clippy results. Running them in parallel means:
>
> - **Faster feedback** --- instead of waiting for test (3 min) then fmt (10 sec) then clippy (1 min), you wait for the slowest one (3 min). Total wall time is the maximum, not the sum.
> - **Independent failure** --- if formatting fails, you see it immediately without waiting for tests. A developer can fix formatting while tests are still running.
>
> In GitHub Actions, jobs run in parallel by default. To make a job wait for another, you use the `needs` keyword: `needs: build`. Without `needs`, jobs start simultaneously.

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

1. **`schedule: cron: "0 0 * * *"`** --- runs daily at midnight UTC. New vulnerabilities are discovered constantly; a daily scan catches them even when the code has not changed. The cron syntax `"0 0 * * *"` means "at minute 0, hour 0, every day of the month, every month, every day of the week."

2. **`push: paths: ["**/Cargo.toml", "**/Cargo.lock"]`** --- runs when dependency files change. This catches vulnerabilities introduced by new or updated dependencies.

The `taiki-e/install-action@cargo-deny` action installs cargo-deny as a pre-compiled binary (faster than `cargo install`). The `cargo deny check advisories` command checks only the advisories section --- license and ban checks are part of a separate step or the main workflow.

---

## Exercises

### Exercise 1: Write the general.yml workflow with test, fmt, and clippy jobs

**Goal:** Create `.github/workflows/general.yml` with three parallel jobs that test, format-check, and lint the codebase.

**Instructions:**

1. Create the file `.github/workflows/general.yml`. You will need to create the directories if they do not exist: `.github/` and `.github/workflows/`.
2. Define the workflow trigger: it should run on pushes to `main` and on pull requests targeting `main`. The pull request types should include `opened`, `synchronize` (when new commits are pushed to the PR), and `reopened`.
3. Define shared environment variables: `CARGO_TERM_COLOR: always`, `SQLX_VERSION: 0.8.0`, `SQLX_FEATURES: "rustls,postgres"`, `APP_USER: app`, `APP_USER_PWD: secret`, `APP_DB_NAME: gritwit`.
4. Create the `test` job with a PostgreSQL service container (image `postgres:17`, user `postgres`, password `password`). The job should: install the Rust nightly toolchain, install sqlx-cli, create the app user in PostgreSQL, run database migrations, and run `cargo test --features ssr`.
5. Create the `fmt` job: install the Rust nightly toolchain with the `rustfmt` component, then run `cargo fmt --check`.
6. Create the `clippy` job: install the Rust nightly toolchain with the `clippy` component, then run `cargo clippy --features ssr -- -D warnings`.

<details>
<summary>Hint 1</summary>

The PostgreSQL service container needs `image: postgres:17`, environment variables for user/password/database, and a port mapping (`5432:5432`). GitHub Actions uses the `services` key at the job level, not at the step level. The container starts before any steps run and is available at `localhost:5432`.
</details>

<details>
<summary>Hint 2</summary>

The test job needs `postgresql-client` installed via `apt-get` to run `psql` commands for creating the app user. Use `PGPASSWORD="password" psql -U "postgres" -h "localhost"` to connect to the service container. The `PGPASSWORD` environment variable avoids an interactive password prompt.
</details>

<details>
<summary>Hint 3</summary>

The fmt and clippy jobs need the `components` field in the toolchain setup action. `rustfmt` for the fmt job, `clippy` for the clippy job. Without the `components` field, the toolchain installs without these optional tools, and `cargo fmt` or `cargo clippy` would fail with "command not found."
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

1. Create the file `.github/workflows/audit.yml`
2. Define two triggers: a daily cron schedule (midnight UTC) and a push trigger that only fires when `Cargo.toml` or `Cargo.lock` files change. The cron syntax for "every day at midnight" is `"0 0 * * *"`.
3. Create a single job called `security_audit` that: checks out the code, installs `cargo-deny` using the `taiki-e/install-action` (which downloads a pre-compiled binary --- much faster than compiling from source), and runs `cargo deny check advisories`.

<details>
<summary>Hint 1</summary>

The cron syntax `"0 0 * * *"` means midnight UTC daily. The five fields are: minute (0), hour (0), day of month (*), month (*), day of week (*). The `schedule` trigger uses an array of cron expressions under the `schedule` key.
</details>

<details>
<summary>Hint 2</summary>

The `push.paths` filter uses glob patterns. `"**/Cargo.toml"` matches `Cargo.toml` at any depth in the repository. This catches changes to workspace member Cargo.toml files too. The trigger only fires when files matching the pattern are modified in the push.
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

1. Create `deny.toml` in the project root
2. Add an `[advisories]` section with an ignore entry for `RUSTSEC-2024-0436` (a known issue in the `paste` crate, a transitive dependency of Leptos that has no fix available). Each ignore entry needs an `id` and a `reason` explaining why you are ignoring it --- this is essential for code review and future reconsideration.
3. Add a `[licenses]` section with an allowlist of common open-source licenses. Include: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Unicode-DFS-2016, OpenSSL, Zlib, MPL-2.0, CC0-1.0, BSL-1.0. These are all permissive licenses safe for commercial use.
4. Add a `[bans]` section that warns (but does not fail) on multiple versions of the same crate, and allows wildcards in dependency version specifications.
5. Add a `[sources]` section that restricts crate sources to the official crates.io registry and disallows git dependencies.

<details>
<summary>Hint 1</summary>

Advisory ignores use `{ id = "RUSTSEC-YYYY-NNNN", reason = "..." }` syntax inside the `ignore` array. The reason string documents why you are ignoring the advisory --- this is essential for code review and future reconsideration. When a fix becomes available, you remove the ignore entry.
</details>

<details>
<summary>Hint 2</summary>

Common open-source licenses to allow: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Unicode-DFS-2016, OpenSSL, Zlib, MPL-2.0, CC0-1.0, BSL-1.0. If you are building a commercial product, you typically cannot allow GPL, AGPL, or SSPL --- these licenses have "copyleft" requirements that could force your entire project to be open-source.
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

Run `cargo deny check` locally to verify the configuration. The first run may produce warnings about multiple versions of common crates (like `syn` or `proc-macro2`) --- these are normal in projects with many dependencies and are safe to leave as warnings.
</details>

### Exercise 4: Write the pre-commit hook and setup script

**Goal:** Create `scripts/pre-commit` with formatting and linting checks, and `scripts/setup-hooks.sh` to install it.

**Instructions:**

1. Create `scripts/pre-commit` --- a shell script that runs two checks before every commit:
   - `cargo fmt --check` --- if it fails, print "Formatting check failed. Run 'cargo fmt' to fix." and exit with error code 1
   - `cargo clippy --features ssr -- -D warnings` --- if it fails, print "Clippy found warnings. Fix them before committing." and exit with error code 1
   - If both pass, print "Pre-commit checks passed."
2. Start the script with `#!/bin/sh` (the shell interpreter) and `set -e` (exit on first error)
3. Create `scripts/setup-hooks.sh` that copies the pre-commit script to `.git/hooks/pre-commit` and makes it executable with `chmod +x`
4. Test by running `./scripts/setup-hooks.sh` and then attempting a commit with a formatting violation

<details>
<summary>Hint 1</summary>

Start the script with `#!/bin/sh` and `set -e`. The `set -e` flag causes the script to exit immediately if any command fails, which is the correct behavior for a pre-commit hook --- any failure should abort the commit. Each check should echo what it is doing before running the command, so the developer sees which check failed.
</details>

<details>
<summary>Hint 2</summary>

For the setup script, use `$(cd "$(dirname "$0")" && pwd)` to find the script's directory regardless of where it is called from. Navigate to the repo root with `$(cd "$SCRIPT_DIR/.." && pwd)`. This way, the script works whether you call it from the project root (`./scripts/setup-hooks.sh`) or from inside the `scripts/` directory (`./setup-hooks.sh`).
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

Note: `set -e` makes the check after each command (`if [ $? -ne 0 ]`) technically redundant --- the script would already exit on failure. However, the explicit check provides a human-readable error message. Without it, the developer would see cargo's error output but not the helpful "Run 'cargo fmt' to fix" suggestion.
</details>

---

## Rust Gym: Quality Tooling Drills

These drills practice using Rust's quality tools and understanding what they catch.

### Drill 1: Fix clippy warnings

<details>
<summary>Exercise</summary>

Fix the following code to pass `cargo clippy -- -D warnings`. Each issue is a common mistake that clippy catches:

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

Clippy flags five issues:
1. `&Vec<i32>` should be `&[i32]` --- a slice is more general (accepts both vectors and arrays)
2. `.len() == 0` should be `.is_empty()` --- more readable and communicates intent
3. `for i in 0..scores.len()` should use an iterator --- eliminates the indexing and potential bounds issues
4. `sum = sum + scores[i]` should use `+=` --- more concise
5. `return Some(avg)` at the end --- in Rust, the last expression is the return value; explicit `return` is unnecessary

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

Each fix makes the code more idiomatic Rust. The fixed version is shorter, clearer, and avoids potential pitfalls.
</details>

### Drill 2: Read a clippy suggestion and understand it

<details>
<summary>Exercise</summary>

You run `cargo clippy` and see this warning:

```
warning: you seem to be trying to use `match` for destructuring a single pattern
  --> src/main.rs:15:5
   |
15 | /     match user.role {
16 | |         UserRole::Admin => do_admin_stuff(),
17 | |         _ => {},
18 | |     }
   | |_____^
   |
   = help: for further information visit https://rust-lang.github.io/rust-clippy/...
help: try
   |
15 |     if let UserRole::Admin = user.role { do_admin_stuff() }
```

What is clippy telling you?

The `match` statement only does something for one variant (`Admin`) and ignores all others (`_ => {}`). This is exactly the pattern that `if let` is designed for. `if let` says "if this value matches this one pattern, do something; otherwise, do nothing." It is simpler and communicates the intent more clearly than a `match` with a catch-all empty arm.

Apply the fix:

```rust
// Before (verbose)
match user.role {
    UserRole::Admin => do_admin_stuff(),
    _ => {},
}

// After (concise)
if let UserRole::Admin = user.role {
    do_admin_stuff();
}
```
</details>

### Drill 3: Understand a deny.toml license failure

<details>
<summary>Exercise</summary>

You add a new dependency and `cargo deny check licenses` fails with:

```
error[licenses]: failed to satisfy license requirements
  ├── crate fancy-feature v0.3.0
  │   ├── is licensed as GPL-3.0-only
  │   └── GPL-3.0-only is not in the allow list
```

What happened and what are your options?

The crate `fancy-feature` uses the GPL-3.0 license, which is not in your `deny.toml` allow list. GPL-3.0 is a "copyleft" license that requires any software using it to also be released under GPL-3.0. For a commercial project, this could be a legal problem.

Your options:
1. **Find an alternative crate** with a permissive license (MIT, Apache-2.0). This is usually the safest choice.
2. **Add GPL-3.0-only to the allow list** --- only if your project is also GPL-licensed or if the legal team approves.
3. **Rewrite the functionality yourself** --- if the crate is small, implement what you need without the dependency.
4. **Contact the crate author** --- some authors are willing to dual-license under a permissive license if asked.

Never blindly add licenses to the allow list. Each addition is a legal decision that affects the entire project.
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

The scheduling algorithm is **topological sort** --- process nodes in an order where every node runs after all its dependencies. GitHub Actions handles this automatically with the `needs` keyword. Implementing it yourself requires the same algorithm as Chapter 8's movement prerequisites (Kahn's algorithm or DFS-based topological sort).

**Bonus challenge:** How would you model a pipeline where `deploy-production` requires manual approval? This is a DAG with a human-in-the-loop node --- the node blocks until an external event (approval) resolves it. GitHub Actions implements this with environments and protection rules.

---

## System Design Corner: CI/CD Pipeline Design

**Interview question:** "Design a CI/CD pipeline for a team of 20 engineers deploying to production multiple times per day."

**What we just built:** A three-job parallel pipeline with database integration testing, formatting enforcement, linting, and dependency auditing.

**Talking points:**

- **Test pyramid** --- unit tests (fast, many) at the base, integration tests (slower, fewer) in the middle, end-to-end tests (slowest, fewest) at the top. GrindIt's pipeline runs integration tests against a real PostgreSQL database (middle of the pyramid). Unit tests run within `cargo test`. End-to-end tests (browser automation) would be a separate job with a deployed staging environment.

- **Parallelism** --- the fmt, clippy, and test jobs run in parallel. Formatting failures (which take 10 seconds to check) are reported at the same time as test failures (which take minutes). This reduces the total feedback time from sequential (sum of all jobs) to parallel (max of all jobs).

- **Fail fast** --- formatting and linting checks are cheap. Running them in parallel with tests means developers get fast feedback on style issues without waiting for the full test suite. Some teams add a "quick check" job that runs `cargo check` (type checking without codegen) as the fastest possible feedback --- it catches compile errors in seconds.

- **Database testing strategy** --- GrindIt uses a service container (PostgreSQL in Docker, managed by GitHub Actions). Alternatives: SQLite for tests (faster but misses PostgreSQL-specific behavior), shared staging database (state leaks between test runs), or ephemeral databases per test (using `sqlx::test` macro). The service container approach balances realism with isolation.

- **Deployment strategies** --- after the pipeline passes, deployment options include: direct push (risky), blue-green deployment (run two identical environments, switch traffic), canary deployment (route a percentage of traffic to the new version), or rolling update (replace instances one at a time). Container orchestrators like Kubernetes support all of these natively.

- **Rollback plans** --- every deployment should have a rollback path. With Docker images, rollback means redeploying the previous image tag. With database migrations, rollback requires writing "down" migrations (which SQLx supports). The CI pipeline should test both the migration and the rollback path.

- **Secret management** --- CI pipelines need access to secrets (database passwords, API keys). GitHub Actions provides encrypted secrets (`${{ secrets.DATABASE_PASSWORD }}`) that are masked in logs. Never hardcode secrets in workflow files. GrindIt's pipeline uses hardcoded test credentials (`password`, `secret`) because the CI database is ephemeral --- production secrets are injected via environment variables at deployment time.

---

> **Design Insight: Strategic Programming** (Ousterhout, Ch. 3)
>
> Setting up CI/CD, pre-commit hooks, and dependency auditing is a **strategic investment**. It costs time upfront and produces no visible features. But it pays dividends in every future chapter: formatting debates disappear, bug categories are eliminated before code review, vulnerable dependencies are caught before deployment, and production deployments become a button press instead of a ceremony. Tactical programmers skip CI setup and pay for it later with production incidents. Strategic programmers invest in automation early and compound the returns.

---

## What You Built

This chapter built the automation layer that enforces code quality across the entire development lifecycle:

- **`cargo fmt --check`** --- enforces a single code style. No configuration, no debates. The formatter is the final authority on whitespace, brace placement, and import ordering.
- **`cargo clippy -- -D warnings`** --- catches over 700 categories of mistakes, from unnecessary clones to manual reimplementations of standard library methods. Treating warnings as errors ensures the codebase stays clean.
- **`cargo deny check`** --- audits the dependency tree for vulnerabilities, license violations, duplicate versions, and untrusted sources. The `deny.toml` configuration makes the policy explicit and version-controlled.
- **PostgreSQL service container** --- provides a real database for integration tests in CI. The test job creates an application user, runs migrations, and executes tests against real SQL queries.
- **Pre-commit hooks** --- run formatting and linting checks before every commit. Faster feedback than CI, fewer wasted pipeline minutes.
- **Parallel CI jobs** --- fmt, clippy, and test run simultaneously. The total pipeline time is the duration of the slowest job, not the sum of all jobs.

Together, these systems form a quality ratchet --- code quality can only go up. Every commit is formatted. Every push is linted, audited, and tested. Every merge to `main` is verified against a real database. The automation handles the repetitive enforcement so code reviewers can focus on design, architecture, and business logic.

This completes the feature chapters of GrindIt. The next chapter steps back from code to reflect on software design principles applied throughout the project.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Directed Acyclic Graph with topological sort for CI pipeline job ordering from scratch in Rust — no libraries, just std.

**→ [DAG Pipeline](../ds-narratives/ch18-dag-pipeline.md)**

---
