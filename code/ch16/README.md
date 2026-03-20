# Chapter 16: REST API Layer

**Spotlight:** Axum Route Organization & API Design

## What This Snapshot Contains

- `src/routes.rs` — REST API handlers, Router::nest for versioning, middleware ordering

## What Was Built

- "Two doors, one database" — server functions for Leptos, REST for 3rd parties
- `Router::nest("/api/v1", api_routes)` for versioning
- `#[utoipa::path]` annotations for OpenAPI docs
- SwaggerUi integration
- Both doors call the same `db.rs` functions — zero business logic duplication

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
