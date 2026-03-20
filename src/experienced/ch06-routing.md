# Chapter 6: Multi-Page Routing

Until now, GrindIt has been a single page. The exercises page fills `<main>`, and the bottom nav tabs are dead links. This chapter brings the app to life with multiple pages: Home, Exercises, Log, History, and stub pages for Login and Profile. Each gets its own URL, the bottom nav highlights the active tab, and navigating between pages feels instant.

The spotlight concept is **modules and project structure** — how Rust organizes code into files, controls visibility, and uses conditional compilation to separate server-only code from browser code. This is the chapter where your single `app.rs` file explodes into a proper project structure.

By the end of this chapter, you will have:

- A `src/pages/` directory with separate modules for each page
- `leptos_router` configured with `<Router>`, `<Routes>`, and `<Route>`
- Active tab highlighting using `use_location().pathname`
- A scroll-to-top reset on every route change
- A clear understanding of Rust's module system, visibility rules, and feature flags

---

## Spotlight: Modules & Project Structure

### The problem

Your `app.rs` file has been growing. It contains the shell, the App component, the Header, the BottomNav, the ExercisesPage, the ExerciseFormPanel, the ConfirmModal, the toast system, and server functions. In JavaScript, you would split this into separate files with `import`/`export`. Rust has its own system.

### `mod` — declaring a module

In Rust, the file system is the module tree. To declare that a file is part of your crate, you use `mod`:

```rust
// src/lib.rs
pub mod app;         // loads src/app.rs
pub mod db;          // loads src/db.rs
pub mod pages;       // loads src/pages/mod.rs (or src/pages.rs)
pub mod validation;  // loads src/validation.rs
```

Each `mod` declaration tells the compiler: "there is a module with this name — go find its file." The compiler looks for either `src/{name}.rs` or `src/{name}/mod.rs`. Both work. The convention is:

- **`src/foo.rs`** — when the module is a single file
- **`src/foo/mod.rs`** — when the module has sub-modules (child files in the `foo/` directory)

### `pub` — visibility

Everything in Rust is private by default. To make something visible outside its module, you add `pub`:

```rust
// src/pages/exercises/mod.rs
pub fn ExercisesPage() { ... }     // visible to anyone who can see this module
fn helper_function() { ... }       // private — only usable inside this file
```

Rust has finer-grained visibility modifiers:

| Modifier | Visible to |
|----------|-----------|
| *(none)* | Only the current module and its children |
| `pub` | Everyone — any crate that depends on this one |
| `pub(crate)` | Anything within this crate, but not external crates |
| `pub(super)` | The parent module only |

For GrindIt, `pub(crate)` is the most common. Our pages should be visible within the `gritwit` crate (so `app.rs` can import them) but not to external crates (nobody else depends on us).

### `use` — bringing names into scope

`mod` declares a module. `use` brings items from a module into the current scope:

```rust
// Without `use`:
crate::pages::exercises::ExercisesPage

// With `use`:
use crate::pages::exercises::ExercisesPage;
ExercisesPage  // much shorter
```

The `crate::` prefix means "start from the root of this crate." You can also use:
- `super::` — the parent module
- `self::` — the current module (rarely needed)

### Re-exports

A common pattern is to re-export items from child modules so the parent presents a clean interface:

```rust
// src/pages/mod.rs
mod exercises;
mod history;
mod home;

// Re-export the page components so callers use `pages::ExercisesPage`
// instead of `pages::exercises::ExercisesPage`
pub use exercises::ExercisesPage;
pub use history::HistoryPage;
pub use home::HomePage;
```

This is the same principle as an `index.ts` that re-exports from subdirectories:

```typescript
// pages/index.ts (JavaScript equivalent)
export { ExercisesPage } from './exercises';
export { HistoryPage } from './history';
export { HomePage } from './home';
```

### Conditional compilation: `#[cfg(feature = "...")]`

Leptos compiles your code twice — once for the server (`ssr`), once for the browser (`hydrate`). Some code should only exist on one side:

```rust
// Only compiled into the server binary
#[cfg(feature = "ssr")]
pub mod routes;

// Only compiled into the WASM bundle
#[cfg(feature = "hydrate")]
pub mod pwa;

// Compiled in both, but with different behavior
pub mod db;  // the struct definitions are shared; the query functions are #[cfg(feature = "ssr")]
```

You have already seen `#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]` — which conditionally applies an attribute. The broader `#[cfg(feature = "ssr")]` removes the entire item from compilation when the feature is not active.

This is more powerful than JavaScript's tree-shaking. Tree-shaking removes *unused* code after bundling. Rust's conditional compilation never compiles the code in the first place — it does not exist in the binary, cannot be imported, and the compiler does not even type-check it for the disabled feature.

> **Coming from JS?**
>
> | Concept | JavaScript | Rust |
> |---------|-----------|------|
> | Define a module | Create a file | Create a file + `mod` declaration in parent |
> | Export from a module | `export function f() {}` | `pub fn f() {}` |
> | Import into scope | `import { f } from './module'` | `use crate::module::f;` |
> | Re-export | `export { f } from './module'` | `pub use module::f;` |
> | Barrel file | `index.ts` with re-exports | `mod.rs` with `pub use` re-exports |
> | Conditional code | `if (typeof window !== 'undefined')` | `#[cfg(feature = "hydrate")]` |
>
> The biggest difference: in JavaScript, importing a file automatically makes it part of the bundle. In Rust, a file does nothing until a `mod` statement claims it. If you create `src/unused.rs` but never write `mod unused;` anywhere, the compiler ignores it completely.

---

## Exercise 1: Reorganize into a Module Structure

**Goal:** Move the ExercisesPage into `src/pages/exercises/mod.rs` and create stub pages for Home, Log, History, Login, and Profile.

### Step 1: Create the directory structure

```
src/
├── app.rs
├── db.rs
├── lib.rs
├── main.rs
├── validation.rs
└── pages/
    ├── mod.rs
    ├── exercises/
    │   └── mod.rs
    ├── home.rs
    ├── history.rs
    ├── log_workout.rs
    ├── login.rs
    └── profile.rs
```

### Step 2: Move the ExercisesPage

Cut the `ExercisesPage` component (and its related server functions, form component, etc.) from `src/app.rs` and paste it into `src/pages/exercises/mod.rs`:

```rust
// src/pages/exercises/mod.rs
use crate::db::Exercise;
use leptos::prelude::*;
use std::collections::HashMap;

// Server functions
#[server]
pub async fn list_exercises() -> Result<Vec<Exercise>, ServerFnError> {
    let pool = crate::db::db().await?;
    crate::db::list_exercises_db(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn create_exercise(
    name: String,
    category: String,
    scoring_type: String,
) -> Result<(), ServerFnError> {
    if name.trim().is_empty() {
        return Err(ServerFnError::new("Name cannot be empty"));
    }
    let pool = crate::db::db().await?;
    crate::db::create_exercise_db(&pool, &name, &category, &scoring_type)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn delete_exercise(id: String) -> Result<(), ServerFnError> {
    let pool = crate::db::db().await?;
    let uuid: uuid::Uuid = id
        .parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
    crate::db::delete_exercise_db(&pool, uuid)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

// Helper functions
pub fn category_color(cat: &str) -> &'static str {
    match cat {
        "weightlifting" => "#3498db",
        "gymnastics"    => "#9b59b6",
        "conditioning"  => "#e74c3c",
        "cardio"        => "#e67e22",
        "mobility"      => "#1abc9c",
        _               => "#888888",
    }
}

const CATEGORY_ORDER: &[(&str, &str)] = &[
    ("weightlifting", "Weightlifting"),
    ("gymnastics", "Gymnastics"),
    ("conditioning", "Conditioning"),
    ("cardio", "Cardio"),
    ("mobility", "Mobility"),
];

// The main page component
#[component]
pub fn ExercisesPage() -> impl IntoView {
    let create_action = ServerAction::<CreateExercise>::new();
    let delete_action = ServerAction::<DeleteExercise>::new();

    let exercises = Resource::new(
        move || (
            create_action.version().get(),
            delete_action.version().get(),
        ),
        |_| list_exercises(),
    );

    let search = RwSignal::new(String::new());
    let show_form = RwSignal::new(false);

    view! {
        <div class="exercises-page">
            <button
                class=move || if show_form.get() { "fab fab--active" } else { "fab" }
                on:click=move |_| show_form.update(|v| *v = !*v)
            >
                <span class="fab-icon"></span>
            </button>

            // Form and exercise list as built in Chapters 3-5
            // (abbreviated for space — your full implementation goes here)

            <Suspense fallback=|| view! { <p class="loading">"Loading exercises..."</p> }>
                {move || {
                    exercises.get().map(|result| {
                        match result {
                            Ok(list) => {
                                view! { <p>{format!("{} exercises loaded", list.len())}</p> }.into_any()
                            }
                            Err(e) => view! { <p class="error">{format!("Error: {}", e)}</p> }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
```

### Step 3: Create the stub pages

Each stub page is a placeholder that will be filled in later chapters.

`src/pages/home.rs`:

```rust
use leptos::prelude::*;

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <div class="home-page">
            <h2>"Welcome to GrindIt"</h2>
            <p>"Today's workout will appear here."</p>
        </div>
    }
}
```

`src/pages/log_workout.rs`:

```rust
use leptos::prelude::*;

#[component]
pub fn LogWorkoutPage() -> impl IntoView {
    view! {
        <div class="log-page">
            <h2>"Log Workout"</h2>
            <p>"Workout logging form coming in Chapter 9."</p>
        </div>
    }
}
```

`src/pages/history.rs`:

```rust
use leptos::prelude::*;

#[component]
pub fn HistoryPage() -> impl IntoView {
    view! {
        <div class="history-page">
            <h2>"History"</h2>
            <p>"Your workout timeline coming in Chapter 10."</p>
        </div>
    }
}
```

`src/pages/login.rs`:

```rust
use leptos::prelude::*;

#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="login-page">
            <h2>"Log In"</h2>
            <p>"Authentication coming in Chapter 7."</p>
        </div>
    }
}
```

`src/pages/profile.rs`:

```rust
use leptos::prelude::*;

#[component]
pub fn ProfilePage() -> impl IntoView {
    view! {
        <div class="profile-page">
            <h2>"Profile"</h2>
            <p>"Profile settings coming in Chapter 12."</p>
        </div>
    }
}
```

### Step 4: Create the pages module with re-exports

`src/pages/mod.rs`:

```rust
mod exercises;
mod history;
mod home;
mod log_workout;
mod login;
mod profile;

pub use exercises::ExercisesPage;
pub use history::HistoryPage;
pub use home::HomePage;
pub use log_workout::LogWorkoutPage;
pub use login::LoginPage;
pub use profile::ProfilePage;
```

This is the re-export pattern. Callers write `use crate::pages::ExercisesPage` — they do not need to know that `ExercisesPage` lives inside an `exercises` subdirectory.

### Step 5: Update `lib.rs`

```rust
pub mod app;
pub mod db;
pub mod pages;
pub mod validation;
```

The `pages` module declaration tells the compiler to look for `src/pages/mod.rs`. That file in turn declares its child modules (`exercises`, `home`, etc.).

### Step 6: Verify it compiles

```bash
cargo leptos watch
```

If you see import errors in `app.rs`, remove the old `ExercisesPage` component and server function definitions that you moved to `pages/exercises/mod.rs`. Update the imports:

```rust
// src/app.rs
use crate::pages::{ExercisesPage, HistoryPage, HomePage, LogWorkoutPage, LoginPage, ProfilePage};
```

<details>
<summary>Hint: If you see "file not found for module"</summary>

Rust looks for module files relative to the declaring file:
- `mod exercises;` in `src/pages/mod.rs` looks for `src/pages/exercises.rs` or `src/pages/exercises/mod.rs`
- `mod pages;` in `src/lib.rs` looks for `src/pages.rs` or `src/pages/mod.rs`

If you created the file in the wrong directory, the compiler will say "file not found for module." Check that the directory structure matches the `mod` declarations exactly.

</details>

---

## Exercise 2: Set Up the Router

**Goal:** Configure `leptos_router` so each page gets its own URL and the correct component renders for each route.

### Step 1: Add the router dependency

Add to `Cargo.toml`:

```toml
[dependencies]
leptos_router = { version = "0.8" }
```

And add it to both features:

```toml
[features]
ssr = ["leptos/ssr", "leptos_meta/ssr", "leptos_router/ssr", "dep:sqlx", "dep:tokio"]
hydrate = ["leptos/hydrate", "leptos_router/hydrate", "dep:console_error_panic_hook", "dep:wasm-bindgen"]
```

### Step 2: Add the Router to App

Update `src/app.rs`:

```rust
use crate::pages::{
    ExercisesPage, HistoryPage, HomePage,
    LogWorkoutPage, LoginPage, ProfilePage,
};
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

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

        <Router>
            <Header/>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=StaticSegment("exercises") view=ExercisesPage/>
                    <Route path=StaticSegment("log") view=LogWorkoutPage/>
                    <Route path=StaticSegment("history") view=HistoryPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <Route path=StaticSegment("profile") view=ProfilePage/>
                </Routes>
            </main>
            <BottomNav/>
        </Router>
    }
}
```

### Step 3: Understand the routing model

**`<Router>`** wraps your entire app. It provides the routing context that all child components can access. Everything that needs routing — including the `BottomNav` (for active tab highlighting) — must be inside `<Router>`.

**`<Routes>`** is the switch. It looks at the current URL and renders the matching `<Route>`. The `fallback` prop defines what to show when no route matches (a 404 page).

**`<Route path=StaticSegment("exercises") view=ExercisesPage/>`** means: when the URL is `/exercises`, render the `ExercisesPage` component. `StaticSegment` matches a literal path segment.

Leptos's router works on both server and client:
- **On the server (SSR):** The router reads the request URL and renders the matching component to HTML.
- **On the client (after hydration):** The router intercepts link clicks, updates the URL with the History API, and swaps the component — no full page reload.

This is why navigation between tabs feels instant after the initial load. The WASM bundle is already loaded, and switching from `/exercises` to `/history` just swaps the component inside `<main>`.

### Step 4: Update navigation links

Your `BottomNav` already uses `<a href="/exercises">`. With the router, these links are intercepted on the client side. No changes needed to the HTML — the router handles it.

However, for client-side routing to work correctly, you should use the Leptos `<A>` component instead of raw `<a>` tags. The `<A>` component integrates with the router to prevent full page reloads:

```rust
use leptos_router::components::A;

#[component]
fn BottomNav() -> impl IntoView {
    view! {
        <nav class="bottom-nav">
            <A href="/" class="tab-item">
                <span class="tab-icon tab-icon--home"></span>
                <span class="tab-label">"Home"</span>
            </A>
            <A href="/exercises" class="tab-item">
                <span class="tab-icon tab-icon--exercises"></span>
                <span class="tab-label">"Exercises"</span>
            </A>
            <A href="/log" class="tab-item">
                <span class="tab-icon tab-icon--plus"></span>
                <span class="tab-label">"Log"</span>
            </A>
            <A href="/history" class="tab-item">
                <span class="tab-icon tab-icon--history"></span>
                <span class="tab-label">"History"</span>
            </A>
        </nav>
    }
}
```

In practice, Leptos intercepts `<a>` tags inside a `<Router>` context too, so both work. The `<A>` component is explicit about intent and adds some extras like automatic `aria-current` on the active link.

Test it: navigate between tabs. Each URL should show a different page content, and the browser's back/forward buttons should work.

<details>
<summary>Hint: If clicking a tab causes a full page reload</summary>

Make sure the `<BottomNav/>` component is rendered *inside* the `<Router>` component, not outside it. If it is outside, the router cannot intercept the link clicks. Also ensure that `leptos_router` is included in both the `ssr` and `hydrate` features in `Cargo.toml` — the router needs to work in both environments.

</details>

---

## Exercise 3: Active Tab Highlighting

**Goal:** Make the BottomNav highlight the tab that matches the current URL.

### Step 1: Use `use_location()`

Leptos Router provides `use_location()` which gives you reactive access to the current URL. The `.pathname` field is a `Signal<String>` that updates whenever the route changes:

```rust
use leptos_router::hooks::use_location;

#[component]
fn BottomNav() -> impl IntoView {
    let pathname = use_location().pathname;

    view! {
        <nav class="bottom-nav">
            <a
                href="/"
                class="tab-item"
                class:active=move || pathname.get() == "/"
            >
                <span class="tab-icon tab-icon--home"></span>
                <span class="tab-label">"Home"</span>
            </a>
            <a
                href="/exercises"
                class="tab-item"
                class:active=move || pathname.get().starts_with("/exercises")
            >
                <span class="tab-icon tab-icon--exercises"></span>
                <span class="tab-label">"Exercises"</span>
            </a>
            <a
                href="/log"
                class="tab-item"
                class:active=move || pathname.get().starts_with("/log")
            >
                <span class="tab-icon tab-icon--plus"></span>
                <span class="tab-label">"Log"</span>
            </a>
            <a
                href="/history"
                class="tab-item"
                class:active=move || pathname.get().starts_with("/history")
            >
                <span class="tab-icon tab-icon--history"></span>
                <span class="tab-label">"History"</span>
            </a>
        </nav>
    }
}
```

### Step 2: Understand `class:active`

The `class:active=move || expr` syntax is Leptos's way of conditionally applying a CSS class. When the closure returns `true`, the `active` class is added to the element. When it returns `false`, it is removed.

For the Home tab, we check exact equality: `pathname.get() == "/"`. For other tabs, we use `.starts_with()` so that sub-routes (like a hypothetical `/exercises/123`) still highlight the Exercises tab.

This is reactive — `pathname` is a signal. When you navigate from `/exercises` to `/history`, the closure for Exercises returns `false` (removing `active`) and the closure for History returns `true` (adding `active`). No manual DOM manipulation needed.

### Step 3: Add the active tab styles

Update `style/_bottom_nav.scss`:

```scss
.tab-item {
  // ... existing styles ...

  &.active {
    color: var(--accent);

    .tab-icon {
      background-color: var(--accent);
    }

    .tab-label {
      color: var(--accent);
    }
  }
}
```

Save and test. As you tap each tab, the icon and label should turn orange (the accent color), while the other tabs remain dim.

<details>
<summary>Hint: If the active class is not being applied</summary>

Check that `use_location()` is being called inside the `<Router>` context. If `BottomNav` is rendered outside `<Router>`, `use_location()` will panic or return a default value. Also verify that the `pathname` check matches your route definitions — `"/exercises"` not `"/exercises/"` (trailing slashes matter).

</details>

---

## Exercise 4: Add ScrollReset on Route Change

**Goal:** When navigating to a new page, scroll the `<main>` element back to the top. Without this, scrolling down on the Exercises page and then navigating to History would show History scrolled to the same position.

### Step 1: Create the ScrollReset component

```rust
use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
fn ScrollReset() -> impl IntoView {
    let pathname = use_location().pathname;

    Effect::new(move |_| {
        // Read the pathname to subscribe to route changes
        let _ = pathname.get();

        // Scroll main to top (only runs in the browser)
        #[cfg(feature = "hydrate")]
        {
            let _ = js_sys::eval(
                "var m=document.querySelector('main');\
                 if(m){m.scrollTo({top:0,behavior:'instant'})}"
            );
        }
    });

    // This component renders nothing — it is a side-effect-only component
}
```

### Step 2: Understand `Effect::new`

`Effect::new` creates a reactive side effect. It runs once immediately and then re-runs whenever any signal it reads changes. Here, it reads `pathname.get()`, so it re-runs on every route change.

The effect body scrolls `<main>` to the top. Since our `<main>` is `position: fixed; overflow-y: auto;`, calling `scrollTo({top: 0})` on it resets the scroll position.

The `#[cfg(feature = "hydrate")]` gate ensures this only runs in the browser. On the server (during SSR), there is no DOM to scroll — and `js_sys::eval` does not exist.

**Why `js_sys::eval` instead of a Rust DOM API?** Leptos provides DOM access through `web_sys`, but for a simple one-liner like scrolling an element, `js_sys::eval` is shorter and avoids importing multiple `web_sys` types. The reference GrindIt app uses this approach. For more complex DOM interactions, you would use `web_sys` for type safety.

### Step 3: Add it to the App

Place `<ScrollReset/>` inside `<Router>` but outside `<Routes>`:

```rust
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/gritwit.css"/>
        <Title text="GrindIt"/>

        <Router>
            <ScrollReset/>
            <Header/>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage/>
                    <Route path=StaticSegment("exercises") view=ExercisesPage/>
                    <Route path=StaticSegment("log") view=LogWorkoutPage/>
                    <Route path=StaticSegment("history") view=HistoryPage/>
                    <Route path=StaticSegment("login") view=LoginPage/>
                    <Route path=StaticSegment("profile") view=ProfilePage/>
                </Routes>
            </main>
            <BottomNav/>
        </Router>
    }
}
```

Test it: scroll down on the Exercises page, tap History, and the view should start at the top.

<details>
<summary>Hint: If the scroll does not reset</summary>

Verify that your `<main>` element has `overflow-y: auto` in CSS. If `<main>` does not scroll (if the body scrolls instead), the `document.querySelector('main').scrollTo()` call will have no effect. Also check that `ScrollReset` is inside `<Router>` — it needs access to `use_location()`.

</details>

---

## Rust Gym

### Drill 1: Module Visibility

Given this file structure:

```
src/
├── lib.rs
├── db.rs
└── pages/
    ├── mod.rs
    └── exercises/
        ├── mod.rs
        ├── helpers.rs
        └── server_fns.rs
```

For each of the following, determine if the access is valid. If not, what visibility modifier is needed?

```rust
// 1. In src/pages/exercises/mod.rs:
use crate::db::Exercise;  // Exercise is pub in db.rs

// 2. In src/pages/exercises/server_fns.rs:
use super::helpers::category_color;  // category_color is not pub

// 3. In src/app.rs:
use crate::pages::exercises::helpers::category_color;
// category_color is pub, helpers module is pub(crate) in pages/exercises/mod.rs

// 4. In an external crate:
use gritwit::pages::ExercisesPage;
// ExercisesPage is pub, pages module is pub in lib.rs
```

<details>
<summary>Solution</summary>

1. **Valid.** `Exercise` is `pub` in `db.rs`, and `db` is `pub` in `lib.rs`. Any module in the crate can access it via `crate::db::Exercise`.

2. **Invalid.** `category_color` needs at least `pub(super)` visibility (visible to the parent module, `exercises/mod.rs`), or `pub(crate)` to be visible to `server_fns.rs` which is a sibling module. Without `pub`, it is only visible inside `helpers.rs` itself. Fix: change `fn category_color` to `pub(crate) fn category_color`.

3. **Valid if the helpers module is `pub(crate)`.** `pub(crate)` means visible to anything in the crate. Since `app.rs` is in the same crate, this works. If `helpers` were only `pub(super)`, it would be visible to `exercises/mod.rs` but not to `app.rs`.

4. **Valid.** `pages` is `pub` in `lib.rs`, and `ExercisesPage` is `pub use`'d in `pages/mod.rs`. The full chain of visibility is public.

</details>

### Drill 2: Re-exports

Refactor this `src/pages/mod.rs` so that the public API is flat — callers use `pages::ExercisesPage` instead of `pages::exercises::ExercisesPage`:

```rust
// Current (requires: use crate::pages::exercises::ExercisesPage)
pub mod exercises;
pub mod history;
pub mod home;
```

<details>
<summary>Solution</summary>

```rust
// src/pages/mod.rs
mod exercises;  // changed from pub to private
mod history;
mod home;

// Public re-exports — the only public API
pub use exercises::ExercisesPage;
pub use history::HistoryPage;
pub use home::HomePage;
```

By making the modules private (`mod` without `pub`) and re-exporting only the components, you hide the internal structure. Callers cannot access `pages::exercises::helper_function()` even if it is `pub` — the `exercises` module itself is not visible. This is the Rust equivalent of a controlled public API.

In the GrindIt reference codebase, `src/pages/mod.rs` uses exactly this pattern:

```rust
mod exercises;
mod history;
mod home;
mod log_workout;
mod login;
mod profile;

pub use exercises::ExercisesPage;
pub use history::HistoryPage;
pub use home::HomePage;
pub use log_workout::LogWorkoutPage;
pub use login::LoginPage;
pub use profile::ProfilePage;
```

</details>

### Drill 3: Feature Flags

Which of these will compile successfully when built with `cargo leptos watch` (which compiles both `ssr` and `hydrate` features separately)?

```rust
// A
#[cfg(feature = "ssr")]
use sqlx::PgPool;

pub fn get_pool() -> PgPool {
    todo!()
}

// B
#[cfg(feature = "ssr")]
use sqlx::PgPool;

#[cfg(feature = "ssr")]
pub fn get_pool() -> PgPool {
    todo!()
}

// C
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Exercise {
    pub name: String,
}
```

<details>
<summary>Solution</summary>

**A: Does NOT compile** for the `hydrate` build. The `use sqlx::PgPool` import is gated behind `ssr`, so on the `hydrate` build, `PgPool` does not exist. But `get_pool()` tries to return `PgPool` — the type is undefined. Both the import AND the function must be gated.

**B: Compiles.** Both the import and the function are gated behind `#[cfg(feature = "ssr")]`. On the `hydrate` build, neither exists — no error. On the `ssr` build, both exist — no error.

**C: Compiles.** `serde` is available in both features. `sqlx::FromRow` is only derived when `ssr` is active, using `cfg_attr`. On the `hydrate` build, the struct has only `Serialize` and `Deserialize` derives. On the `ssr` build, it additionally has `FromRow`. This is the standard pattern for shared structs in Leptos.

The rule: if a type from a gated dependency (like `sqlx`) appears in a function signature, struct field, or expression, that entire item must also be gated. The compiler must be able to resolve every type in the active feature set.

</details>

---

## DSA in Context: Tree Matching

Route resolution in `leptos_router` works like a **prefix tree (trie)** traversal. Each URL segment is a node in the tree, and the router walks the tree to find a match:

```
         (root)
        /      \
    exercises   log     history    login    profile
       |
     [id]    (future: /exercises/:id)
```

When the URL is `/exercises`, the router:
1. Starts at the root
2. Matches the first segment `exercises` to the `exercises` node
3. No more segments — renders `ExercisesPage`

When the URL is `/exercises/abc-123` (a future detail page):
1. Starts at the root
2. Matches `exercises`
3. Matches `abc-123` against the `[id]` parameter node
4. Renders `ExerciseDetailPage` with `id = "abc-123"`

This is O(depth) — proportional to the number of URL segments, not the total number of routes. Whether you have 5 routes or 500, matching `/exercises` takes the same time: walk 1 level deep.

The `fallback` on `<Routes>` is the "no match" case — when the trie traversal reaches a dead end. In our app, navigating to `/nonexistent` renders "Page not found."

---

## System Design Corner: URL Design

URLs are the API of your frontend. They should be:

- **Readable:** `/exercises` not `/page?id=2`
- **Predictable:** `/exercises/abc-123` for a specific exercise, following REST conventions
- **Bookmarkable:** Any URL should render the correct content when loaded directly (not just when navigated to from within the app)
- **Shareable:** A user should be able to copy a URL and send it to someone, who sees the same content

**RESTful URL patterns for GrindIt:**

| URL | What it shows |
|-----|--------------|
| `/` | Home — today's WOD |
| `/exercises` | Exercise library |
| `/exercises/abc-123` | Detail page for a specific exercise |
| `/log` | Log a new workout |
| `/history` | Past workout timeline |
| `/history/2024-03-15` | Workouts on a specific date |
| `/profile` | User settings |
| `/login` | Authentication |

**Query parameters vs path segments:** Use path segments for identifying resources (`/exercises/abc-123`) and query parameters for filtering or modifying the view (`/exercises?category=weightlifting&q=squat`). This is a REST convention, not a technical requirement.

**Deep linking:** Because Leptos uses SSR + hydration, every URL is deep-linkable by default. Loading `/exercises` directly renders the exercises page on the server — no client-side JavaScript needed for the first render. This is a major advantage over pure SPA frameworks where every URL serves the same `index.html` and the JavaScript figures out what to render.

> **Interview talking point:** *"Our URL structure follows REST conventions — path segments for resources, query parameters for filters. Because we use SSR with hydration, every URL is deep-linkable and SEO-friendly. The server renders the correct content for each URL, and the client-side router handles subsequent navigation without page reloads. Route matching uses a prefix tree for O(depth) performance regardless of the number of routes."*

---

## Design Insight: Strategic Programming

In *A Philosophy of Software Design*, Ousterhout contrasts **tactical programming** (get it working, move on) with **strategic programming** (invest in good structure now, even if it takes longer).

The module reorganization in this chapter is pure strategic programming. We could have kept everything in `app.rs` — it compiled, it worked, the user did not care. But the cost of the single-file approach compounds:

- **Finding code:** Where is the exercises search logic? Somewhere in the 500-line `app.rs`. Versus: it is in `src/pages/exercises/mod.rs`.
- **Merge conflicts:** Two people editing `app.rs` at the same time produce conflicts. Two people editing different files in `src/pages/` do not.
- **Compilation speed:** Rust recompiles changed files and their dependents. A change to `pages/history.rs` does not trigger recompilation of `pages/exercises/mod.rs`.
- **Mental model:** Each file has one responsibility. `src/db.rs` is the database layer. `src/pages/exercises/mod.rs` is the exercises UI. `src/validation.rs` is input validation. You can reason about each in isolation.

The investment is small — maybe 20 minutes of moving code and fixing imports. The return is permanent: every future chapter benefits from the clean structure. That is the essence of strategic programming.

The GrindIt reference codebase takes this further with submodules inside each page: `exercises/mod.rs`, `exercises/exercise_card.rs`, `exercises/exercise_form.rs`, `exercises/helpers.rs`, `exercises/server_fns.rs`. Each file has a single responsibility and a clear name. You do not need this level of decomposition yet — but when your exercises page grows past 300 lines, you will know exactly how to split it.

---

## What You Built

In this chapter, you:

1. **Reorganized the project** — moved from a single `app.rs` to `src/pages/` with separate modules for each page, re-exported through `pages/mod.rs`
2. **Set up the router** — `<Router>`, `<Routes>`, `<Route>` with `StaticSegment` for each page
3. **Added active tab highlighting** — `use_location().pathname` with `class:active` for reactive CSS class toggling
4. **Built `ScrollReset`** — `Effect::new` that scrolls `<main>` to the top on route changes, gated with `#[cfg(feature = "hydrate")]`
5. **Practiced modules** — `mod`, `pub`, `use`, `pub(crate)`, re-exports, and `#[cfg(feature = "...")]` conditional compilation

GrindIt now feels like a real multi-page app. Each tab navigates to a different page, the active tab lights up, and the scroll position resets cleanly. The project structure is ready to scale — each future chapter adds content to its respective page module without touching the others.

In Chapter 7, we will add authentication — user login, session management, and route guards that redirect anonymous users to the login page.

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/pages/mod.rs` | [`src/pages/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/mod.rs) |
| `src/pages/exercises/mod.rs` | [`src/pages/exercises/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/mod.rs) |
| `src/pages/home.rs` | [`src/pages/home/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/home/mod.rs) |
| `src/app.rs` (Router + Routes) | [`src/app.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/app.rs) — `<Router>`, `<Routes>`, `<Route>` with `StaticSegment` |
| `BottomNav` with active tabs | [`src/app.rs` — `BottomNav`](https://github.com/sivakarasala/gritwit/blob/main/src/app.rs) — `pathname.get().starts_with(...)` |
| `ScrollReset` component | [`src/app.rs` — `ScrollReset`](https://github.com/sivakarasala/gritwit/blob/main/src/app.rs) — `Effect::new` with `js_sys::eval` |
| `src/lib.rs` module structure | [`src/lib.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/lib.rs) — `pub mod app; pub mod db; pub mod pages;` |
