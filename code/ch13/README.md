# Chapter 13: Video Uploads

**Spotlight:** Smart Pointers (Arc) & Enum-Based Abstraction

## What This Snapshot Contains

- `src/storage.rs` — StorageBackend enum with Local/R2 variants, Arc sharing, magic byte validation

## What Was Built

- `StorageBackend` enum with `Local` and `R2 { bucket, public_url }` variants
- `from_config()` constructor, `async fn upload()` with match dispatch
- Magic byte validation for MP4, WebM, AVI
- Axum multipart upload route with auth and size limits

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
