# Chapter 11: Reusable Components

**Spotlight:** Ownership & Borrowing Deep Dive

## What This Snapshot Contains

- `src/delete_modal.rs` — DeleteModal with callback props, signal sharing, Portal
- `src/single_select.rs` — SingleSelect dropdown with ownership patterns

## What Was Built

- `DeleteModal` component with `show: RwSignal<bool>` and `on_confirm` callback
- `SingleSelect` and `MultiSelect` dropdown components
- `move ||` closures capturing signals — the clone → move pattern
- `<Portal>` for escaping overflow

> This is a progressive snapshot. For the complete compilable project, see [github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit) (tag: `book-v1`)
