# Chapter 14: PWA & WASM Interop

GrindIt runs in the browser, but it should feel like a native app. This chapter builds the Progressive Web App layer: a service worker for offline support, a theme toggle that persists across sessions, and an install banner that detects iOS vs Android and prompts accordingly. All of this is orchestrated from Rust using `wasm_bindgen`, `js_sys`, and `web_sys` — Rust code that compiles to WebAssembly and calls JavaScript APIs directly.

The spotlight concept is **WASM-JavaScript interop** — the bridge between Rust compiled to WebAssembly and the browser's JavaScript runtime. You will see three techniques: `js_sys` for calling built-in JS objects, `web_sys` for DOM and Web API access, and `#[wasm_bindgen(inline_js)]` for embedding custom JavaScript. You will also learn why hydration makes this tricky — code that runs differently on server vs client will cause mismatches.

By the end of this chapter, you will have:

- A `register_service_worker()` function using `js_sys` reflection and `spawn_local`
- A `toggle_theme()` function using `#[wasm_bindgen(inline_js)]` with localStorage and `document.documentElement`
- An `InstallBanner` component with iOS detection, `beforeinstallprompt` handling, and dismiss persistence
- A `manifest.json` with icons, theme color, and standalone display
- A `sw.js` with network-first navigation caching and stale-while-revalidate for static assets

---

## Spotlight: wasm_bindgen, web_sys, js_sys

### The WASM-JS boundary

WebAssembly runs in its own sandbox. It cannot directly access the DOM, call `fetch`, read `localStorage`, or do anything browser-specific. To interact with the browser, WASM must call JavaScript functions through a bridge. Rust's `wasm_bindgen` crate generates this bridge automatically.

There are three layers of abstraction:

1. **`js_sys`** — bindings to JavaScript built-in objects (`Promise`, `Function`, `Reflect`, `Array`, etc.). These exist in every JS environment, including Node.js.
2. **`web_sys`** — bindings to Web APIs (`Window`, `Document`, `Element`, `Navigator`, `ServiceWorkerContainer`, etc.). These exist only in browsers.
3. **`#[wasm_bindgen(inline_js)]`** — embed raw JavaScript and call it from Rust. The escape hatch for anything not covered by the binding crates.

> **Coming from JS?** In JavaScript, you call `navigator.serviceWorker.register('/sw.js')` and it just works. In Rust-WASM, every property access is a function call through the FFI boundary. `navigator.serviceWorker` becomes `js_sys::Reflect::get(&navigator, &JsValue::from_str("serviceWorker"))`. This verbosity is the cost of strong typing. The benefit: if the API does not exist (older browsers, server-side rendering), the `Result` type forces you to handle it.

### js_sys::Reflect — property access without types

`js_sys::Reflect` provides the `get()` and `set()` functions for accessing JavaScript object properties by name. This is the equivalent of JavaScript's bracket notation (`obj["key"]`):

```rust
use wasm_bindgen::prelude::*;

// JavaScript: window.navigator.serviceWorker
// Rust:
let global = js_sys::global();  // window or globalThis
let navigator = js_sys::Reflect::get(&global, &JsValue::from_str("navigator"))
    .unwrap_or(JsValue::UNDEFINED);
let sw = js_sys::Reflect::get(&navigator, &JsValue::from_str("serviceWorker"))
    .unwrap_or(JsValue::UNDEFINED);
```

Every `Reflect::get` returns `Result<JsValue, JsValue>` — it can fail if the property does not exist or the object is not extensible. The `unwrap_or(JsValue::UNDEFINED)` pattern converts missing properties to `undefined` rather than panicking, matching JavaScript's own behavior.

### wasm_bindgen_futures::spawn_local

Browser APIs return Promises. Rust's async functions return Futures. `spawn_local` bridges the two — it takes a Rust Future and runs it on the browser's microtask queue:

```rust
wasm_bindgen_futures::spawn_local(async move {
    let promise = register_fn.call1(&sw_container, &JsValue::from_str("/sw.js"))?;
    let result = JsFuture::from(promise.unchecked_into::<js_sys::Promise>()).await;
    // ...
});
```

`spawn_local` is the client-side equivalent of `tokio::spawn`. It does not create a thread (browsers are single-threaded for WASM). Instead, it schedules the future as a microtask. The `async move` closure captures variables by moving them into the future, which is necessary because the future outlives the current scope.

### The #[cfg(feature = "hydrate")] guard

Leptos compiles your code twice: once for the server (`feature = "ssr"`) and once for the client (`feature = "hydrate"`). Browser APIs exist only on the client. If you call `js_sys::global()` on the server, the code panics.

The solution is conditional compilation:

```rust
Effect::new(move |_| {
    #[cfg(feature = "hydrate")]
    {
        // This block only compiles for the WASM client
        let global = js_sys::global();
        // ... browser API calls ...
    }
});
```

The `Effect::new` ensures the code runs after hydration — when the WASM client has taken over from the server-rendered HTML. Without the `Effect`, the code would run during SSR and fail.

### The hydration mismatch problem

Server-side rendering generates HTML. The client hydrates that HTML — attaching event listeners and making it interactive. If the server renders different HTML than the client expects, Leptos detects a **hydration mismatch** and may throw errors or produce broken UI.

This matters for PWA features because:

- The server does not know if the user is on iOS
- The server does not know the user's theme preference
- The server does not know if the app is installed

The solution: render a neutral initial state on both server and client, then update in an `Effect` after hydration. The `InstallBanner` starts with `show = false` on both server and client. After hydration, an `Effect` checks the browser state and may set `show = true`. The DOM updates reactively — no mismatch.

---

## Service Worker Registration from Rust

### The register_service_worker function

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

pub fn register_service_worker() {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &JsValue::from_str("navigator"))
        .unwrap_or(JsValue::UNDEFINED);

    let sw_container = js_sys::Reflect::get(
        &navigator, &JsValue::from_str("serviceWorker")
    ).unwrap_or(JsValue::UNDEFINED);

    if sw_container.is_undefined() {
        return;  // Browser does not support service workers
    }

    wasm_bindgen_futures::spawn_local(async move {
        let register_fn = match js_sys::Reflect::get(
            &sw_container, &JsValue::from_str("register")
        ) {
            Ok(f) if f.is_function() => f.unchecked_into::<js_sys::Function>(),
            _ => return,
        };

        let promise = match register_fn.call1(
            &sw_container, &JsValue::from_str("/sw.js")
        ) {
            Ok(p) => p.unchecked_into::<js_sys::Promise>(),
            Err(_) => return,
        };

        match JsFuture::from(promise).await {
            Ok(_) => { let _ = log_to_console("Service worker registered"); }
            Err(_) => { let _ = log_to_console("Service worker registration failed"); }
        }
    });
}
```

The step-by-step navigation through JavaScript objects:

1. **`js_sys::global()`** — returns `window` in a browser, `globalThis` in Node.js.
2. **`Reflect::get(&global, "navigator")`** — accesses `window.navigator`.
3. **`Reflect::get(&navigator, "serviceWorker")`** — accesses `navigator.serviceWorker`. If the browser does not support service workers (e.g., older iOS WebView), this is `undefined`.
4. **`Reflect::get(&sw, "register")`** — gets the `register` method as a `JsValue`.
5. **`unchecked_into::<js_sys::Function>()`** — casts the `JsValue` to a `Function`. The "unchecked" means we trust that `register` is indeed a function. If it is not, this would cause undefined behavior — but the `.is_function()` guard protects us.
6. **`register_fn.call1(&sw_container, &JsValue::from_str("/sw.js"))`** — calls `navigator.serviceWorker.register("/sw.js")`. The first argument (`&sw_container`) is the `this` binding. The second is the function argument.
7. **`JsFuture::from(promise).await`** — converts a JavaScript `Promise` into a Rust `Future` and awaits it.

### The log_to_console helper

```rust
fn log_to_console(msg: &str) -> Result<(), JsValue> {
    let global = js_sys::global();
    let console = js_sys::Reflect::get(&global, &JsValue::from_str("console"))?;
    let log_fn = js_sys::Reflect::get(&console, &JsValue::from_str("log"))?
        .unchecked_into::<js_sys::Function>();
    log_fn.call1(&console, &JsValue::from_str(msg))?;
    Ok(())
}
```

This is four lines of Rust to call `console.log(msg)`. The verbosity illustrates why `#[wasm_bindgen(inline_js)]` exists — for frequently used browser APIs, inline JS is dramatically shorter.

---

## Theme Toggle with inline_js

### The wasm_bindgen inline_js approach

The `toggle_theme()` function is defined in JavaScript and called from Rust:

```rust
#[wasm_bindgen(inline_js = "
export function toggle_theme() {
    const html = document.documentElement;
    const current = html.getAttribute('data-theme') || 'dark';
    const next = current === 'dark' ? 'light' : 'dark';
    html.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
    return next;
}
")]
extern "C" {
    pub fn toggle_theme() -> String;
}
```

The `#[wasm_bindgen(inline_js = "...")]` attribute embeds the JavaScript directly in the WASM module. The `extern "C"` block declares the Rust function signature. When Rust calls `toggle_theme()`, it crosses the WASM-JS boundary, executes the JavaScript, and returns the result as a Rust `String`.

Why inline JS for this function?

The pure Rust equivalent using `web_sys` would be:

```rust
// Pure web_sys approach — 12 lines vs 6 lines of inline JS
fn toggle_theme_web_sys() -> String {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let html = document.document_element().unwrap();
    let current = html.get_attribute("data-theme")
        .unwrap_or_else(|| "dark".to_string());
    let next = if current == "dark" { "light" } else { "dark" };
    html.set_attribute("data-theme", next).unwrap();
    let storage = window.local_storage().unwrap().unwrap();
    storage.set_item("theme", next).unwrap();
    next.to_string()
}
```

More code, more `unwrap()` calls, and the same result. For a function this simple, inline JS wins on clarity. The rule: use `web_sys` when you need Rust type safety (e.g., building DOM elements programmatically). Use inline JS when you are calling a well-understood browser API with no complex logic.

### Theme initialization without hydration mismatch

The theme must be applied before the page renders — otherwise the user sees a flash of the wrong theme. GrindIt handles this with a blocking `<script>` in the HTML `<head>`:

```html
<script>
  (function() {
    var t = localStorage.getItem('theme') || 'dark';
    document.documentElement.setAttribute('data-theme', t);
  })()
</script>
```

This runs synchronously before any CSS or WASM loads. The `data-theme` attribute is set on `<html>`, and CSS variables switch the color scheme:

```css
:root, [data-theme="dark"] {
    --bg-primary: #0f0f1a;
    --text-primary: #e8e8e8;
}
[data-theme="light"] {
    --bg-primary: #f5f5f5;
    --text-primary: #1a1a2e;
}
```

The Rust `toggle_theme()` function is only called when the user clicks the theme button — by that point, hydration is complete and the WASM bridge is active.

### The Header component with theme toggle

```rust
#[component]
fn Header(user: Option<AuthUser>) -> impl IntoView {
    view! {
        <header class="top-bar">
            <span class="top-bar__logo">"Grind"<span class="top-bar__flame"></span>"t"</span>
            <div class="top-bar__actions">
                <button class="theme-toggle" on:click=move |_| {
                    #[cfg(feature = "hydrate")]
                    { crate::voice::toggle_theme(); }
                }>
                    <span class="theme-icon theme-icon--sun"></span>
                    <span class="theme-icon theme-icon--moon"></span>
                </button>
            </div>
        </header>
    }
}
```

The `#[cfg(feature = "hydrate")]` guard ensures `toggle_theme()` is only called on the client. On the server, the `on:click` handler compiles to nothing — the button renders but clicking it does nothing until hydration completes. After hydration, the WASM event listener takes over and the toggle works.

---

## The InstallBanner Component

### PWA install detection

Browsers use two different mechanisms for PWA installation:

- **Android/Chrome**: Fires a `beforeinstallprompt` event when the PWA meets installability criteria. The app can capture this event and trigger the install prompt later.
- **iOS/Safari**: No `beforeinstallprompt` event. The user must manually tap "Share" then "Add to Home Screen". The app can only show instructions.

GrindIt captures the `beforeinstallprompt` event in a `<script>` tag in the HTML `<head>`:

```html
<script>
  window.__pwaInstallPrompt = null;
  window.addEventListener('beforeinstallprompt', function(e) {
    e.preventDefault();
    window.__pwaInstallPrompt = e;
  });
  window.__isIos = /iPad|iPhone|iPod/.test(navigator.userAgent) && !window.MSStream;
  window.__isStandalone = window.matchMedia('(display-mode:standalone)').matches
    || navigator.standalone === true;
</script>
```

Three global variables are set:

- **`__pwaInstallPrompt`** — the captured `BeforeInstallPromptEvent`, or `null` if not fired
- **`__isIos`** — `true` if the user agent matches iOS devices
- **`__isStandalone`** — `true` if the app is already running in standalone mode (installed)

These are set in a `<script>` tag rather than in Rust because the `beforeinstallprompt` event fires early — often before WASM loads. If we waited for WASM initialization, we would miss the event.

### The InstallBanner component

```rust
#[component]
fn InstallBanner() -> impl IntoView {
    let show = RwSignal::new(false);
    let is_ios = RwSignal::new(false);

    Effect::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            use wasm_bindgen::prelude::*;
            let global = js_sys::global();

            let standalone = js_sys::Reflect::get(
                &global, &JsValue::from_str("__isStandalone")
            ).unwrap_or(JsValue::FALSE)
                .as_bool().unwrap_or(false);

            let dismissed = {
                let ls = js_sys::Reflect::get(
                    &global, &JsValue::from_str("localStorage")
                ).unwrap_or(JsValue::UNDEFINED);
                if !ls.is_undefined() {
                    let get_fn = js_sys::Reflect::get(
                        &ls, &JsValue::from_str("getItem")
                    ).unwrap_or(JsValue::UNDEFINED);
                    if let Ok(f) = get_fn.dyn_into::<js_sys::Function>() {
                        f.call1(&ls, &JsValue::from_str("pwa_install_dismissed"))
                            .unwrap_or(JsValue::NULL)
                            == JsValue::from_str("1")
                    } else { false }
                } else { false }
            };

            if !standalone && !dismissed {
                let ios = js_sys::Reflect::get(
                    &global, &JsValue::from_str("__isIos")
                ).unwrap_or(JsValue::FALSE)
                    .as_bool().unwrap_or(false);
                let has_prompt = !js_sys::Reflect::get(
                    &global, &JsValue::from_str("__pwaInstallPrompt")
                ).unwrap_or(JsValue::NULL).is_null();

                if ios || has_prompt {
                    show.set(true);
                    is_ios.set(ios);
                }
            }
        }
    });
```

The detection logic runs in an `Effect` after hydration. The decision tree:

1. **Already standalone?** Do not show the banner — the app is already installed.
2. **Previously dismissed?** Check `localStorage` for the dismiss flag. If set, do not show.
3. **iOS device?** Show the iOS-specific banner with "Add to Home Screen" instructions.
4. **Has install prompt?** Show the Android/Chrome banner with an "Install" button.
5. **Neither?** Do not show — the browser does not support PWA installation.

### Dismiss with localStorage persistence

```rust
let dismiss = move |_| {
    show.set(false);
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        let global = js_sys::global();
        let ls = js_sys::Reflect::get(
            &global, &JsValue::from_str("localStorage")
        ).unwrap_or(JsValue::UNDEFINED);
        if !ls.is_undefined() {
            if let Ok(f) = js_sys::Reflect::get(
                &ls, &JsValue::from_str("setItem")
            ).unwrap_or(JsValue::UNDEFINED)
                .dyn_into::<js_sys::Function>()
            {
                let _ = f.call2(&ls,
                    &JsValue::from_str("pwa_install_dismissed"),
                    &JsValue::from_str("1"),
                );
            }
        }
    }
};
```

The dismiss handler does two things: hides the banner immediately (`show.set(false)`) and persists the dismissal to `localStorage`. On the next page load, the `Effect` checks this flag and skips showing the banner. The `let _ =` prefix on `f.call2()` explicitly discards the `Result` — if `localStorage.setItem` fails (e.g., private browsing mode with full storage), the banner still hides.

### Install trigger for Android/Chrome

```rust
let install = move |_| {
    #[cfg(feature = "hydrate")]
    {
        use wasm_bindgen::prelude::*;
        let global = js_sys::global();
        let prompt = js_sys::Reflect::get(
            &global, &JsValue::from_str("__pwaInstallPrompt")
        ).unwrap_or(JsValue::NULL);
        if !prompt.is_null() {
            if let Ok(f) = js_sys::Reflect::get(
                &prompt, &JsValue::from_str("prompt")
            ).unwrap_or(JsValue::UNDEFINED)
                .dyn_into::<js_sys::Function>()
            {
                let _ = f.call0(&prompt);
            }
        }
        show.set(false);
    }
};
```

This retrieves the captured `BeforeInstallPromptEvent` from `window.__pwaInstallPrompt` and calls its `.prompt()` method. The browser then shows the native install dialog. Whether the user accepts or declines, the banner hides.

### Conditional rendering: iOS vs Android

```rust
move || {
    if !show.get() { return ().into_any(); }
    if is_ios.get() {
        view! {
            <div class="install-banner">
                <div class="install-banner__text">
                    <strong>"Install GrindIt"</strong>
                    <span class="install-banner__sub">
                        "Tap " /* share icon SVG */ " then \"Add to Home Screen\""
                    </span>
                </div>
                <button class="install-banner__close" on:click=dismiss>"x"</button>
            </div>
        }.into_any()
    } else {
        view! {
            <div class="install-banner">
                <div class="install-banner__text">
                    <strong>"Install GrindIt"</strong>
                    <span class="install-banner__sub">
                        "Add to your home screen for the best experience"
                    </span>
                </div>
                <button class="install-banner__btn" on:click=install>"Install"</button>
                <button class="install-banner__close" on:click=dismiss>"x"</button>
            </div>
        }.into_any()
    }
}
```

The iOS banner shows instructions (there is no programmatic install API). The Android banner shows an "Install" button that triggers the native prompt. Both show a dismiss button. The `.into_any()` calls are necessary because the two branches return different concrete view types — `into_any()` erases the type to `AnyView`, which Leptos can render uniformly.

---

## The Service Worker

### Caching strategies

The `sw.js` file implements two caching strategies:

**Network-first** for navigation (HTML pages):

```javascript
if (event.request.mode === "navigate") {
    event.respondWith(
        fetch(event.request)
            .then((response) => {
                if (response.ok) {
                    const clone = response.clone();
                    caches.open(STATIC_CACHE)
                        .then((cache) => cache.put(event.request, clone));
                }
                return response;
            })
            .catch(() => caches.match(event.request))
    );
    return;
}
```

Try the network first. If successful, cache the response for offline use. If the network fails (offline), serve the cached version. This ensures users see the latest content when online, but can still use the app when offline.

**Stale-while-revalidate** for static assets (JS, CSS, images):

```javascript
event.respondWith(
    caches.match(event.request).then((cached) => {
        const fetchPromise = fetch(event.request).then((response) => {
            if (response.ok) {
                const clone = response.clone();
                caches.open(STATIC_CACHE)
                    .then((cache) => cache.put(event.request, clone));
            }
            return response;
        });
        return cached || fetchPromise;
    })
);
```

Return the cached version immediately (fast), while fetching a fresh version in the background (fresh for next time). If no cached version exists, wait for the network response. This gives the best perceived performance for assets that change infrequently.

### DSA connection: Cache invalidation

The service worker uses **versioned cache names** for invalidation:

```javascript
const CACHE_VERSION = "v6";
const STATIC_CACHE = `grindit-static-${CACHE_VERSION}`;

self.addEventListener("activate", (event) => {
    event.waitUntil(
        caches.keys().then((keys) => {
            return Promise.all(
                keys.filter((key) =>
                    key.startsWith("grindit-static-") && key !== STATIC_CACHE
                ).map((key) => caches.delete(key))
            );
        })
    );
});
```

When the service worker updates (because `sw.js` changed), the `activate` event fires. The handler deletes all caches except the current version. This solves the stale cache problem — old cached assets are cleaned up when the new service worker takes over.

Phil Karlton's quote, "There are only two hard things in Computer Science: cache invalidation and naming things," applies directly here. The version string is a simple but effective solution to cache invalidation — bump the version, deploy, and old caches are purged.

### System Design: Caching strategies compared

| Strategy | Use case | Latency | Freshness |
|---|---|---|---|
| Cache-first | Static assets, fonts | Instant | May be stale |
| Network-first | HTML pages, API calls | Network delay | Always fresh when online |
| Stale-while-revalidate | JS/CSS bundles | Instant | Fresh on next load |
| Network-only | Auth endpoints, uploads | Network delay | Always fresh |

GrindIt skips caching for `/auth/` routes (`if (url.pathname.startsWith("/auth/")) return;`) because authentication responses must never be cached — a cached login page could leak session data.

---

## Rust Gym

### Call JS from Rust with js_sys

<details>
<summary>Exercise: read a value from localStorage using pure js_sys</summary>

```rust
fn read_local_storage(key: &str) -> Option<String> {
    let global = js_sys::global();
    let ls = js_sys::Reflect::get(&global, &JsValue::from_str("localStorage"))
        .ok()?;
    if ls.is_undefined() { return None; }
    let get_fn = js_sys::Reflect::get(&ls, &JsValue::from_str("getItem"))
        .ok()?
        .dyn_into::<js_sys::Function>().ok()?;
    let result = get_fn.call1(&ls, &JsValue::from_str(key)).ok()?;
    result.as_string()
}
```

Each `?` short-circuits on failure: missing `localStorage`, missing `getItem` method, call failure, or non-string result. The function returns `None` for any of these, which is the correct behavior — the value simply is not available.
</details>

### Spawn async WASM tasks

<details>
<summary>Exercise: fetch JSON from an API endpoint using spawn_local and JsFuture</summary>

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

fn fetch_data(url: &str, on_done: impl Fn(String) + 'static) {
    let url = url.to_string();
    wasm_bindgen_futures::spawn_local(async move {
        let global = js_sys::global();
        let fetch_fn = js_sys::Reflect::get(&global, &JsValue::from_str("fetch"))
            .unwrap()
            .unchecked_into::<js_sys::Function>();

        let promise = fetch_fn.call1(&global, &JsValue::from_str(&url))
            .unwrap()
            .unchecked_into::<js_sys::Promise>();
        let response = JsFuture::from(promise).await.unwrap();

        let text_fn = js_sys::Reflect::get(&response, &JsValue::from_str("text"))
            .unwrap()
            .unchecked_into::<js_sys::Function>();
        let text_promise = text_fn.call0(&response)
            .unwrap()
            .unchecked_into::<js_sys::Promise>();
        let text = JsFuture::from(text_promise).await.unwrap();

        on_done(text.as_string().unwrap_or_default());
    });
}
```

Two awaits: one for `fetch()` and one for `response.text()`. Both return Promises that become Rust Futures through `JsFuture::from()`. The callback pattern (`on_done`) lets the caller handle the result without blocking.
</details>

### Access DOM APIs with web_sys

<details>
<summary>Exercise: get an element by ID and read its text content using web_sys</summary>

```rust
fn get_element_text(id: &str) -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let element = document.get_element_by_id(id)?;
    element.text_content()
}
```

`web_sys` provides typed bindings — `document.get_element_by_id()` returns `Option<Element>`, not a `JsValue`. The `?` operator chains naturally because every method returns `Option`. Compare this to the `js_sys::Reflect` approach — `web_sys` is more concise when typed bindings exist.
</details>

---

## Exercises

### Exercise 1: Register service worker from Rust using js_sys and spawn_local

Write a `register_service_worker()` function that navigates `window.navigator.serviceWorker`, calls `.register("/sw.js")`, awaits the resulting Promise, and logs success or failure to the console. Use `js_sys::Reflect` for property access and `wasm_bindgen_futures::spawn_local` for async execution.

<details>
<summary>Hints</summary>

- Start with `js_sys::global()` to get the window object
- Use `Reflect::get` to navigate: global -> navigator -> serviceWorker
- Check `sw_container.is_undefined()` — return early if service workers are not supported
- Get the `register` property and cast it to `js_sys::Function` with `unchecked_into`
- Call `register_fn.call1(&sw_container, &JsValue::from_str("/sw.js"))` — the first arg is `this`
- Cast the result to `js_sys::Promise` and wrap in `JsFuture::from()` to await it
</details>

<details>
<summary>Solution</summary>

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

pub fn register_service_worker() {
    let global = js_sys::global();
    let navigator = js_sys::Reflect::get(&global, &JsValue::from_str("navigator"))
        .unwrap_or(JsValue::UNDEFINED);
    let sw_container = js_sys::Reflect::get(&navigator, &JsValue::from_str("serviceWorker"))
        .unwrap_or(JsValue::UNDEFINED);

    if sw_container.is_undefined() { return; }

    wasm_bindgen_futures::spawn_local(async move {
        let register_fn = match js_sys::Reflect::get(
            &sw_container, &JsValue::from_str("register")
        ) {
            Ok(f) if f.is_function() => f.unchecked_into::<js_sys::Function>(),
            _ => return,
        };

        let promise = match register_fn.call1(
            &sw_container, &JsValue::from_str("/sw.js")
        ) {
            Ok(p) => p.unchecked_into::<js_sys::Promise>(),
            Err(_) => return,
        };

        match JsFuture::from(promise).await {
            Ok(_) => { let _ = log_to_console("Service worker registered"); }
            Err(_) => { let _ = log_to_console("SW registration failed"); }
        }
    });
}

fn log_to_console(msg: &str) -> Result<(), JsValue> {
    let global = js_sys::global();
    let console = js_sys::Reflect::get(&global, &JsValue::from_str("console"))?;
    let log_fn = js_sys::Reflect::get(&console, &JsValue::from_str("log"))?
        .unchecked_into::<js_sys::Function>();
    log_fn.call1(&console, &JsValue::from_str(msg))?;
    Ok(())
}
```

The `match` with a guard (`Ok(f) if f.is_function()`) ensures we only call `unchecked_into` on values that are actually functions. Without the guard, casting a non-function `JsValue` to `Function` would cause undefined behavior.
</details>

### Exercise 2: Build theme toggle with inline_js, localStorage, and document.documentElement

Implement a `toggle_theme()` function using `#[wasm_bindgen(inline_js)]`. It should read the current theme from `document.documentElement.getAttribute('data-theme')`, toggle between "dark" and "light", write the new theme to both the DOM attribute and `localStorage`, and return the new theme name.

<details>
<summary>Hints</summary>

- Use `#[wasm_bindgen(inline_js = "export function toggle_theme() { ... }")]`
- Declare the Rust binding in `extern "C" { pub fn toggle_theme() -> String; }`
- In the JS: read `html.getAttribute('data-theme')`, default to `'dark'` if null
- Toggle: `const next = current === 'dark' ? 'light' : 'dark'`
- Write: `html.setAttribute('data-theme', next)` and `localStorage.setItem('theme', next)`
- Add a blocking `<script>` in `<head>` for initial theme application before CSS loads
</details>

<details>
<summary>Solution</summary>

```rust
#[wasm_bindgen(inline_js = "
export function toggle_theme() {
    const html = document.documentElement;
    const current = html.getAttribute('data-theme') || 'dark';
    const next = current === 'dark' ? 'light' : 'dark';
    html.setAttribute('data-theme', next);
    localStorage.setItem('theme', next);
    return next;
}
")]
extern "C" {
    pub fn toggle_theme() -> String;
}
```

In the shell function, add the blocking script for initial theme application:

```rust
<script>"(function(){var t=localStorage.getItem('theme')||'dark';document.documentElement.setAttribute('data-theme',t)})()"</script>
```

The inline JS function is 6 lines. The equivalent `web_sys` code would be ~12 lines with multiple `unwrap()` calls. For a simple, well-understood operation like this, inline JS is the right choice.
</details>

### Exercise 3: Build InstallBanner component with iOS detection and beforeinstallprompt

Build an `InstallBanner` Leptos component that shows a PWA install prompt. It should detect iOS (show "Add to Home Screen" instructions) vs Android/Chrome (show an "Install" button that triggers `beforeinstallprompt`). The banner should not show if the app is already installed or if the user previously dismissed it.

<details>
<summary>Hints</summary>

- Start with `show = RwSignal::new(false)` and `is_ios = RwSignal::new(false)` — neutral initial state prevents hydration mismatch
- Run detection logic in `Effect::new` wrapped in `#[cfg(feature = "hydrate")]`
- Read `__isStandalone`, `__isIos`, and `__pwaInstallPrompt` from the global scope
- Check `localStorage.getItem("pwa_install_dismissed")` for previous dismissal
- The dismiss handler should `show.set(false)` and write to `localStorage`
- The install handler should retrieve `__pwaInstallPrompt` and call `.prompt()` on it
- Use `.into_any()` on both iOS and Android branch views for type erasure
</details>

<details>
<summary>Solution</summary>

The full implementation is in `src/app.rs` as the `InstallBanner` component. Key patterns:

1. **Neutral initial state**: Both `show` and `is_ios` start as `false`. The server renders nothing. After hydration, the `Effect` updates the signals, triggering a reactive DOM update.

2. **Three-layer decision**: standalone check, dismiss check, platform detection. Each layer can short-circuit — if the app is standalone, no further checks run.

3. **Separate install vs dismiss handlers**: The `install` closure calls `__pwaInstallPrompt.prompt()`, the `dismiss` closure writes to `localStorage`. Both hide the banner.

```rust
#[component]
fn InstallBanner() -> impl IntoView {
    let show = RwSignal::new(false);
    let is_ios = RwSignal::new(false);

    Effect::new(move |_| {
        #[cfg(feature = "hydrate")]
        {
            // ... detection logic reading __isStandalone, localStorage, __isIos ...
            // sets show.set(true) and is_ios.set(ios) if banner should display
        }
    });

    let dismiss = move |_| { show.set(false); /* + localStorage persist */ };
    let install = move |_| { /* call __pwaInstallPrompt.prompt() */ show.set(false); };

    move || {
        if !show.get() { return ().into_any(); }
        if is_ios.get() {
            // iOS: instructions view with share icon
            view! { /* ... */ }.into_any()
        } else {
            // Android: Install button + dismiss
            view! { /* ... */ }.into_any()
        }
    }
}
```
</details>

### Exercise 4: Create manifest.json and sw.js with network-first caching strategy

Write the PWA manifest and service worker. The manifest should declare the app name, icons, theme color, and standalone display mode. The service worker should use network-first for navigation requests and stale-while-revalidate for static assets, with versioned cache names for invalidation.

<details>
<summary>Hints</summary>

- `manifest.json` needs: `name`, `short_name`, `start_url: "/"`, `display: "standalone"`, `background_color`, `theme_color`, and `icons` array with 192px and 512px sizes
- Add `purpose: "any maskable"` to icons for adaptive icon support
- `sw.js` events: `install` (precache), `activate` (clean old caches), `fetch` (caching logic)
- Use `self.skipWaiting()` in install and `self.clients.claim()` in activate for immediate takeover
- Skip non-GET requests and cross-origin requests
- Skip `/auth/` routes — never cache authentication responses
- For navigation: `fetch().then(cache).catch(() => caches.match())`
- For assets: `caches.match().then(cached => cached || fetch())`
</details>

<details>
<summary>Solution</summary>

**manifest.json:**

```json
{
  "name": "GrindIt - Body & Mind Tracker",
  "short_name": "GrindIt",
  "description": "Track CrossFit workouts, meditation, breathing, and chanting",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#0f0f1a",
  "theme_color": "#e74c3c",
  "orientation": "portrait-primary",
  "icons": [
    { "src": "/favicon.png", "sizes": "32x32", "type": "image/png" },
    { "src": "/icons/icon-192.png", "sizes": "192x192",
      "type": "image/png", "purpose": "any maskable" },
    { "src": "/icons/icon-512.png", "sizes": "512x512",
      "type": "image/png", "purpose": "any maskable" }
  ]
}
```

**sw.js:**

```javascript
const CACHE_VERSION = "v6";
const STATIC_CACHE = `grindit-static-${CACHE_VERSION}`;
const PRECACHE_URLS = ["/manifest.json"];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(STATIC_CACHE).then((cache) => cache.addAll(PRECACHE_URLS))
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys.filter((k) => k.startsWith("grindit-static-") && k !== STATIC_CACHE)
            .map((k) => caches.delete(k))
      )
    )
  );
  self.clients.claim();
});

self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (url.origin !== location.origin) return;
  if (event.request.method !== "GET") return;
  if (url.pathname.startsWith("/auth/")) return;

  if (event.request.mode === "navigate") {
    event.respondWith(
      fetch(event.request)
        .then((r) => {
          if (r.ok) {
            const c = r.clone();
            caches.open(STATIC_CACHE).then((cache) => cache.put(event.request, c));
          }
          return r;
        })
        .catch(() => caches.match(event.request))
    );
    return;
  }

  event.respondWith(
    caches.match(event.request).then((cached) => {
      const net = fetch(event.request).then((r) => {
        if (r.ok) {
          const c = r.clone();
          caches.open(STATIC_CACHE).then((cache) => cache.put(event.request, c));
        }
        return r;
      });
      return cached || net;
    })
  );
});
```

The version-based cache invalidation is the simplest correct approach. When you deploy a new version, bump `CACHE_VERSION`, and the `activate` handler purges all old caches.
</details>

---

## Summary

This chapter bridged Rust and the browser through three WASM interop techniques:

- **`js_sys`** — low-level access to JavaScript built-in objects via `Reflect::get` and `Function::call`. Verbose but universally available. Used for service worker registration and install banner detection.
- **`#[wasm_bindgen(inline_js)]`** — embed JavaScript directly in the WASM module. Concise for simple browser API calls. Used for theme toggle and video upload.
- **`web_sys`** — typed bindings to Web APIs. Best for complex DOM manipulation where Rust types add safety.

The PWA infrastructure combines these techniques:

- **Service worker**: registered from Rust via `js_sys`, implemented in plain JavaScript for the caching logic
- **Theme toggle**: inline JS function called from a Leptos `on:click` handler, with a blocking `<script>` for initial application
- **Install banner**: Leptos component with `Effect`-based browser detection, conditional rendering for iOS vs Android, and `localStorage` persistence

The hydration mismatch problem — server and client rendering different HTML — is solved by starting with neutral state and updating in `Effect` callbacks after hydration.

The next chapter sets up configuration management and structured logging: multi-environment YAML config, environment variable overrides, Bunyan JSON formatting, and request ID propagation.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds an LRU Cache — the gym bouncer that evicts the least recently used page when storage is full.

**→ [LRU Cache PWA](../ds-narratives/ch14-lru-cache-pwa.md)**

---
