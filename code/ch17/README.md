# Chapter 17: Docker & Deployment

**Spotlight:** Multi-Stage Builds & Build Optimization

## What This Snapshot Contains

- `Dockerfile` — Full 4-stage multi-stage build (Chef → Planner → Builder → Runtime)

## What Was Built

- 4-stage Dockerfile: Chef → Planner → Builder → Runtime
- cargo-chef for dependency caching
- `SQLX_OFFLINE=true` for building without a database
- Dart Sass installation for SCSS compilation
- Production configuration with env var overrides

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
