# GrindIt: Learn Rust + Leptos by Building a Real Fitness Tracker

A hands-on guide to **Rust**, **Leptos**, **DSA**, and **System Design** — from zero to deployed PWA.

## What You'll Build

A production-grade CrossFit workout tracker with:
- Leptos 0.8 (SSR + WASM hydration)
- Axum 0.8 HTTP server
- SQLx + PostgreSQL
- Auth (OAuth, password, OTP)
- PWA with offline support
- REST API with Swagger UI
- Docker deployment + CI/CD

## Two Learning Tracks

| Track | For | Starts At |
|-------|-----|-----------|
| **Beginner** | Never coded before | Part 0: Programming Fundamentals |
| **Experienced** | Programmers learning Rust | Chapter 1: Hello, GrindIt! |

Both tracks build the same app and converge at the capstone chapters.

## Reading the Book

```bash
# Install mdBook
cargo install mdbook

# Build and serve locally
mdbook serve --open
```

## Structure

- `src/part-0-foundations/` — Programming fundamentals (beginner track only)
- `src/beginner/` — Beginner track chapters 1-18
- `src/experienced/` — Experienced track chapters 1-18
- `src/ds-narratives/` — 22 DS Deep Dives: narrative-driven data structures & Rust concepts built from scratch
- `src/capstone/` — Coding challenges, system design, mock interviews (shared)
- `src/evolution/` — Living changelog as the reference app evolves
- `code/ch00/` — Part 0 standalone Rust exercises
- `code/ch01/` through `code/ch18/` — Compilable project snapshots per chapter
- `code/capstone/` — 8 DSA exercises (DP, Trie, Heap, Topo Sort, etc.)

## Reference Implementation

The complete GrindIt app: [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)

## CI: Drift Detection & Sync

This repo has a CI pipeline (`.github/workflows/sync-check.yml`) that:
1. **Detects drift** between the pinned `book-v1` tag and GrindIt's `main` branch
2. **Verifies code snapshots** compile (`code/ch00`, `code/capstone`)
3. **Builds the book** and checks for warnings

It runs weekly (Monday 9am UTC), on manual trigger, and automatically when GrindIt's `main` branch is updated via cross-repo dispatch.

### Setup: Cross-Repo Dispatch

To enable automatic sync checks when the GrindIt app changes:

1. Create a **GitHub Personal Access Token** (PAT) with `repo` scope
2. Go to the **gritwit** repo → Settings → Secrets and variables → Actions
3. Add a new secret: `BOOK_REPO_TOKEN` with the PAT value
4. The `notify-book.yml` workflow in gritwit will dispatch to this repo on every push to `main`

## License

MIT
