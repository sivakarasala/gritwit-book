# Chapter 2: The Exercise Library

**Spotlight:** Structs & `impl` Blocks

## What This Snapshot Contains

- `src/db.rs` — Exercise struct with `new()`, `summary()`, `is_weightlifting()` methods + hardcoded data

## What Was Built

- `Exercise` struct with name, category, scoring_type fields
- Associated function `new()` and methods `summary()`, `is_weightlifting()`
- Hardcoded exercise data grouped by category
- Exercise cards rendered in the UI with color-coded category headers

## Project State After This Chapter

```
src/
├── app.rs
├── db.rs               ← included (Exercise struct + data)
├── lib.rs
├── main.rs
└── pages/
    └── exercises/
        ├── mod.rs
        └── exercise_card.rs
```

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
