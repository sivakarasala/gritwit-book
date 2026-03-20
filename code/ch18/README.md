# Chapter 18: CI/CD

**Spotlight:** Quality Automation & Rust Tooling

## What This Snapshot Contains

- `.github/workflows/general.yml` — Main CI pipeline
- `deny.toml` — Dependency auditing configuration
- `scripts/pre-commit` — Pre-commit hook

## What Was Built

- `cargo fmt --check` for formatting
- `cargo clippy -- -D warnings` for linting
- `cargo deny check` for license/vulnerability auditing
- PostgreSQL service container for integration tests
- Pre-commit hooks running fmt + clippy locally

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
