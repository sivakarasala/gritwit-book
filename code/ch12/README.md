# Chapter 12: Profile & Admin

**Spotlight:** Authorization & Role-Based Access Control

## What This Snapshot Contains

- `src/auth_guards.rs` — require_auth, require_role guard functions, role hierarchy

## What Was Built

- `require_auth()` and `require_role(min_role)` guard functions
- Role hierarchy: Athlete < Coach < Admin with `rank()` comparison
- Ownership checks: "only creator or admin can delete"
- Conditional UI rendering based on role

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
