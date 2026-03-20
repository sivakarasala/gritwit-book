# Chapter 15: Configuration & Telemetry

**Spotlight:** Serde Deep Dive & Configuration Patterns

## What This Snapshot Contains

- `src/configuration.rs` — Settings struct hierarchy, YAML + env var layering
- `src/telemetry.rs` — Structured logging with tracing

## What Was Built

- `Settings` struct hierarchy with serde deserialization
- YAML base → env-specific overlay → env var override
- `TryFrom<String> for Environment` conversion
- Bunyan JSON formatter for structured logs
- TraceLayer middleware with request ID propagation

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
