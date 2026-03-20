# Chapter 14: PWA & WASM Interop

**Spotlight:** wasm_bindgen, web_sys, js_sys

## What This Snapshot Contains

- `src/pwa.rs` — Service worker registration, theme toggle, wasm_bindgen interop

## What Was Built

- `#[wasm_bindgen(inline_js = "...")]` for calling JS from Rust
- `web_sys::Window`, `web_sys::Element` for DOM access
- Service worker registration and cache strategies
- Theme toggle persisted in localStorage
- PWA install banner

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
