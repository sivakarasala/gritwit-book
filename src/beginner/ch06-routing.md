# Chapter 6: Multi-Page Routing

Until now, GrindIt has been a single page. The exercises page fills `<main>`, and the bottom nav tabs are dead links. Tap "History" and nothing happens. Tap "Home" and nothing happens. This chapter brings the app to life.

You will create separate pages for Home, Exercises, Log, and History. Each gets its own URL. The bottom nav will highlight the active tab. Navigating between pages will feel instant --- no full-page reload, just a smooth swap of content. This is what makes a web app feel like a native app.

But before we can add pages, we need to solve a practical problem: your `app.rs` file is getting big. Really big. It holds the shell, the header, the bottom nav, the exercises page, the form panel, the confirm modal, the toast system, and the server functions. Finding anything in there is like searching for a specific shirt in a pile of laundry.

This chapter teaches you how to organize your code into modules --- Rust's system for splitting code across files and controlling what is visible where.

By the end of this chapter, you will have:

- A `src/pages/` directory with separate modules for each page
- `leptos_router` configured with `<Router>`, `<Routes>`, and `<Route>`
- Active tab highlighting using `use_location().pathname`
- A scroll-to-top reset on every route change
- A clear understanding of Rust's module system, visibility rules, and feature flags

---

## Spotlight: Modules & Project Structure

### The problem with one big file

Your `app.rs` probably has 300+ lines by now. In JavaScript, you would split this into separate files with `import`/`export`:

```javascript
// pages/ExercisesPage.jsx
export function ExercisesPage() { ... }

// pages/HomePage.jsx
export function HomePage() { ... }

// App.jsx
import { ExercisesPage } from './pages/ExercisesPage';
import { HomePage } from './pages/HomePage';
```

Rust has its own system for organizing code into files. It works differently from JavaScript, but the goal is the same: keep each file focused on one thing, and make the connections between files explicit.

> **Programming Concept: What are Modules?**
>
> Think of a library (the building, not the code kind). Books are not piled in one giant heap. They are organized into sections: Fiction, Science, History, Reference. Each section has shelves, and each shelf has a label.
>
> Modules work the same way. Instead of putting all your code in one file, you organize it into named sections:
> - `pages/exercises` --- everything related to the exercises page
> - `pages/home` --- the home page
> - `db` --- database functions
> - `validation` --- input checking
>
> Each module is a file (or a folder with files). The module system tells the compiler which files belong to your project and how they connect to each other.

### `mod` --- declaring a module

In Rust, the file system *is* the module tree. But unlike JavaScript, simply creating a file does not make it part of your project. You must explicitly declare it with `mod`:

```rust
// src/lib.rs
pub mod app;         // loads src/app.rs
pub mod db;          // loads src/db.rs
pub mod pages;       // loads src/pages/mod.rs (or src/pages.rs)
pub mod validation;  // loads src/validation.rs
```

Each `mod` declaration tells the compiler: "there is a module with this name --- go find its file." The compiler looks for either `src/{name}.rs` or `src/{name}/mod.rs`. The convention is:

- **`src/foo.rs`** --- when the module is a single file
- **`src/foo/mod.rs`** --- when the module has sub-modules (child files in the `foo/` directory)

If you create a file `src/unused.rs` but never write `mod unused;` anywhere, the compiler ignores it completely. It is as if the file does not exist. This is very different from JavaScript, where importing a file automatically includes it in the bundle.

### `pub` --- visibility

> **Programming Concept: What is `pub`?**
>
> Think of a store. The front counter is public --- customers can see and interact with it. The back room is private --- only employees can access it.
>
> In Rust, everything is private by default. If you write `fn helper()`, only code in the same file can call it. To make it visible to the outside world, you add `pub`: `pub fn helper()`.
>
> This is about controlling your API --- deciding what other code is allowed to use. Private functions are implementation details that you can change freely. Public functions are promises that other code depends on.

```rust
// src/pages/exercises/mod.rs
pub fn ExercisesPage() { ... }     // visible to anyone who can see this module
fn helper_function() { ... }       // private --- only usable inside this file
```

Rust has finer-grained visibility than just "public" or "private":

| Modifier | Visible to | Analogy |
|----------|-----------|---------|
| *(none)* | Only the current module and its children | Your desk drawer |
| `pub` | Everyone | A billboard on the highway |
| `pub(crate)` | Anything within this crate (project) | Internal company memo |
| `pub(super)` | The parent module only | A note you leave for your roommate |

For GrindIt, `pub(crate)` is the sweet spot for most things. Our pages should be visible within the `gritwit` crate (so `app.rs` can import them) but not to external crates that might depend on our library.

### `use` --- bringing names into scope

`mod` declares that a module exists. `use` brings items from that module into your current scope so you can use them without typing the full path:

```rust
// Without `use` --- full path every time:
crate::pages::exercises::ExercisesPage

// With `use` --- bring it into scope once:
use crate::pages::exercises::ExercisesPage;
ExercisesPage  // much shorter!
```

The `crate::` prefix means "start from the root of this crate." You can also use:
- `super::` --- the parent module (like `../` in file paths)
- `self::` --- the current module (rarely needed)

### Re-exports --- a clean public API

A common pattern is to re-export items from child modules so the parent presents a simple, flat interface:

```rust
// src/pages/mod.rs
mod exercises;      // private --- outsiders cannot access exercises::*
mod history;
mod home;

// Re-export just the page components
pub use exercises::ExercisesPage;
pub use history::HistoryPage;
pub use home::HomePage;
```

Now callers write `use crate::pages::ExercisesPage` instead of the longer `use crate::pages::exercises::ExercisesPage`. The internal structure (the `exercises` subdirectory) is hidden. This is like a store's front counter --- customers see a clean selection, not the messy stockroom.

This is the same pattern as a JavaScript `index.ts` barrel file:

```typescript
// pages/index.ts (JavaScript equivalent)
export { ExercisesPage } from './exercises';
export { HistoryPage } from './history';
export { HomePage } from './home';
```

### Conditional compilation: `#[cfg(feature = "...")]`

> **Programming Concept: What is Conditional Compilation?**
>
> Imagine you write a cookbook with two sets of instructions: one for a home cook and one for a professional chef. Each reader only sees the instructions relevant to them.
>
> Conditional compilation works the same way. Leptos compiles your code *twice*:
> - Once for the **server** (`ssr` feature) --- this build talks to the database, handles HTTP requests, and renders HTML
> - Once for the **browser** (`hydrate` feature) --- this build runs as WebAssembly, handles clicks, and updates the DOM
>
> Some code should only exist in one build. Database queries belong on the server. DOM manipulation belongs in the browser. Conditional compilation lets you include different code depending on which build is happening.

```rust
// Only compiled into the server binary
#[cfg(feature = "ssr")]
pub mod routes;

// Only compiled into the WASM bundle
#[cfg(feature = "hydrate")]
pub mod pwa;

// Compiled in both, but with conditional parts inside
pub mod db;  // the struct definitions are shared; the query functions are #[cfg(feature = "ssr")]
```

You have already seen `#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]` in Chapter 5 --- that conditionally applies an attribute. The broader `#[cfg(feature = "ssr")]` removes the *entire item* from compilation when the feature is not active.

This is more powerful than JavaScript's tree-shaking. Tree-shaking removes *unused* code after bundling. Rust's conditional compilation never compiles the code in the first place --- it does not exist in the binary, and the compiler does not even type-check it for the disabled feature.

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
> The biggest difference: in JavaScript, importing a file automatically makes it part of the bundle. In Rust, a file does nothing until a `mod` statement claims it.

---

## Exercise 1: Reorganize into a Module Structure

**Goal:** Move the ExercisesPage into `src/pages/exercises/mod.rs` and create stub pages for Home, Log, History, Login, and Profile.

### Step 1: Create the directory structure

Here is the target structure. You will be creating several new files and folders:

```
src/
+-- app.rs
+-- db.rs
+-- lib.rs
+-- main.rs
+-- validation.rs
+-- pages/
    +-- mod.rs
    +-- exercises/
    |   +-- mod.rs
    +-- home.rs
    +-- history.rs
    +-- log_workout.rs
    +-- login.rs
    +-- profile.rs
```

Create the directories first:

```bash
mkdir -p src/pages/exercises
```

### Step 2: Move the ExercisesPage

This is the biggest step. Cut the `ExercisesPage` component (and its related server functions, form component, etc.) from `src/app.rs` and paste it into `src/pages/exercises/mod.rs`:

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
            // (your full implementation goes here)

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

The code itself has not changed from Chapter 5. The only difference is that it now lives in its own file instead of being crammed into `app.rs`.

### Step 3: Create the stub pages

Each stub page is a placeholder that will be filled in later chapters. They follow a simple pattern:

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

Each page follows the same pattern: import Leptos, define a component function marked with `#[component]` and `pub`, return a view. Simple and consistent.

### Step 4: Create the pages module with re-exports

This is the "table of contents" for all your pages. Create `src/pages/mod.rs`:

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

Notice two important things:

1. **The modules are `mod`, not `pub mod`.** This means the modules themselves are private --- code outside `pages/` cannot reach into `pages::exercises::helper_function()`. Only the re-exported items are visible.

2. **The `pub use` lines re-export exactly what callers need.** This is the "clean front counter" pattern. Callers write `use crate::pages::ExercisesPage`, not `use crate::pages::exercises::ExercisesPage`. The internal structure is hidden.

### Step 5: Update `lib.rs`

```rust
pub mod app;
pub mod db;
pub mod pages;
pub mod validation;
```

The `pub mod pages;` declaration tells the compiler to look for `src/pages/mod.rs`. That file in turn declares its child modules (`exercises`, `home`, etc.). The entire tree is connected through `mod` declarations, from `lib.rs` down to the individual page files.

### Step 6: Update `app.rs` imports

Remove the old `ExercisesPage` component and server functions from `app.rs` (you moved them to `pages/exercises/mod.rs`). Add the import:

```rust
// src/app.rs
use crate::pages::{ExercisesPage, HistoryPage, HomePage, LogWorkoutPage, LoginPage, ProfilePage};
```

### Step 7: Verify it compiles

```bash
cargo leptos watch
```

If you see import errors, they are usually one of two things:
1. You forgot to move a function or type that `app.rs` was using
2. You forgot a `pub` on something that needs to be visible outside its module

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

> **Programming Concept: What is Routing?**
>
> Think of a street address. The address `123 Main St` tells the mail carrier exactly which building to go to. Web routing works the same way:
>
> - URL `/` goes to the Home page
> - URL `/exercises` goes to the Exercises page
> - URL `/history` goes to the History page
>
> The **router** is the mail carrier. It looks at the URL and delivers you to the right page. Without a router, every URL would show the same content.

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

The router must be in *both* features because it works on both sides:
- On the **server**, it reads the incoming request URL and renders the matching page to HTML
- On the **client**, it intercepts link clicks and swaps page content without a full reload

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

### Step 3: Understand the routing components

Let us understand each piece:

**`<Router>`** wraps your entire app. It provides the routing context that all child components can read. Everything that needs routing information --- including the `BottomNav` (for active tab highlighting) --- must be inside `<Router>`.

**`<Routes>`** is the switch. It looks at the current URL and renders the matching `<Route>`. Think of it as a receptionist: "You are looking for `/exercises`? That is the ExercisesPage. Right this way."

**`<Route path=StaticSegment("exercises") view=ExercisesPage/>`** means: when the URL is `/exercises`, render the `ExercisesPage` component. `StaticSegment` matches a literal path segment --- the word "exercises" exactly.

**`fallback=|| "Page not found.".into_view()`** is the 404 page. When someone navigates to `/nonexistent`, no route matches, and this fallback renders.

The `<Header/>` and `<BottomNav/>` are outside `<Routes>` but inside `<Router>`. This means they render on *every* page. Only the content inside `<Routes>` changes when you navigate.

### Step 4: How client-side routing works

When you first load the app, the server renders the correct page (SSR). The HTML arrives in the browser with the right content already in it.

After the WASM bundle loads and hydrates, something changes. Now when you click a nav link:

1. The router **intercepts** the click (it does not become a real page navigation)
2. The URL updates using the browser's History API (the address bar changes)
3. The router swaps the component inside `<Routes>` (ExercisesPage out, HistoryPage in)
4. No server request. No page reload. Instant.

This is why navigation between tabs feels so fast after the initial load. The WASM bundle is already loaded, and switching pages just means rendering a different component.

### Step 5: Update navigation links

Your `BottomNav` already uses `<a href="/exercises">`. With the router in place, these links are automatically intercepted on the client side. You can optionally use Leptos's `<A>` component for explicit router integration:

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

Test it: navigate between tabs. Each URL should show different page content, and the browser's back/forward buttons should work.

<details>
<summary>Hint: If clicking a tab causes a full page reload</summary>

Make sure the `<BottomNav/>` component is rendered *inside* the `<Router>` component, not outside it. If it is outside, the router cannot intercept the link clicks. Also ensure that `leptos_router` is included in both the `ssr` and `hydrate` features in `Cargo.toml`.

</details>

---

## Exercise 3: Active Tab Highlighting

**Goal:** Make the BottomNav highlight the tab that matches the current URL.

### Step 1: Use `use_location()`

Right now, all four tabs look the same no matter which page you are on. Users need a visual cue showing where they are. The router provides `use_location()`, which gives you reactive access to the current URL:

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

For the Home tab, we check exact equality: `pathname.get() == "/"`. This is because we only want the Home tab highlighted on the root URL, not on every URL that starts with `/` (which would be all of them).

For other tabs, we use `.starts_with()` so that sub-routes (like a future `/exercises/abc-123` detail page) still highlight the Exercises tab.

This is **reactive**. `pathname` is a signal. When you navigate from `/exercises` to `/history`:
1. `pathname` updates to `"/history"`
2. The closure for Exercises re-runs: `"/history".starts_with("/exercises")` is `false` --- `active` class is removed
3. The closure for History re-runs: `"/history".starts_with("/history")` is `true` --- `active` class is added
4. The tab icons update visually

No manual DOM manipulation. No `document.querySelector`. Just reactive signals doing their thing.

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

Save and test. As you tap each tab, the icon and label should turn orange (the accent color), while the other tabs remain dim. This small visual detail makes a big difference in usability.

<details>
<summary>Hint: If the active class is not being applied</summary>

Check that `use_location()` is being called inside the `<Router>` context. If `BottomNav` is rendered outside `<Router>`, `use_location()` will panic or return a default value. Also verify that the pathname check matches your route definitions --- `"/exercises"` not `"/exercises/"` (trailing slashes matter).

</details>

---

## Exercise 4: Add ScrollReset on Route Change

**Goal:** When navigating to a new page, scroll back to the top. Without this, scrolling down on the Exercises page and then navigating to History would show History scrolled to the same position --- confusing and janky.

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

    // This component renders nothing --- it is a side-effect-only component
}
```

Let us break this down:

**`Effect::new(move |_| { ... })`** creates a reactive side effect. It runs once immediately and then re-runs whenever any signal it reads changes. Since we read `pathname.get()` inside the effect, it re-runs on every route change.

**`let _ = pathname.get();`** --- we do not care about the actual pathname value. We just need to *read* it so the effect subscribes to changes. The `let _ =` means "I am intentionally ignoring this value."

**`#[cfg(feature = "hydrate")]`** --- the scroll code only runs in the browser. On the server (during SSR), there is no DOM to scroll, and `js_sys::eval` does not exist.

**`js_sys::eval(...)`** executes a JavaScript snippet. It finds the `<main>` element and scrolls it to the top. We use `js_sys::eval` instead of `web_sys` for brevity --- for a simple one-liner, it is shorter than importing multiple DOM types.

This component renders nothing visible. It exists purely for its side effect (scrolling). This is a valid and common pattern in Leptos --- components that set up reactive behavior without producing UI.

### Step 2: Add it to the App

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

Verify that your `<main>` element has `overflow-y: auto` in CSS. If `<main>` does not scroll (if the body scrolls instead), the `document.querySelector('main').scrollTo()` call will have no effect. Also check that `ScrollReset` is inside `<Router>` --- it needs access to `use_location()`.

</details>

---

## Rust Gym

### Drill 1: Module Paths

For each `use` statement, predict whether it will compile. If not, explain why.

Given this file structure:

```
src/
+-- lib.rs         (contains: pub mod app; pub mod db; pub mod pages;)
+-- app.rs
+-- db.rs          (contains: pub struct Exercise { ... })
+-- pages/
    +-- mod.rs     (contains: mod exercises; pub use exercises::ExercisesPage;)
    +-- exercises/
        +-- mod.rs (contains: pub fn ExercisesPage() { ... }
                              fn helper() { ... })
```

```rust
// A: In src/app.rs:
use crate::pages::ExercisesPage;

// B: In src/app.rs:
use crate::pages::exercises::ExercisesPage;

// C: In src/pages/exercises/mod.rs:
use crate::db::Exercise;

// D: In src/app.rs:
use crate::pages::exercises::helper;
```

<details>
<summary>Solution</summary>

**A: Compiles.** `ExercisesPage` is re-exported with `pub use` in `pages/mod.rs`, so it is accessible as `crate::pages::ExercisesPage`.

**B: Does NOT compile.** The `exercises` module is declared as `mod exercises` (private) in `pages/mod.rs`. From `app.rs`, you cannot reach into a private module. You can only use what `pages/mod.rs` re-exports.

**C: Compiles.** `Exercise` is `pub` in `db.rs`, and `db` is `pub` in `lib.rs`. Any module in the crate can access it via `crate::db::Exercise`.

**D: Does NOT compile for two reasons.** First, `exercises` is private (same as B). Second, even if it were public, `helper` is not `pub` --- it is private to its file.

</details>

### Drill 2: Building a Module

Create a module structure for a simple blog app. Given these requirements:
- A `Post` struct (id, title, body) in a `models` module
- A `list_posts()` function in a `db` module
- A `PostsPage` component in a `pages` module
- The `pages` module should re-export `PostsPage` but hide its internal structure

Write the `mod` and `pub use` declarations for `src/lib.rs` and `src/pages/mod.rs`.

<details>
<summary>Hint</summary>

Think about which modules need `pub` and which do not. `lib.rs` needs `pub mod` for anything that the binary crate needs to access. `pages/mod.rs` makes the inner module private but re-exports the component.

</details>

<details>
<summary>Solution</summary>

```rust
// src/lib.rs
pub mod models;    // pub because app.rs needs Post
pub mod db;        // pub because app.rs needs list_posts
pub mod pages;     // pub because app.rs needs PostsPage

// src/pages/mod.rs
mod posts;         // private --- internal structure is hidden

pub use posts::PostsPage;  // only the component is re-exported
```

Now `app.rs` can write:
```rust
use crate::pages::PostsPage;   // works --- re-exported
use crate::models::Post;        // works --- pub module, pub struct
// use crate::pages::posts::PostsPage;  // would NOT work --- posts is private
```

</details>

### Drill 3: Feature Flags

Which of these will compile when built with `cargo leptos watch` (which compiles both `ssr` and `hydrate` features separately)?

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
<summary>Hint</summary>

Think about what happens during the `hydrate` (WASM) build. If a type from a gated import appears in a non-gated function signature, the WASM build will not know what that type is.

</details>

<details>
<summary>Solution</summary>

**A: Does NOT compile** for the `hydrate` build. The `use sqlx::PgPool` import is gated behind `ssr`, so on the `hydrate` build, `PgPool` does not exist. But `get_pool()` tries to return `PgPool` --- undefined type. Both the import AND the function must be gated.

**B: Compiles.** Both the import and the function are gated behind `#[cfg(feature = "ssr")]`. On the `hydrate` build, neither exists. On the `ssr` build, both exist.

**C: Compiles.** `serde` is available in both features. The `sqlx::FromRow` derive only applies when `ssr` is active (via `cfg_attr`). On the `hydrate` build, the struct just has `Serialize` and `Deserialize`. This is the standard pattern for shared structs in Leptos.

The rule: if a type from a gated dependency (like `sqlx`) appears in a function signature, struct field, or expression, that entire item must also be gated.

</details>

---

## What You Built

In this chapter, you:

1. **Reorganized the project** --- moved from a single `app.rs` to `src/pages/` with separate modules for each page, re-exported through `pages/mod.rs`
2. **Set up the router** --- `<Router>`, `<Routes>`, `<Route>` with `StaticSegment` for each page
3. **Added active tab highlighting** --- `use_location().pathname` with `class:active` for reactive CSS class toggling
4. **Built `ScrollReset`** --- `Effect::new` that scrolls `<main>` to the top on route changes, gated with `#[cfg(feature = "hydrate")]`
5. **Practiced modules** --- `mod`, `pub`, `use`, re-exports, and `#[cfg(feature = "...")]` conditional compilation

GrindIt now feels like a real multi-page app. Each tab navigates to a different page, the active tab lights up, and the scroll position resets cleanly. The project structure is ready to scale --- each future chapter adds content to its own page module without touching the others.

This organizational investment pays off immediately. Need to find the exercises search logic? It is in `src/pages/exercises/mod.rs`. Need to add a new page? Create a file in `src/pages/`, add a `mod` declaration, add a route. The pattern is clear and repeatable.

In Chapter 7, we will add authentication --- user login, session management, and route guards that redirect anonymous users to the login page.

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/pages/mod.rs` | [`src/pages/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/mod.rs) |
| `src/pages/exercises/mod.rs` | [`src/pages/exercises/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/mod.rs) |
| `src/pages/home.rs` | [`src/pages/home/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/home/mod.rs) |
| `src/app.rs` (Router + Routes) | [`src/app.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/app.rs) |
| `BottomNav` with active tabs | [`src/app.rs` --- `BottomNav`](https://github.com/sivakarasala/gritwit/blob/main/src/app.rs) |
| `ScrollReset` component | [`src/app.rs` --- `ScrollReset`](https://github.com/sivakarasala/gritwit/blob/main/src/app.rs) |
| `src/lib.rs` module structure | [`src/lib.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/lib.rs) |
