# Chapter 7: User Authentication

**Spotlight:** Enums & Pattern Matching

## What This Snapshot Contains

- `src/auth.rs` — UserRole enum with `rank()`, `Display` impl, pattern matching for auth methods

## What Was Built

- `enum UserRole { Athlete, Coach, Admin }` with `rank()` method
- `match` for exhaustive handling, `if let` for Option
- `Display` trait implementation for UserRole
- OAuth flow, Argon2 password hashing, OTP verification
- Session management with tower-sessions

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
