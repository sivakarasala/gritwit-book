# Chapter 1: Hello, GrindIt!

You made it through Part 0. You can open a terminal, create files, and write Rust programs with variables, functions, and loops. That is a real foundation.

Now we take a big leap: you are going to build a **web page** — a real one, running in your browser, styled with a dark theme, and built entirely in Rust. Not HTML files opened from your desktop. A proper web application with a server, a client, and the architecture that professional apps use.

By the end of this chapter, you will have:

- A running web application at `localhost:3000`
- A dark-themed header with the "GrindIt" logo and a lightning bolt icon
- A bottom navigation bar with 4 tabs: Home, Exercises, +Log, History
- SCSS styling with CSS custom properties for theming
- A clear mental model of how Leptos compiles your code twice — once for the server, once for the browser

This is the skeleton that every future page of GrindIt will live inside. Let's build it.

---

## Spotlight: Variables, Types & the Leptos Toolchain

Every chapter in this book has one **spotlight concept** — the Rust idea we dig into deeply. This chapter's spotlight is **variables and types**, and how they interact with the Leptos build system.

You covered variables and types in Part 0. This section is a quick review, placing those concepts in the context of web development. If you feel confident, skim it. If anything feels fuzzy, read carefully — we are building on this foundation for the rest of the book.

### Variables: `let` bindings and immutability by default

In Rust, you declare variables with `let`. Variables are **immutable by default** — once set, they cannot be changed:

```rust
let name = "GrindIt";    // immutable — cannot be reassigned
let mut count = 0;        // mutable — can be reassigned
count += 1;               // OK
// name = "FitTrack";     // ERROR: cannot assign twice to immutable variable
```

You learned this in Part 0, and it will come up constantly. In the web app we are about to build, most of our variables are immutable — they hold text, configuration values, or component definitions that never change. When we get to interactive features (forms, counters, timers), we will introduce Leptos **signals**, which are the framework's way of handling values that change over time.

> **Programming Concept: Why Immutability Matters in Web Apps**
>
> Imagine a web page showing a user's name in the header, the sidebar, and the footer. If the name variable could be changed from anywhere in the code, a bug in the footer could silently corrupt the header. Immutability by default means you can look at where a variable is created and know *with certainty* what it contains — no need to search the entire codebase to see if something changed it later.

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

In Part 0, you practiced writing type annotations on every variable. As you get more comfortable, you will let the compiler infer types and only annotate when it helps clarity. Both styles are valid Rust.

### `String` vs `&str` — the two string types

This trips up every newcomer. Rust has two main string types:

- **`&str`** — a *string slice*. A read-only view into string data. Zero-cost, does not own the data. This is what string literals like `"hello"` produce.
- **`String`** — an *owned string*. Heap-allocated, growable, owned by the variable. You create one with `String::from("hello")` or `"hello".to_string()`.

```rust
let greeting: &str = "Hello";                    // borrowed, read-only
let owned_greeting: String = String::from("Hello"); // owned, can be modified
```

In the Leptos `view!` macro (which we will learn shortly), string literals are `&str` and that works fine. You will encounter `String` when dealing with data from databases, user input, or anywhere text needs to be owned and passed around.

> **Programming Concept: Ownership (First Glimpse)**
>
> In many languages (Python, JavaScript, Java), a "garbage collector" automatically frees memory you are no longer using. Rust does not have a garbage collector. Instead, every piece of data has exactly one **owner** — the variable that is responsible for it. When that variable goes out of scope, the data is freed.
>
> `String` is *owned* — the variable holds the data and is responsible for freeing it. `&str` is *borrowed* — it points to data owned by someone else. You do not need to fully understand ownership yet. Just know that the two string types exist because of this system, and the compiler will guide you when you use the wrong one.

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

> **Programming Concept: What is WebAssembly (WASM)?**
>
> Normally, web browsers run JavaScript. WebAssembly is a newer technology that lets browsers run code written in other languages — like Rust, C, or C++ — at near-native speed. When Leptos compiles your Rust code with the `hydrate` feature, it produces a `.wasm` file that the browser downloads and runs alongside the HTML. This is why you can write a web app entirely in Rust without writing a single line of JavaScript.

The server renders the initial HTML (fast first paint). The browser loads the WASM bundle and "hydrates" the page — attaching event handlers to the already-rendered HTML so clicks and interactions work.

This is why `cargo leptos watch` is the right command — not `cargo run`, which would only compile the server binary and skip the WASM half entirely.

Do not worry if dual compilation feels abstract right now. You will see it in action shortly, and it will become second nature as you build more features.

---

## Exercise 1: Set Up the Project

**Goal:** Install the Leptos toolchain, scaffold a new project, and see it running in your browser.

### Prerequisites

You need Rust installed. If you followed Part 0, you already have it. Verify:

```bash
rustc --version
cargo --version
```

Both commands should print version numbers. If not, install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Accept the defaults, then restart your terminal (or run `source ~/.cargo/env`).

### Step 1: Add the WASM compilation target

Leptos compiles part of your code to WebAssembly. Rust does not include the WASM target by default, so you need to add it:

```bash
rustup target add wasm32-unknown-unknown
```

> **Programming Concept: What is a Compilation Target?**
>
> When Rust compiles your code, it produces machine instructions for a specific platform. Your laptop uses one kind of instructions (x86 or ARM). A web browser's WebAssembly engine uses a different kind. The `wasm32-unknown-unknown` target tells Rust: "also learn how to produce WebAssembly instructions." This is a one-time setup — you will not need to run this command again.

### Step 2: Install cargo-leptos

`cargo-leptos` is the build tool that orchestrates the dual compilation — it compiles the server binary, the WASM bundle, and the SCSS stylesheets all in one command:

```bash
cargo install cargo-leptos
```

This may take a few minutes on first install. That is normal — it is compiling the tool itself from source.

### Step 3: Create the project

```bash
cd ~/rusty
cargo leptos new gritwit
```

When prompted, select **Axum** as the server framework.

> **Programming Concept: What is a Web Server?**
>
> A web server is a program that runs on a computer and listens for requests from web browsers. When you type `localhost:3000` in your browser, the browser sends a request to port 3000 on your own computer. The web server receives that request, builds an HTML page, and sends it back. The browser then displays that HTML.
>
> **Axum** is a web server framework written in Rust. It handles the networking details (listening for connections, parsing requests, sending responses) so you can focus on what your pages should contain. Other languages have their own frameworks: Python has Flask and Django, JavaScript has Express, Go has the standard library. Axum is the Rust equivalent.

Now enter the project directory:

```bash
cd gritwit
```

### Step 4: Run it

```bash
cargo leptos watch
```

The first build takes a while — sometimes several minutes. Leptos has many dependencies, and Rust compiles every one from source. This is a one-time cost; subsequent builds are much faster because Rust caches the compiled dependencies.

When you see output like:

```
listening on http://0.0.0.0:3000
```

Open [http://localhost:3000](http://localhost:3000) in your browser. You should see the default Leptos starter page — a simple page with a counter button.

**What you should see:** A web page with some default content and a working button. The exact content depends on the Leptos starter template version, but the important thing is that the page loads and the button works (try clicking it). If the button works, that means both the server (which rendered the HTML) and the client (the WASM bundle that makes the button interactive) are running correctly.

> **Why `cargo leptos watch` and not `cargo run`?**
>
> `cargo run` compiles and runs the server binary only. It does not compile the WASM client bundle, and it does not process SCSS into CSS. `cargo leptos watch` does all three — and it watches for file changes, recompiling automatically when you save. Think of it as your development command. You will leave it running in a terminal the entire time you are working on the app.

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

Let's walk through each file:

- **`Cargo.toml`** — The project configuration file. It lists dependencies (libraries your project uses), features (optional capabilities), and metadata that cargo-leptos needs.
- **`src/main.rs`** — The server's entry point. This file is only compiled when building the server binary (with the `ssr` feature). It starts the Axum web server.
- **`src/lib.rs`** — The library root. This is the code shared between the server and the client. It declares which modules (files) exist in your project.
- **`src/app.rs`** — Your Leptos components. This is where you will spend most of your time. The same code runs on both the server (to render HTML) and the client (to make the page interactive).
- **`style/main.scss`** — The stylesheet entry point. SCSS is a superset of CSS that adds features like variables, nesting, and imports. cargo-leptos compiles it into regular CSS.

Open `Cargo.toml` and study it. Here are the key sections:

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

There is a lot here. You do not need to memorize it. Here are the three things worth understanding now:

> **Programming Concept: What are Features in Cargo?**
>
> Features are optional capabilities you can turn on or off when compiling a Rust project. Think of them like switches. In GrindIt, the `ssr` feature switch says "compile this code for the server" and the `hydrate` feature switch says "compile this code for the browser." The `[features]` section in `Cargo.toml` defines what each switch enables.
>
> The `bin-features = ["ssr"]` and `lib-features = ["hydrate"]` lines in `[package.metadata.leptos]` tell cargo-leptos which switches to flip for which compilation target. You flip `ssr` for the server binary, `hydrate` for the WASM library.

- **`crate-type = ["cdylib", "rlib"]`** — This tells Rust to produce two kinds of output from your library code. `cdylib` is the format needed to create the WASM bundle. `rlib` is the standard Rust library format that the server binary links against. Two output types for two compilation targets.
- **`[features]`** — `ssr` and `hydrate` are mutually exclusive feature flags. They control which parts of Leptos are available in each compilation.
- **`style-file = "style/main.scss"`** — cargo-leptos compiles this SCSS file into CSS and serves it at `/pkg/gritwit.css`.

<details>
<summary>Hint: If the starter project layout differs from what's shown here</summary>

The `cargo leptos new` template may evolve over time. The important thing is that you have `src/app.rs` (or equivalent), `src/main.rs`, `src/lib.rs`, and `style/main.scss`. If files are named slightly differently, adjust the instructions accordingly. The concepts are the same.

</details>

---

## Exercise 2: Build the Header Component

**Goal:** Replace the starter page content with a `<Header>` component showing the GrindIt logo with a lightning bolt icon.

### What is a component?

Before we write code, let's understand what we are building.

> **Programming Concept: What is a Component?**
>
> A component is a reusable piece of a user interface. Think of it like a Lego brick. A **Header** component displays the app logo. A **BottomNav** component displays navigation tabs. A **Card** component displays a piece of content in a box.
>
> Components are functions that return a description of what should appear on screen. You build a page by combining components — just like you build a Lego model by snapping bricks together.
>
> In React (JavaScript), a component is a function that returns JSX. In Leptos (Rust), a component is a function that returns a `view!` macro call. The concept is the same.

### Step 1: Understand the shell

Open `src/app.rs`. You will see (or create) a `shell` function. This is the outer HTML document — the `<html>`, `<head>`, and `<body>` tags. It is rendered only on the server.

> **Programming Concept: What is HTML?**
>
> HTML (HyperText Markup Language) is the language that describes the *structure* of a web page. It uses **tags** — labels wrapped in angle brackets — to define elements:
>
> ```html
> <h1>This is a heading</h1>
> <p>This is a paragraph.</p>
> <div>This is a generic container.</div>
> ```
>
> Tags usually come in pairs: an opening tag (`<h1>`) and a closing tag (`</h1>`). Some tags are self-closing: `<meta charset="utf-8"/>`. Tags can be nested inside each other to create structure — a `<header>` might contain a `<span>`, which contains text.
>
> HTML gives a page its structure (headings, paragraphs, links, images). CSS gives it its appearance (colors, sizes, positions). We will write both.

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

That is a lot of new syntax. Let's break it down piece by piece.

### Step 2: Break down what you just wrote

**The `use` statements at the top:**

```rust
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
```

These are **imports** — they bring tools from the Leptos library into your file so you can use them. `use leptos::prelude::*` imports everything commonly needed from Leptos (the `*` means "everything in this module"). The second line imports specific items from `leptos_meta`, a companion library for managing the page's `<head>` section (title, stylesheets, meta tags).

Think of `use` like importing from a toolbox. You do not carry every tool to every job — you bring the ones you need.

**The `shell` function:**

```rust
pub fn shell(options: LeptosOptions) -> impl IntoView {
```

This is a plain function (not a component) that returns the HTML document skeleton. It takes `LeptosOptions` as an argument — this contains the configuration from `Cargo.toml` (site address, package directory, etc.).

Two things that may be new:
- **`pub`** means "public" — other files in the project can use this function. Without `pub`, a function is private to its own file.
- **`-> impl IntoView`** is the return type. It means "this function returns something that can be turned into HTML." You do not need to understand the `impl` keyword deeply right now — just read it as "returns a view."

Inside the shell, `AutoReload` injects a script that reloads the page during development. `HydrationScripts` injects the `<script>` tag that loads your WASM bundle. `<App/>` is where your actual application content goes.

**The `#[component]` attribute and `view!` macro:**

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
    }
}
```

> **Programming Concept: What is a Macro?**
>
> You have already used one macro: `println!`. Now you are meeting `view!`. In Rust, macros are special functions that end with `!`. They are more powerful than regular functions because they can generate code at compile time.
>
> `println!("Hello, {}", name)` generates the code needed to format and print text. `view!` generates the code needed to create HTML elements. You write what *looks like* HTML inside `view! { ... }`, and the macro transforms it into Rust code that builds the page.
>
> Why use a macro instead of a regular function? Because regular functions cannot accept HTML-like syntax as input. The macro lets you write familiar HTML structure while still getting all the safety and performance benefits of Rust.

The `#[component]` attribute (the part before the function) transforms a regular function into a Leptos component. Components must:

- Be named in **PascalCase** — `Header`, `App`, `BottomNav` (not `header`, `app`, `bottom_nav`). This is how Leptos distinguishes your components from regular HTML tags in the `view!` macro.
- Return `impl IntoView`
- Use the `view!` macro to describe their HTML output

The `view!` macro looks like HTML but has a few important differences:

1. **Strings must be quoted** — `"Welcome to GrindIt!"` not `Welcome to GrindIt!`. The macro needs to distinguish text content from Rust expressions.
2. **Self-closing tags need the slash** — `<meta charset="utf-8"/>` not `<meta charset="utf-8">`.
3. **`class` not `className`** — Leptos uses standard HTML attribute names.
4. **Components use PascalCase tags** — `<Header/>` renders your `Header` component. `<header>` renders a plain HTML `<header>` element.

**The `Header` component:**

```rust
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

This is your first custom component. It renders a `<header>` HTML element with the class `"top-bar"`. Inside it, a `<span>` contains the logo text: "Grind", then an empty `<span>` (which will become the lightning bolt icon via CSS), then "t". Together they read as "Grind⚡t" — the word "GrindIt" with a lightning bolt replacing the "I".

The `class` attribute assigns a **CSS class** to the element. We will use these class names in the next exercise to style everything.

<details>
<summary>Hint: If you see "unresolved import" errors</summary>

Make sure your `src/lib.rs` includes `pub mod app;` and that it re-exports the `shell` function if your `main.rs` references it. The starter template should have this wired up, but if you see import errors, check the module declarations.

</details>

Save the file. If `cargo leptos watch` is running, it will recompile automatically. The page will not look like much yet — just some plain text on a white background. We need the CSS to bring the dark theme to life.

---

## Exercise 3: Add SCSS Dark Theme

**Goal:** Create the dark theme with CSS custom properties, a browser reset, and layout rules that position the header at the top and navigation at the bottom with a scrollable main area between them.

> **Programming Concept: What is CSS?**
>
> CSS (Cascading Style Sheets) controls the *appearance* of a web page. If HTML is the skeleton (structure), CSS is the skin, clothing, and paint (style).
>
> ```css
> .top-bar {
>   background: #1a1a2e;    /* dark blue background */
>   color: #f97316;          /* orange text */
>   padding: 0.6rem 1rem;   /* space inside the element */
> }
> ```
>
> The `.top-bar` is a **selector** — it targets HTML elements with `class="top-bar"`. Inside the curly braces are **declarations** — each sets a property (like `background`) to a value (like `#1a1a2e`).
>
> **SCSS** is a superset of CSS — it adds features like variables, nesting, and file imports. cargo-leptos compiles SCSS into regular CSS that browsers understand.

### Step 1: Create the theme file

Create a new file at `style/_themes.scss`:

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

> **Programming Concept: What are CSS Custom Properties (CSS Variables)?**
>
> CSS custom properties are variables that live in your stylesheet. They start with `--` and can be used anywhere with `var(--name)`:
>
> ```css
> :root {
>   --bg-base: #0f0f1a;     /* define the variable */
> }
>
> body {
>   background: var(--bg-base);  /* use the variable */
> }
> ```
>
> Why not just write `#0f0f1a` everywhere? Because if you later want to change the background color, you would have to find and change it in every file. With a variable, you change it once (in the theme file) and every component that uses `var(--bg-base)` updates automatically.
>
> Later chapters will add a light theme by defining the same variable names with different values under `[data-theme="light"]`. The components will not need to change at all — they already reference the variables.

Notice the underscore in the filename: `_themes.scss`. In SCSS, files that start with `_` are called **partials**. They are not compiled on their own — they are meant to be imported by another file. This keeps things organized.

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

There is a lot going on here, and it is all worth understanding:

**`* { box-sizing: border-box; }`** — By default, CSS measures the width of an element *excluding* padding and borders. `border-box` changes this so width *includes* padding and borders. This makes layouts much more predictable. Almost every modern web project includes this rule.

**`html, body { height: 100%; overflow: hidden; }`** — We make the HTML document fill the entire screen and then *prevent it from scrolling*. This may seem strange — do we not want scrolling? Yes, but we want only the `main` content area to scroll, not the entire page. This creates the "app shell" feel where the header and navigation stay fixed.

**The `body` font stack** — `font-family` lists fonts in order of preference. The browser uses the first one it finds installed. "Inter" is a popular modern font, and the rest are fallbacks that are available on most operating systems.

**The `main` layout:**
- `position: fixed; inset: 0;` — Makes `main` fill the entire viewport (the visible browser area)
- `padding-top: var(--header-h);` — Leaves space at the top for the fixed header so content does not hide behind it
- `padding-bottom: var(--nav-h);` — Leaves space at the bottom for the fixed navigation bar
- `overflow-y: auto;` — Allows vertical scrolling *only within the main area*

The result: the header stays pinned to the top, the navigation stays pinned to the bottom, and content scrolls between them. This is the standard layout pattern for mobile apps.

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

Let's unpack the important parts:

**`position: fixed`** — This takes the header out of the normal page flow and pins it to the top of the screen. Even when the content scrolls, the header stays put. `top: 0; left: 0; right: 0;` means it stretches across the full width, flush against the top.

**`z-index: 100`** — When elements overlap (and our fixed header will overlap the scrolling content), `z-index` controls which one appears on top. A higher number means "closer to the viewer." We use 100 to make sure the header is always above the content.

**`display: flex; align-items: center; justify-content: center;`** — Flexbox is a CSS layout system for arranging elements in a row or column. These three lines center the logo both vertically and horizontally within the header bar.

**`&__flame` and `&__logo`** — The `&` is SCSS syntax for "the parent selector." So `&__flame` inside `.top-bar { ... }` becomes `.top-bar__flame` in the compiled CSS. This naming pattern (called **BEM** — Block, Element, Modifier) keeps CSS organized: `.top-bar` is the block, `__flame` and `__logo` are elements within that block.

**The mask-image technique — how the lightning bolt icon works:**

This is the most complex CSS in the file, so let's go through it carefully:

1. The `<span class="top-bar__flame">` is an empty HTML element — it has no text content
2. `background-color: var(--flame)` gives it a gold background color
3. `mask-image` defines a *shape* — the lightning bolt SVG path
4. The browser only shows the background color where the mask is opaque (where the SVG path is)
5. The result: a gold lightning bolt icon, with no actual image file

The long URL string (`data:image/svg+xml,...`) is an SVG image encoded directly into the CSS. The `%3C` characters are URL-encoded angle brackets (`<` and `>`). You do not need to write these from scratch — you can find SVG icons online and URL-encode them with a tool.

> **Why not just use `<svg>` tags in the HTML?**
>
> This is a Leptos-specific lesson. Leptos renders HTML on the server and then "hydrates" it in the browser. The server-rendered HTML and the browser's version must match *exactly*. Inline SVGs have attributes like `viewBox`, `stroke-linecap`, and `xmlns` that can cause tiny mismatches between the two. When they do not match, Leptos throws "hydration errors" and the page may not become interactive.
>
> The mask-image approach avoids this entirely — the icon is pure CSS, so there is nothing to mismatch. It is the most reliable way to render icons in Leptos.

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

`@use` imports each partial file. The order matters: themes must come first so the CSS custom properties are defined before any component tries to use them with `var(...)`.

Notice that you write `@use "themes"` not `@use "_themes.scss"` — SCSS automatically looks for files with the `_` prefix and `.scss` extension.

Save all files. Your browser should reload and show the dark-themed header with "Grind⚡t" centered at the top. The background should be a deep dark blue (`#0f0f1a`), the header bar a slightly lighter dark blue (`#1a1a2e`), and the logo text in orange with a gold lightning bolt.

**What you should see:** A dark page with an orange "Grind⚡t" logo centered in a header bar at the top. Below it, "Welcome to GrindIt!" in light gray text. The overall feel should be dark, modern, and app-like.

<details>
<summary>Hint: If the page is completely unstyled (white background, default font)</summary>

1. Verify that `style/main.scss` exists (not `styles/` or `css/`).
2. Check `Cargo.toml` — the `style-file` setting under `[package.metadata.leptos]` should be `style-file = "style/main.scss"`.
3. Make sure `<Stylesheet id="leptos" href="/pkg/gritwit.css"/>` is in your `App` component. This is the link to the compiled CSS.
4. Restart `cargo leptos watch` if the SCSS files were created while it was running — it sometimes misses new files.

</details>

<details>
<summary>Hint: If the lightning bolt icon does not appear</summary>

The most likely issue is that the `_header.scss` file was not saved correctly. Double-check that the `&__flame` block is inside the `.top-bar` block (not after it). Also verify that `@use "header";` appears in `main.scss`.

</details>

---

## Exercise 4: Build the BottomNav

**Goal:** Add a fixed bottom navigation bar with 4 tabs: Home, Exercises, +Log, and History. Each tab has an icon (rendered with mask-image) and a label.

### Step 1: Add the BottomNav component

Open `src/app.rs` and add the `BottomNav` component. You can place it after the `Header` component — in Rust, the order of function definitions does not matter:

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

This component creates a `<nav>` element (the HTML tag for navigation) with four `<a>` (anchor/link) elements inside it. Each link has:

- An `href` — the URL it points to (`/`, `/exercises`, `/log`, `/history`). These URLs do not work yet; we will add routing in Chapter 6.
- A `class="tab-item"` — for CSS styling.
- Two `<span>` children: one for the icon and one for the text label.

Notice the icon spans have *two* classes: `tab-icon` (the base class with shared sizing) and `tab-icon--home` (the modifier class with the specific icon shape). This is the BEM pattern again: the base class handles what all icons share, and the modifier handles what is unique.

### Step 2: Update the App component

Update the `App` component to include `<BottomNav/>`:

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

Notice how `<BottomNav/>` is used like an HTML tag. That is the `#[component]` macro at work — it transforms your Rust function into something the `view!` macro can render. The PascalCase name (`BottomNav` not `bottom_nav`) is what tells the `view!` macro "this is a Leptos component, not a regular HTML tag."

### Step 3: Add the BottomNav styles

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

The icon classes follow the same mask-image pattern you learned in Exercise 3. Each modifier class (`--home`, `--exercises`, `--plus`, `--history`) sets only the `mask-image` — the specific SVG shape. The base `.tab-icon` class handles everything else: size (24x24), background color, and mask sizing. We include both `-webkit-mask-image` and `mask-image` for Safari compatibility.

A few styling details worth noting:

- **`display: flex; flex-direction: column;`** on `.tab-item` — This stacks the icon above the label vertically.
- **`justify-content: space-around;`** on `.bottom-nav` — This distributes the four tabs evenly across the full width.
- **`transition: color 0.2s;`** — This makes the color change smooth when hovering or tapping, instead of an instant jump.
- **`-webkit-tap-highlight-color: transparent;`** — On mobile, tapping a link usually shows a blue/gray highlight rectangle. This removes it for a cleaner look.

### Step 4: Wire up the new stylesheet

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

**What you should see:** The complete app shell — dark themed, with a header at the top and four navigation tabs at the bottom (Home, Exercises, +Log, History). Each tab has a small icon above its label. The icons are gray and slightly brighten when you hover over them.

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

Here is the full file after all exercises. If something is not working, compare your file against this:

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

Time for reps. These drills reinforce the spotlight concept for this chapter — variables and types — and give you hands-on practice modifying the app you just built. Do the first three in a Rust playground ([play.rust-lang.org](https://play.rust-lang.org)) or in a scratch `main.rs`. Do the last two directly in your GrindIt project.

### Drill 1: Change the App Name

Open `src/app.rs` in your GrindIt project. Change the logo text from "Grind⚡t" to something else — maybe "Fit⚡t" or "Iron⚡t". You need to change the string literals `"Grind"` and `"t"` inside the `Header` component.

Save the file and watch the browser reload. Congratulations — you just modified a live web application.

Now change it back to "Grind" and "t" before moving on.

### Drill 2: Add a Fifth Tab

Add a "Profile" tab to the bottom navigation. You will need to:

1. Add another `<a>` element inside the `<nav>` in the `BottomNav` component
2. Use `href="/profile"` and the label `"Profile"`
3. For the icon, reuse an existing icon class (like `tab-icon--home`) for now

Save and check the browser. You should see five tabs evenly spaced across the bottom. The layout adjusts automatically because of `justify-content: space-around`.

When you are done experimenting, remove the fifth tab to match the reference implementation.

### Drill 3: Modify a Theme Color

Open `style/_themes.scss`. Change `--accent: #f97316;` (orange) to a different color. Try:

- `#22c55e` (green)
- `#3b82f6` (blue)
- `#ef4444` (red)

Save and watch the header logo change color instantly. This demonstrates the power of CSS custom properties — one change, and everything that references `var(--accent)` updates.

Change it back to `#f97316` when you are done.

### Drill 4: Type Detective

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

### Drill 5: Mutability Matters

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

---

## System Design Corner: SSR vs CSR

You just built an app that uses **SSR with hydration**. Let's understand what that means in simple terms.

> **Programming Concept: Three Ways to Build a Web Page**
>
> When you visit a website, your browser needs to get HTML to display. There are three main strategies for how that HTML gets created:

### Strategy 1: Client-Side Rendering (CSR)

The server sends a nearly empty HTML page plus a big JavaScript (or WASM) file. The browser downloads that file, runs it, and the code builds the page content. Until the code finishes running, you see a blank screen.

**Analogy:** You order a piece of furniture, but the store ships you a box of parts and an instruction manual. Nothing is assembled until you (the browser) put it together yourself. You wait, staring at a pile of wood, until it is done.

### Strategy 2: Server-Side Rendering (SSR)

The server builds the complete HTML page and sends it to the browser ready to display. Every time you click a link, the browser asks the server for a brand new page.

**Analogy:** You order furniture and the store delivers it fully assembled. Instant gratification. But every time you want to rearrange a room, you have to call the store and wait for a new delivery.

### Strategy 3: SSR + Hydration (what Leptos does)

The server builds the complete HTML and sends it to the browser (fast first display). Then the browser downloads the WASM bundle and "hydrates" the page — connecting interactive behavior to the already-visible elements. The page appears instantly, and becomes interactive shortly after.

**Analogy:** The store delivers your furniture fully assembled *and* includes a toolkit. You can see and use the furniture immediately. Once you open the toolkit, you can also rearrange, modify, and interact with everything without calling the store again.

### Why this matters for GrindIt

| Strategy | First paint | Interactivity | Good for GrindIt? |
|----------|------------|---------------|--------------------|
| **CSR** | Slow (blank screen) | Fast once loaded | No — users open the app in the gym, possibly on spotty WiFi. A blank screen is a bad experience. |
| **SSR** | Fast | Slow (full page reloads) | Partially — fast initial load, but clunky navigation. |
| **SSR + Hydration** | Fast | Fast (after hydration) | Yes — fast load, smooth interactivity, and it works with poor connectivity. |

The key tradeoff: the browser must download the WASM bundle before the page becomes fully interactive. For GrindIt, this is a few hundred kilobytes — acceptable for a fitness app used daily. The server-rendered HTML means the user sees content immediately, even before the WASM finishes loading.

---

## What You Built

Take a moment to appreciate what you accomplished. You just:

1. **Installed the Leptos toolchain** — `rustup`, `wasm32-unknown-unknown` target, `cargo-leptos`
2. **Understood dual compilation** — the same code compiles to a server binary (SSR) and a WASM bundle (hydration)
3. **Built the `Header` component** — with the "Grind⚡t" logo using CSS mask-image for the icon
4. **Created a dark theme system** — CSS custom properties for colors, ready for a light theme later
5. **Built the `BottomNav` component** — 4 tabs with CSS-only icons (home, barbell, plus, clock)
6. **Practiced variables, types, and hands-on modification** — changing colors, adding tabs, understanding type inference

You wrote a Rust function, and it became a web page in your browser. You wrote SCSS, and it became a dark theme with icons. You learned about components, macros, CSS custom properties, flexbox, fixed positioning, and the mask-image technique.

That is a lot for one chapter. And it is a real foundation — every single page you build from now on will live inside this shell. The header and bottom nav will be there on every screen.

In Chapter 2, we will fill the Exercises tab with data — building a static exercise library using `Vec`, structs, and Leptos's `For` component. The app is about to start feeling real.

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
