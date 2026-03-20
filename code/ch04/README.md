# Chapter 4: Exercise CRUD

**Spotlight:** Error Handling (Result, Option, ?)

## What This Snapshot Contains

- `src/server_fns.rs` — Create, edit, soft-delete server functions with Result/Option error handling

## What Was Built

- Create form with validation returning `Result`
- Edit mode with `Option<Exercise>` (None = create, Some = edit)
- Soft delete with ownership check
- `clean_error()` utility for user-friendly error messages

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
