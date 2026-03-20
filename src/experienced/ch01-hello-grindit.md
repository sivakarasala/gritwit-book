# Chapter 1: Hello, GrindIt!

You are about to build a real fitness tracking app in Rust. Not a toy. Not "Hello World." A dark-themed, mobile-ready app shell with a header and bottom navigation bar — the skeleton that every page of GrindIt will live inside.

By the end of this chapter, you will have:

- A running Leptos app at `localhost:3000`
- A dark-themed header with the "GrindIt" logo and a lightning bolt icon
- A bottom navigation bar with 4 tabs: Home, Exercises, +Log, History
- SCSS styling with CSS custom properties for theming
- A clear mental model of how Leptos compiles your code twice — once for the server, once for the browser

---

## Spotlight: Variables, Types & the Leptos Toolchain

Every chapter in this book has one **spotlight concept** — the Rust idea we dig into deeply. This chapter's spotlight is **variables and types**, and how they interact with the Leptos build system.

### Variables: `let` bindings and immutability by default

In Rust, you declare variables with `let`. Unlike most languages you know, variables are **immutable by default**:

```rust
let name = "GrindIt";    // immutable — cannot be reassigned
let mut count = 0;        // mutable — can be reassigned
count += 1;               // OK
// name = "FitTrack";     // ERROR: cannot assign twice to immutable variable
```

This is not a quirk — it is a deliberate design choice. Immutability by default means the compiler catches accidental mutations. You opt into mutability explicitly with `mut`, which makes your intent clear to anyone reading the code.

> **Coming from JS/Python/Go?**
>
> | Language | Immutable | Mutable |
> |----------|-----------|---------|
> | JavaScript | `const x = 5;` | `let x = 5;` |
> | Python | *(no keyword — everything is mutable)* | `x = 5` |
> | Go | `const x = 5` | `var x = 5` or `x := 5` |
> | Rust | `let x = 5;` | `let mut x = 5;` |
>
> Notice that Rust's `let` is closer to JavaScript's `const` than to JavaScript's `let`. If you want reassignment, you ask for it.

### Type inference

Rust has strong static typing, but you rarely need to write types explicitly. The compiler infers them:

```rust
let port = 3000;           // inferred as i32 (default integer type)
let name = "GrindIt";      // inferred as &str (a string slice — a reference to text)
let running = true;         // inferred as bool
let pi = 3.14;             // inferred as f64 (default float type)
```

When inference is not enough — or when you want to be explicit — you annotate:

```rust
let port: u16 = 3000;     // unsigned 16-bit integer (0 to 65535 — perfect for ports)
```

### `String` vs `&str` — the two string types

This trips up every newcomer. Rust has two main string types:

- **`&str`** — a *string slice*. A read-only view into string data. Zero-cost, does not own the data. This is what string literals like `"hello"` produce.
- **`String`** — an *owned string*. Heap-allocated, growable, owned by the variable. You create one with `String::from("hello")` or `"hello".to_string()`.

```rust
let greeting: &str = "Hello";                    // borrowed, read-only
let owned_greeting: String = String::from("Hello"); // owned, can be modified
```

In Leptos `view!` macros, string literals are `&str` and that works fine. You will encounter `String` when dealing with data from databases, user input, or anywhere text needs to be owned and passed around.

> **Coming from JS/Python/Go?**
>
> Think of `&str` like a JavaScript `const` string — it exists, you can read it, but you do not own the memory. `String` is like a `let` string you can `.push()` onto. In Python, all strings are immutable and managed by the runtime, so you never think about this. In Go, `string` is immutable and `[]byte` is the mutable counterpart. Rust makes the ownership explicit because there is no garbage collector deciding when to free the memory — you decide, and the compiler enforces it.

### The Leptos toolchain: dual compilation

Here is the key insight about Leptos that will save you hours of confusion: **your code is compiled twice**.

```
                    ┌──────────────┐
                    │  Your Code   │
                    │  (src/*.rs)  │
                    └──────┬───────┘
                           │
              ┌────────────┴────────────┐
              │                         │
     ┌────────▼─────────┐    ┌─────────▼────────┐
     │  feature = "ssr"  │    │ feature = "hydrate"│
     │  Server Binary    │    │  WASM Bundle       │
     │  (runs on server) │    │  (runs in browser) │
     └──────────────────┘    └────────────────────┘
```

When you run `cargo leptos watch`, the tool:

1. Compiles your crate with the `ssr` feature enabled, producing a **server binary** (an Axum web server that renders HTML)
2. Compiles your crate with the `hydrate` feature enabled, producing a **WASM bundle** (WebAssembly that runs in the browser and makes the page interactive)

The server renders the initial HTML (fast first paint, good for SEO). The browser loads the WASM bundle and "hydrates" the page — attaching event handlers to the already-rendered HTML so clicks and interactions work.

This is why `Cargo.toml` has two feature flags, and why `cargo leptos watch` is the right command — not `cargo run`, which would only compile the server binary and skip the WASM half entirely.

---

## Exercise 1: Set Up the Project

**Goal:** Install the Leptos toolchain, scaffold a new project, and see it running in your browser.

### Prerequisites

You need Rust installed. If you have not done this yet:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Accept the defaults, then restart your terminal (or run `source ~/.cargo/env`).

Verify:

```bash
rustc --version
cargo --version
```

### Step 1: Add the WASM compilation target

Leptos compiles part of your code to WebAssembly. Rust does not include the WASM target by default:

```bash
rustup target add wasm32-unknown-unknown
```

### Step 2: Install cargo-leptos

`cargo-leptos` is the build tool that orchestrates the dual compilation:

```bash
cargo install cargo-leptos
```

This may take a few minutes on first install.

### Step 3: Create the project

```bash
cd ~/rusty
cargo leptos new gritwit
```

When prompted, select **Axum** as the server framework. This generates a starter project with the correct directory structure.

```bash
cd gritwit
```

### Step 4: Run it

```bash
cargo leptos watch
```

The first build takes a while (Leptos has many dependencies). When you see output like:

```
listening on http://0.0.0.0:3000
```

Open [http://localhost:3000](http://localhost:3000) in your browser. You should see the default Leptos starter page.

> **Why `cargo leptos watch` and not `cargo run`?**
>
> `cargo run` compiles and runs the server binary only. It does not compile the WASM client bundle, and it does not process SCSS into CSS. `cargo leptos watch` does all three — and it watches for file changes, recompiling automatically when you save.

### Step 5: Understand the project structure

After scaffolding, your project looks like this:

```
gritwit/
├── Cargo.toml          # Project configuration and dependencies
├── public/             # Static assets (served as-is)
├── src/
│   ├── main.rs         # Server entry point (only compiled with "ssr" feature)
│   ├── lib.rs          # Library root — shared between server and client
│   └── app.rs          # Your Leptos components live here
└── style/
    └── main.scss       # SCSS entry point
```

Open `Cargo.toml` and study it. The key sections:

```toml
[package]
name = "gritwit"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
leptos = { version = "0.8" }
leptos_meta = { version = "0.8" }
serde = { version = "1", features = ["derive"] }
console_error_panic_hook = { version = "0.1", optional = true }
wasm-bindgen = { version = "0.2.106", optional = true }

[features]
hydrate = ["leptos/hydrate", "dep:console_error_panic_hook", "dep:wasm-bindgen"]
ssr = ["leptos/ssr", "leptos_meta/ssr"]

[package.metadata.leptos]
bin-target = "gritwit"
output-name = "gritwit"
site-root = "target/site"
site-pkg-dir = "pkg"
style-file = "style/main.scss"
assets-dir = "public"
site-addr = "0.0.0.0:3000"
reload-port = 3001
env = "DEV"
bin-features = ["ssr"]
bin-default-features = false
lib-features = ["hydrate"]
lib-default-features = false
lib-profile-release = "wasm-release"
```

A few things worth noting:

- **`crate-type = ["cdylib", "rlib"]`** — `cdylib` produces a C-compatible dynamic library (which wasm-bindgen needs to create the WASM bundle). `rlib` is the standard Rust library format (which the server binary links against). Two crate types for two compilation targets.
- **`[features]`** — `ssr` and `hydrate` are mutually exclusive feature flags. The `bin-features` and `lib-features` in `[package.metadata.leptos]` tell cargo-leptos which to use for which target.
- **`style-file = "style/main.scss"`** — cargo-leptos compiles this SCSS file into CSS and serves it at `/pkg/gritwit.css`.

<details>
<summary>Hint: If the starter project layout differs from what's shown here</summary>

The `cargo leptos new` template may evolve over time. The important thing is that you have `src/app.rs` (or equivalent), `src/main.rs`, `src/lib.rs`, and `style/main.scss`. If files are named slightly differently, adjust the instructions accordingly. The concepts are the same.

</details>

---

## Exercise 2: Build the Header Component

**Goal:** Replace the starter page content with a `<Header>` component showing the GrindIt logo with a lightning bolt icon.

### Step 1: Understand the shell

Open `src/app.rs`. You will see (or create) a `shell` function. This is the outer HTML document — the `<html>`, `<head>`, and `<body>` tags. It is rendered only on the server.

Replace the contents of `src/app.rs` with:

```rust
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/gritwit.css"/>
        <Title text="GrindIt"/>
        <Header/>
        <main>
            <p>"Welcome to GrindIt!"</p>
        </main>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <header class="top-bar">
            <span class="top-bar__logo">
                "Grind"
                <span class="top-bar__flame"></span>
                "t"
            </span>
        </header>
    }
}
```

### Step 2: Break down what you just wrote

**The `shell` function** is a plain function (not a component) that returns the HTML document skeleton. It takes `LeptosOptions` as an argument — this contains the configuration from `Cargo.toml` (site address, package directory, etc.). `AutoReload` injects a script that reloads the page during development. `HydrationScripts` injects the `<script>` tag that loads your WASM bundle.

**The `#[component]` macro** transforms a function into a Leptos component. Components must:
- Be named in `PascalCase` (e.g., `Header`, not `header`)
- Return `impl IntoView`
- Use the `view!` macro to describe their HTML output

**The `view!` macro** looks like HTML but is actually Rust code that builds a virtual DOM. Key differences from HTML/JSX:

> **Coming from JS/Python/Go?**
>
> If you have used React, the `view!` macro is Leptos's equivalent of JSX:
>
> ```jsx
> // React JSX
> function Header() {
>   return (
>     <header className="top-bar">
>       <span className="top-bar__logo">
>         Grind<span className="top-bar__flame"></span>t
>       </span>
>     </header>
>   );
> }
> ```
>
> ```rust
> // Leptos view! macro
> #[component]
> fn Header() -> impl IntoView {
>     view! {
>         <header class="top-bar">
>             <span class="top-bar__logo">
>                 "Grind"
>                 <span class="top-bar__flame"></span>
>                 "t"
>             </span>
>         </header>
>     }
> }
> ```
>
> Three differences to memorize:
> 1. **`class` not `className`** — Leptos uses standard HTML attributes
> 2. **Strings must be quoted** — `"Grind"` not `Grind`. The macro needs to distinguish text content from Rust expressions.
> 3. **Self-closing tags need the slash** — `<meta charset="utf-8"/>` not `<meta charset="utf-8">`

**The logo trick:** The GrindIt logo reads "Grind⚡t" — the word "GrindIt" with a lightning bolt replacing the "I". We achieve this by placing a `<span class="top-bar__flame">` between the text "Grind" and "t". The flame icon is rendered entirely via CSS (which we will write in Exercise 3).

<details>
<summary>Hint: If you see "unresolved import" errors</summary>

Make sure your `src/lib.rs` includes `pub mod app;` and that it re-exports the `shell` function if your `main.rs` references it. The starter template should have this wired up, but if you see import errors, check the module declarations.

</details>

Save the file. If `cargo leptos watch` is running, it will recompile automatically. The page will not look like much yet — we need the CSS.

---

## Exercise 3: Add SCSS Dark Theme

**Goal:** Create the dark theme with CSS custom properties, a browser reset, and layout rules that position the header at the top and navigation at the bottom with a scrollable main area between them.

### Step 1: Create the theme file

Create `style/_themes.scss`:

```scss
// ============================================
// GrindIt Theme System
// Dark: "Ember" — fiery orange on deep night
// ============================================

:root,
[data-theme="dark"] {
  // Backgrounds
  --bg-base: #0f0f1a;
  --bg-card: #1a1a2e;
  --bg-input: #0f0f1a;
  --bg-hover: #22223a;

  // Borders
  --border: #2a2a3e;

  // Text
  --text-primary: #f5e6d0;
  --text-secondary: #e0e0e0;
  --text-muted: #aaaaaa;
  --text-dim: #888888;
  --text-faint: #666666;

  // Accent — Ember orange
  --accent: #f97316;
  --accent-hover: #ea580c;
  --accent-rgb: 249, 115, 22;

  // Flame icon
  --flame: #ffd700;
}
```

These **CSS custom properties** (CSS variables) define the color palette. Every component in the app will reference these variables instead of hard-coding colors. Later chapters will add a light theme by defining the same variables with different values under `[data-theme="light"]`.

> **Why CSS custom properties instead of SCSS variables?**
>
> SCSS variables (`$bg-base: #0f0f1a;`) are resolved at compile time — they become hard-coded hex values in the output CSS. CSS custom properties (`--bg-base: #0f0f1a;`) exist at runtime — they can be changed dynamically (e.g., toggling between dark and light themes) without recompiling the stylesheet.

### Step 2: Create the reset and layout file

Create `style/_reset.scss`:

```scss
* {
  box-sizing: border-box;
}

// Prevent iOS Safari auto-zoom on input focus (triggers at font-size < 16px)
input,
textarea,
select {
  font-size: 16px;
}

:root {
  --header-h: 2.5rem;
  --nav-h: 3.5rem;
}

html,
body {
  height: 100%;
  overflow: hidden;
}

body {
  font-family:
    "Inter",
    "SF Pro Display",
    -apple-system,
    BlinkMacSystemFont,
    sans-serif;
  margin: 0;
  width: 100%;
  background: var(--bg-card);
  color: var(--text-secondary);
}

main {
  position: fixed;
  inset: 0;
  overflow-y: auto;
  overflow-x: hidden;
  -webkit-overflow-scrolling: touch;
  padding-top: var(--header-h);
  padding-bottom: var(--nav-h);
  background: var(--bg-base);
}
```

The layout strategy is important: `html` and `body` have `overflow: hidden` to prevent the document itself from scrolling. `main` is `position: fixed` filling the entire viewport, with padding at the top (for the header) and bottom (for the nav). Only `main` scrolls. This creates the "app shell" feel — the header and nav stay fixed, content scrolls between them.

### Step 3: Create the header styles

Create `style/_header.scss`:

```scss
.top-bar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border);
  padding: 0.6rem 1rem;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;

  &__flame {
    display: inline-block;
    width: 16px;
    height: 16px;
    vertical-align: -2px;
    margin: 0 1px;
    background-color: var(--flame);
    -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='black'%3E%3Cpath d='M13 2L3 14h9l-1 8 10-12h-9l1-8z'/%3E%3C/svg%3E");
    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='black'%3E%3Cpath d='M13 2L3 14h9l-1 8 10-12h-9l1-8z'/%3E%3C/svg%3E");
    -webkit-mask-size: contain;
    mask-size: contain;
    -webkit-mask-repeat: no-repeat;
    mask-repeat: no-repeat;
  }

  &__logo {
    font-family: "Russo One", sans-serif;
    font-size: 1.2rem;
    font-weight: 400;
    color: var(--accent);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
}
```

**The mask-image technique:** We render the lightning bolt icon using CSS `mask-image` with an inline SVG encoded as a data URI. Here is how it works:

1. The `<span>` has a `background-color` (gold, via `var(--flame)`)
2. The `mask-image` defines a shape — the lightning bolt SVG path
3. The browser only shows the background color where the mask is opaque

This produces a crisp, color-controllable icon without any `<svg>` element in the HTML.

> **Why not just use `<svg>` in the template?**
>
> Leptos renders HTML on the server (SSR) and then hydrates it in the browser (client-side). Inline SVGs have attributes like `viewBox`, `stroke-linecap`, and `xmlns` that can cause mismatches between the server-rendered HTML and the client-side hydration. When the HTML does not match exactly, Leptos throws hydration errors and the page may not become interactive.
>
> The `mask-image` approach avoids this entirely — the icon is pure CSS, so there is nothing to mismatch. It is the most reliable way to render icons in Leptos.

### Step 4: Wire up the main stylesheet

Replace the contents of `style/main.scss`:

```scss
// Theme variables (must come before everything else)
@use "themes";

// Base reset and layout
@use "reset";

// Components
@use "header";
```

SCSS `@use` imports each partial file (files prefixed with `_` are partials — they are not compiled on their own, only when imported). The order matters: themes must come first so the CSS custom properties are defined before any component references them.

Save all files. Your browser should reload and show the dark-themed header with "Grind⚡t" centered at the top.

<details>
<summary>Hint: If the page is completely unstyled</summary>

1. Verify that `style/main.scss` exists (not `styles/` or `css/`).
2. Check `Cargo.toml` — the `style-file` setting under `[package.metadata.leptos]` should be `style-file = "style/main.scss"`.
3. Make sure `<Stylesheet id="leptos" href="/pkg/gritwit.css"/>` is in your `App` component. This is the link to the compiled CSS.
4. Restart `cargo leptos watch` if the SCSS files were created while it was running — it sometimes misses new files.

</details>

---

## Exercise 4: Build the BottomNav

**Goal:** Add a fixed bottom navigation bar with 4 tabs: Home, Exercises, +Log, and History. Each tab has an icon (rendered with mask-image) and a label.

### Step 1: Add the BottomNav component

Open `src/app.rs` and add the `BottomNav` component after the `Header`:

```rust
#[component]
fn BottomNav() -> impl IntoView {
    view! {
        <nav class="bottom-nav">
            <a href="/" class="tab-item">
                <span class="tab-icon tab-icon--home"></span>
                <span class="tab-label">"Home"</span>
            </a>
            <a href="/exercises" class="tab-item">
                <span class="tab-icon tab-icon--exercises"></span>
                <span class="tab-label">"Exercises"</span>
            </a>
            <a href="/log" class="tab-item">
                <span class="tab-icon tab-icon--plus"></span>
                <span class="tab-label">"+Log"</span>
            </a>
            <a href="/history" class="tab-item">
                <span class="tab-icon tab-icon--history"></span>
                <span class="tab-label">"History"</span>
            </a>
        </nav>
    }
}
```

Update the `App` component to include it:

```rust
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/gritwit.css"/>
        <Title text="GrindIt"/>
        <Header/>
        <main>
            <p>"Welcome to GrindIt!"</p>
        </main>
        <BottomNav/>
    }
}
```

Notice how `<BottomNav/>` is used like an HTML tag. That is the `#[component]` macro at work — it transforms your Rust function into something the `view!` macro can render as a custom element.

### Step 2: Add the BottomNav styles

Create `style/_bottom_nav.scss`:

```scss
.bottom-nav {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  background: var(--bg-card);
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: space-around;
  align-items: center;
  padding: 0.4rem 0;
  z-index: 100;
}

.tab-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.15rem;
  text-decoration: none;
  color: var(--text-faint);
  flex: 1;
  padding: 0.25rem 0;
  transition: color 0.2s;
  -webkit-tap-highlight-color: transparent;

  &:hover,
  &:active {
    color: var(--text-muted);

    .tab-icon {
      background-color: var(--text-muted);
    }
  }
}

.tab-icon,
.tab-label {
  pointer-events: none;
}

.tab-icon {
  display: block;
  width: 24px;
  height: 24px;
  background-color: var(--text-faint);
  -webkit-mask-size: contain;
  mask-size: contain;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  -webkit-mask-position: center;
  mask-position: center;
}

// Home — house icon
.tab-icon--home {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'/%3E%3Cpolyline points='9 22 9 12 15 12 15 22'/%3E%3C/svg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z'/%3E%3Cpolyline points='9 22 9 12 15 12 15 22'/%3E%3C/svg%3E");
}

// Exercises — barbell icon
.tab-icon--exercises {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cline x1='2' y1='12' x2='22' y2='12'/%3E%3Crect x='4' y='8' width='3' height='8' rx='1'/%3E%3Crect x='17' y='8' width='3' height='8' rx='1'/%3E%3Crect x='1' y='10' width='2' height='4' rx='0.5'/%3E%3Crect x='21' y='10' width='2' height='4' rx='0.5'/%3E%3C/svg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cline x1='2' y1='12' x2='22' y2='12'/%3E%3Crect x='4' y='8' width='3' height='8' rx='1'/%3E%3Crect x='17' y='8' width='3' height='8' rx='1'/%3E%3Crect x='1' y='10' width='2' height='4' rx='0.5'/%3E%3Crect x='21' y='10' width='2' height='4' rx='0.5'/%3E%3C/svg%3E");
}

// +Log — plus icon
.tab-icon--plus {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cline x1='12' y1='5' x2='12' y2='19'/%3E%3Cline x1='5' y1='12' x2='19' y2='12'/%3E%3C/svg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.5' stroke-linecap='round' stroke-linejoin='round'%3E%3Cline x1='12' y1='5' x2='12' y2='19'/%3E%3Cline x1='5' y1='12' x2='19' y2='12'/%3E%3C/svg%3E");
}

// History — clock icon
.tab-icon--history {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='12' cy='12' r='10'/%3E%3Cpolyline points='12 6 12 12 16 14'/%3E%3C/svg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='12' cy='12' r='10'/%3E%3Cpolyline points='12 6 12 12 16 14'/%3E%3C/svg%3E");
}

.tab-label {
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0.02em;
}
```

Each icon class follows the same pattern:

1. The base `.tab-icon` class sets the size (24x24), background color, and mask sizing
2. Each modifier (`--home`, `--exercises`, `--plus`, `--history`) sets only the `mask-image` — the specific SVG shape
3. We include both `-webkit-mask-image` and `mask-image` for Safari compatibility

### Step 3: Wire up the new stylesheet

Update `style/main.scss` to include the bottom nav:

```scss
// Theme variables (must come before everything else)
@use "themes";

// Base reset and layout
@use "reset";

// Components
@use "header";
@use "bottom_nav";
```

Save everything. Your browser should now show:

- A dark background (`#0f0f1a`)
- An orange "Grind⚡t" logo centered in the header
- "Welcome to GrindIt!" text in the scrollable content area
- A bottom navigation bar with 4 icon+label tabs

The tabs do not navigate anywhere yet — that comes in Chapter 6 when we add routing. For now, they are styled anchor tags that form the visual shell of our app.

<details>
<summary>Hint: If the icons do not appear</summary>

The most common issue is a typo in the data URI SVG strings. These are URL-encoded SVGs — the `%3C` is `<`, `%3E` is `>`, `%20` is a space. If an icon is missing:

1. Check that both `-webkit-mask-image` and `mask-image` are present (Safari needs the webkit prefix)
2. Make sure `background-color` is set on `.tab-icon` — the mask needs a color to reveal
3. Verify the SVG data URI is well-formed — a missing `%3E` at the end will break it

</details>

---

## The Complete `app.rs`

Here is the full file after all exercises:

```rust
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/gritwit.css"/>
        <Title text="GrindIt"/>
        <Header/>
        <main>
            <p>"Welcome to GrindIt!"</p>
        </main>
        <BottomNav/>
    }
}

#[component]
fn Header() -> impl IntoView {
    view! {
        <header class="top-bar">
            <span class="top-bar__logo">
                "Grind"
                <span class="top-bar__flame"></span>
                "t"
            </span>
        </header>
    }
}

#[component]
fn BottomNav() -> impl IntoView {
    view! {
        <nav class="bottom-nav">
            <a href="/" class="tab-item">
                <span class="tab-icon tab-icon--home"></span>
                <span class="tab-label">"Home"</span>
            </a>
            <a href="/exercises" class="tab-item">
                <span class="tab-icon tab-icon--exercises"></span>
                <span class="tab-label">"Exercises"</span>
            </a>
            <a href="/log" class="tab-item">
                <span class="tab-icon tab-icon--plus"></span>
                <span class="tab-label">"+Log"</span>
            </a>
            <a href="/history" class="tab-item">
                <span class="tab-icon tab-icon--history"></span>
                <span class="tab-label">"History"</span>
            </a>
        </nav>
    }
}
```

---

## Rust Gym

Time for reps. These drills focus on variables and types — the spotlight concept for this chapter. Do them in a Rust playground ([play.rust-lang.org](https://play.rust-lang.org)) or in a scratch `main.rs`.

### Drill 1: Type Detective

What is the type of each variable? Predict first, then verify by uncommenting the print line.

```rust
fn main() {
    let a = 42;
    let b = "deadlift";
    let c = 3.14;
    let d = true;
    let e = 'A';
    let f = [1, 2, 3];

    // Uncomment one at a time to check your answers:
    // println!("{}", std::any::type_name_of_val(&a));
    // println!("{}", std::any::type_name_of_val(&b));
    // println!("{}", std::any::type_name_of_val(&c));
    // println!("{}", std::any::type_name_of_val(&d));
    // println!("{}", std::any::type_name_of_val(&e));
    // println!("{}", std::any::type_name_of_val(&f));
}
```

<details>
<summary>Solution</summary>

- `a`: `i32` (default integer type)
- `b`: `&str` (a string slice — all string literals are `&str`)
- `c`: `f64` (default float type)
- `d`: `bool`
- `e`: `char` (Rust `char` is a 4-byte Unicode scalar value, not a single byte like C)
- `f`: `[i32; 3]` (a fixed-size array of 3 `i32` values)

</details>

### Drill 2: Mutability Matters

This code does not compile. Fix it in two different ways.

```rust
fn main() {
    let reps = 10;
    reps = 12; // Coach says do 2 more
    println!("Do {} reps", reps);
}
```

<details>
<summary>Solution</summary>

**Fix 1: Make it mutable**

```rust
fn main() {
    let mut reps = 10;
    reps = 12;
    println!("Do {} reps", reps);
}
```

**Fix 2: Shadow it with a new binding**

```rust
fn main() {
    let reps = 10;
    let reps = 12; // this "shadows" the first reps — creates a new variable
    println!("Do {} reps", reps);
}
```

Shadowing is idiomatic in Rust. It creates a brand new variable that happens to have the same name. The original `reps` is gone (the compiler may even optimize it away). This is different from mutation — you can even change the type when shadowing:

```rust
let reps = "twelve";  // now it's a &str, not an i32
```

</details>

### Drill 3: String Ownership

What does this code print, and why? If it does not compile, explain the error and fix it.

```rust
fn main() {
    let exercise = String::from("Back Squat");
    let favorite = exercise;
    println!("My favorite: {}", exercise);
}
```

<details>
<summary>Solution</summary>

This code does not compile. The error:

```
error[E0382]: borrow of moved value: `exercise`
```

When you assign a `String` to another variable (`let favorite = exercise;`), the ownership **moves**. `exercise` is no longer valid — it has been "consumed." This is Rust's ownership system preventing double-free bugs (two variables trying to free the same heap memory).

**Fix 1: Clone it** (creates a deep copy)

```rust
let exercise = String::from("Back Squat");
let favorite = exercise.clone();
println!("My favorite: {}", exercise); // works — exercise still owns its data
```

**Fix 2: Borrow it** (take a reference instead of ownership)

```rust
let exercise = String::from("Back Squat");
let favorite = &exercise;
println!("My favorite: {}", exercise); // works — exercise was never moved
```

Note that `&str` string slices do not have this problem — they are references already and implement `Copy`, so assigning them just copies the reference (a pointer + length), not the underlying data:

```rust
let exercise = "Back Squat"; // &str — Copy
let favorite = exercise;     // copies the reference
println!("{}", exercise);    // still valid
```

This is why string literals work seamlessly in the `view!` macro. We will explore ownership and borrowing deeply in later chapters.

</details>

---

## System Design Corner: SSR vs CSR vs Hydration

You just built an app that uses **SSR with hydration**. In a system design interview, you might be asked: *"How would you architect the front-end for a fitness tracking app?"* Here is how to frame this discussion.

### Three rendering strategies

| Strategy | How it works | First paint | Interactivity | SEO |
|----------|-------------|-------------|---------------|-----|
| **CSR** (Client-Side Rendering) | Ship an empty HTML shell + a JS/WASM bundle. The browser runs the code and builds the DOM. | Slow (blank page until JS loads) | Fast once loaded | Poor (search engines see empty HTML) |
| **SSR** (Server-Side Rendering) | The server renders full HTML for every request. Each navigation is a new page load. | Fast | Slow (full page reloads) | Good |
| **SSR + Hydration** | The server renders full HTML for the first load. The browser loads JS/WASM and "hydrates" the existing DOM — attaching event handlers without re-rendering. | Fast | Fast (after hydration) | Good |

### What Leptos does

Leptos uses **SSR + hydration**. The server binary (compiled with the `ssr` feature) renders HTML. The WASM bundle (compiled with the `hydrate` feature) runs in the browser and makes the page interactive. The HTML the server produces and the DOM the client expects must match exactly — which is why inline SVGs (which can cause tiny attribute mismatches) are dangerous and why we use CSS mask-image instead.

### Key tradeoffs to discuss

- **Hydration cost:** The browser must download and execute the WASM bundle before the page becomes interactive. For GrindIt, this is a few hundred KB — acceptable for a fitness app used daily, but worth monitoring.
- **Time to Interactive (TTI):** The gap between when the user sees content (fast, thanks to SSR) and when they can interact with it (slower, waiting for hydration). For our app shell, this gap is small because the header and nav are mostly static. It matters more for pages with forms and buttons.
- **Complexity:** You maintain one codebase, but the same code must work in two environments (server and browser). The `#[cfg(feature = "ssr")]` and `#[cfg(feature = "hydrate")]` gates control what runs where. This is simpler than a separate backend + frontend, but it requires understanding the dual compilation model.

> **Interview talking point:** *"We chose SSR with hydration because our app needs fast initial loads on mobile (users open it in the gym, possibly on spotty WiFi), good SEO for public exercise pages, and rich interactivity for workout logging. Leptos compiles to WASM for the client, which gives us near-native performance for reactive UI updates without a virtual DOM diffing step."*

---

## What You Built

In this chapter, you:

1. **Installed the Leptos toolchain** — `rustup`, `wasm32-unknown-unknown` target, `cargo-leptos`
2. **Understood dual compilation** — the same code compiles to a server binary (SSR) and a WASM bundle (hydration)
3. **Built the `Header` component** — with the "Grind⚡t" logo using CSS mask-image for the icon
4. **Created a dark theme system** — CSS custom properties for colors, ready for light theme later
5. **Built the `BottomNav` component** — 4 tabs with CSS-only icons (home, barbell, plus, clock)
6. **Practiced variables and types** — `let` vs `let mut`, `String` vs `&str`, type inference, ownership

Your app now has the skeleton that every future page will live inside. In Chapter 2, we will fill the Exercises tab with data — building a static exercise library using `Vec`, structs, and Leptos's `For` component.

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/app.rs` | [`src/app.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/app.rs) (simplified — no router, no auth) |
| `style/_themes.scss` | [`style/_themes.scss`](https://github.com/sivakarasala/gritwit/blob/main/style/_themes.scss) |
| `style/_reset.scss` | [`style/_reset.scss`](https://github.com/sivakarasala/gritwit/blob/main/style/_reset.scss) |
| `style/_header.scss` | [`src/components/_header.scss`](https://github.com/sivakarasala/gritwit/blob/main/src/components/_header.scss) |
| `style/_bottom_nav.scss` | Bottom nav section of [`src/components/_header.scss`](https://github.com/sivakarasala/gritwit/blob/main/src/components/_header.scss) |
| `style/main.scss` | [`style/main.scss`](https://github.com/sivakarasala/gritwit/blob/main/style/main.scss) |
