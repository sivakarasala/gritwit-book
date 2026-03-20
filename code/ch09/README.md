# Chapter 9: Workout Logging & Scoring

**Spotlight:** Traits & Generics

## What This Snapshot Contains

- `src/log_workout.rs` — Scoring types, trait usage, callback props pattern

## What Was Built

- `Serialize`/`Deserialize` for all models (serde traits)
- `impl IntoView` as return type for components
- `impl Fn() + Copy + 'static` for callback props
- Generic `Resource::new()` pattern for async data fetching
- Different scoring types (ForTime, AMRAP, Strength)

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
