# Chapter 6: Multi-Page Routing

**Spotlight:** Modules & Project Structure

## What This Snapshot Contains

- `src/app.rs` — Router setup with all page routes
- `src/lib.rs` — Module tree organization

## What Was Built

- Organized code into `src/pages/`, `src/components/`, `src/auth/`
- `mod.rs`, `pub use`, `pub(crate)` visibility rules
- `#[cfg(feature = "ssr")]` conditional compilation
- `<Router>`, `<Routes>`, `StaticSegment` for all pages
- Active tab highlighting with `use_location().pathname`

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
