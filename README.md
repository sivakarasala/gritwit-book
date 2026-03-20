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
- `src/capstone/` — Coding challenges, system design, mock interviews (shared)
- `src/evolution/` — Living changelog as the reference app evolves
- `code/ch01/` through `code/ch18/` — Compilable project snapshots per chapter

## Reference Implementation

The complete GrindIt app: [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)

## License

MIT
