# Chapter 3: Search & Filter

**Spotlight:** Closures & Iterators

## What This Snapshot Contains

- `src/exercises_page.rs` — Search bar, category filter, expand/collapse with closures and iterator chains

## What Was Built

- Real-time search input bound to `RwSignal<String>`
- Filter exercises with `.iter().filter().collect()` using closures
- Expand/collapse cards with `RwSignal<Option<String>>`
- Category grouping with `.filter().count()` for badge counts

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
