# Chapter 10: History & Leaderboard

**Spotlight:** Collections & Sorting Deep Dive

## What This Snapshot Contains

- `src/history.rs` — HashMap grouping, streak calculation, custom sorting for leaderboard

## What Was Built

- `HashMap<String, Vec<WorkoutLog>>` for grouping by date
- Custom sorting for leaderboard (Rx first, then score, then time)
- Streak calculation using greedy consecutive-day matching
- Iterator chains for stats aggregation

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
