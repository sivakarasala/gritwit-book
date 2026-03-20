# Chapter 5: Database Persistence

**Spotlight:** Async/Await & SQLx

## What This Snapshot Contains

- `src/db.rs` — Async SQLx queries with global pool via OnceLock
- `migrations/20240101_create_exercises.sql` — First migration

## What Was Built

- PostgreSQL setup with Docker (`init_db.sh`)
- First migration creating the exercises table
- Async query functions: `list_exercises_db()`, `create_exercise_db()`
- Global pool with `OnceLock<PgPool>`

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
