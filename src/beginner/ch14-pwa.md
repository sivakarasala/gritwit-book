# Chapter 14: PWA & WASM Interop

GrindIt runs in the browser, but it should feel like a native app. This chapter builds the Progressive Web App layer: a service worker for offline support, a theme toggle that persists across sessions, and an install banner that detects iOS vs Android and prompts accordingly. All of this is orchestrated from Rust using `wasm_bindgen`, `js_sys`, and `web_sys` --- Rust code that compiles to WebAssembly and calls JavaScript APIs directly.

The spotlight concept is **WASM-JavaScript interop** --- the bridge between Rust compiled to WebAssembly and the browser's JavaScript runtime. You will see three techniques: `js_sys` for calling built-in JS objects, `web_sys` for DOM and Web API access, and `#[wasm_bindgen(inline_js)]` for embedding custom JavaScript. You will also learn why hydration makes this tricky --- code that runs differently on server vs client will cause mismatches.

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

> **Programming Concept: What is WebAssembly?**
>
> Normally, web browsers run JavaScript. WebAssembly (WASM) is a second language that browsers can run --- but unlike JavaScript, it is a low-level binary format designed for speed.
>
> Think of it this way: JavaScript is like giving someone written instructions in English. They read each step, interpret it, and follow along. WebAssembly is like giving them a pre-built LEGO instruction booklet with diagrams --- faster to follow because there is less interpretation needed.
>
> When you write Rust with Leptos, the compiler produces two outputs:
>
> 1. **A server binary** --- regular compiled Rust that runs on your server
> 2. **A WASM file** --- compiled Rust that runs in the browser
>
> The WASM file makes the page interactive (handling clicks, updating the DOM, calling the server). But WASM lives in a sandbox --- it has no direct access to browser APIs like `localStorage` or `document`. To use those APIs, WASM must call JavaScript through a bridge. That bridge is `wasm_bindgen`.

There are three layers of abstraction for crossing the WASM-JS boundary:

1. **`js_sys`** --- bindings to JavaScript built-in objects (`Promise`, `Function`, `Reflect`, `Array`, etc.). These exist in every JS environment, including Node.js.
2. **`web_sys`** --- bindings to Web APIs (`Window`, `Document`, `Element`, `Navigator`, `ServiceWorkerContainer`, etc.). These exist only in browsers.
3. **`#[wasm_bindgen(inline_js)]`** --- embed raw JavaScript and call it from Rust. The escape hatch for anything not covered by the binding crates.

### js_sys::Reflect --- property access without types

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

Every `Reflect::get` returns `Result<JsValue, JsValue>` --- it can fail if the property does not exist or the object is not extensible. The `unwrap_or(JsValue::UNDEFINED)` pattern converts missing properties to `undefined` rather than panicking, matching JavaScript's own behavior.

> **Programming Concept: What is a Service Worker?**
>
> A service worker is a script that the browser runs in the background, separate from your web page. It acts as a middleman between your app and the network.
>
> Think of it like a personal assistant who intercepts your mail:
>
> - When you send a letter (make a network request), the assistant can either forward it to the post office (network) or check if they already have a copy from last time (cache).
> - When a letter arrives (network response), the assistant can save a copy for next time before handing it to you.
> - If the post office is closed (you are offline), the assistant can give you the saved copy instead.
>
> Service workers enable:
> - **Offline support** --- the app works without internet by serving cached pages
> - **Faster loading** --- cached assets load instantly instead of waiting for the network
> - **Background updates** --- the worker can fetch fresh data even when the page is not focused
>
> The browser requires service workers to be registered before they start working. That registration is what we will do from Rust.

### wasm_bindgen_futures::spawn_local

Browser APIs return Promises. Rust's async functions return Futures. `spawn_local` bridges the two --- it takes a Rust Future and runs it on the browser's microtask queue:

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

The `Effect::new` ensures the code runs after hydration --- when the WASM client has taken over from the server-rendered HTML. Without the `Effect`, the code would run during SSR and fail.

> **Programming Concept: What is Hydration?**
>
> When you visit a Leptos app, the server sends pre-rendered HTML so the page appears instantly. But that HTML is static --- buttons do not work, forms do not submit. The browser then downloads the WASM code, which "hydrates" the static HTML by attaching event listeners and making everything interactive.
>
> Think of it like building a model house:
>
> 1. **Server-side rendering (SSR)** --- the server builds the house structure (walls, roof, windows). It looks like a house, but nothing works. The lights do not turn on, the doors do not open.
> 2. **Hydration** --- the WASM code arrives and wires everything up. Now the lights work, doors open, and buttons respond to clicks.
>
> The tricky part: the WASM code expects to find the exact same HTML structure that the server produced. If the server renders a `<div>` but the client expects a `<span>`, Leptos detects a **hydration mismatch** and the UI breaks.
>
> This matters for PWA features because the server does not know browser-specific information (is the user on iOS? what is their theme preference?). The solution: render a neutral initial state on both server and client, then update after hydration using `Effect`.

### The hydration mismatch problem

Server-side rendering generates HTML. The client hydrates that HTML --- attaching event listeners and making it interactive. If the server renders different HTML than the client expects, Leptos detects a **hydration mismatch** and may throw errors or produce broken UI.

This matters for PWA features because:

- The server does not know if the user is on iOS
- The server does not know the user's theme preference
- The server does not know if the app is installed

The solution: render a neutral initial state on both server and client, then update in an `Effect` after hydration. The `InstallBanner` starts with `show = false` on both server and client. After hydration, an `Effect` checks the browser state and may set `show = true`. The DOM updates reactively --- no mismatch.

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

Let us walk through the step-by-step navigation through JavaScript objects. This is verbose compared to JavaScript, but every step is explicit about what could go wrong:

1. **`js_sys::global()`** --- returns `window` in a browser, `globalThis` in Node.js.
2. **`Reflect::get(&global, "navigator")`** --- accesses `window.navigator`. In JavaScript, you would just write `window.navigator`. In Rust-WASM, every property access is a function call.
3. **`Reflect::get(&navigator, "serviceWorker")`** --- accesses `navigator.serviceWorker`. If the browser does not support service workers (e.g., older iOS WebView), this is `undefined`, and we return early.
4. **`Reflect::get(&sw, "register")`** --- gets the `register` method as a `JsValue`.
5. **`unchecked_into::<js_sys::Function>()`** --- casts the `JsValue` to a `Function`. The "unchecked" means we trust that `register` is indeed a function. The `.is_function()` guard protects us.
6. **`register_fn.call1(&sw_container, &JsValue::from_str("/sw.js"))`** --- calls `navigator.serviceWorker.register("/sw.js")`. The first argument (`&sw_container`) is the `this` binding. The second is the function argument.
7. **`JsFuture::from(promise).await`** --- converts a JavaScript `Promise` into a Rust `Future` and awaits it.

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

This is four lines of Rust to call `console.log(msg)`. The verbosity illustrates why `#[wasm_bindgen(inline_js)]` exists --- for frequently used browser APIs, inline JS is dramatically shorter.

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

> **Programming Concept: What is localStorage?**
>
> Every web browser provides a small storage area called `localStorage` that persists across page loads and browser restarts. It is a simple key-value store --- you save a string under a name, and you can retrieve it later.
>
> Think of it like a sticky note on your monitor:
>
> - `localStorage.setItem('theme', 'dark')` --- write "dark" on a sticky note labeled "theme"
> - `localStorage.getItem('theme')` --- read the sticky note labeled "theme"
> - The sticky note stays even if you close and reopen the browser
>
> We use it to remember the user's theme preference. When they choose "light" mode, we save it to localStorage. On the next visit, we read it back before the page renders, so they never see a flash of the wrong theme.
>
> Limitations:
> - Only stores strings (no numbers, objects, or arrays directly)
> - Limited to about 5 MB per domain
> - Synchronous (blocks the thread while reading/writing --- fine for small values)
> - Not available in private/incognito mode on some browsers

Why inline JS instead of pure Rust `web_sys`? The pure Rust equivalent would be:

```rust
// Pure web_sys approach --- 12 lines vs 6 lines of inline JS
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

The theme must be applied before the page renders --- otherwise the user sees a flash of the wrong theme. GrindIt handles this with a blocking `<script>` in the HTML `<head>`:

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

The Rust `toggle_theme()` function is only called when the user clicks the theme button --- by that point, hydration is complete and the WASM bridge is active.

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

The `#[cfg(feature = "hydrate")]` guard ensures `toggle_theme()` is only called on the client. On the server, the `on:click` handler compiles to nothing --- the button renders but clicking it does nothing until hydration completes. After hydration, the WASM event listener takes over and the toggle works.

---

## The InstallBanner Component

### PWA install detection

> **Programming Concept: What is a PWA?**
>
> A Progressive Web App (PWA) is a website that can be "installed" on your phone or computer and behave like a native app. After installation:
>
> - It gets its own icon on your home screen
> - It opens in a standalone window (no browser address bar)
> - It can work offline using cached data
> - It can receive push notifications
>
> The browser decides whether a website qualifies as a PWA by checking for:
>
> 1. A `manifest.json` file that describes the app (name, icons, colors)
> 2. A registered service worker for offline support
> 3. HTTPS (secure connection)
>
> Different browsers handle installation differently:
> - **Android/Chrome**: Shows a banner or fires a `beforeinstallprompt` event that the app can capture and use to show a custom install button
> - **iOS/Safari**: No automatic prompt. The user must manually tap Share and then "Add to Home Screen"
>
> Our `InstallBanner` component handles both platforms with different UI.

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

- **`__pwaInstallPrompt`** --- the captured `BeforeInstallPromptEvent`, or `null` if not fired
- **`__isIos`** --- `true` if the user agent matches iOS devices
- **`__isStandalone`** --- `true` if the app is already running in standalone mode (installed)

These are set in a `<script>` tag rather than in Rust because the `beforeinstallprompt` event fires early --- often before WASM loads. If we waited for WASM initialization, we would miss the event.

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

The detection logic runs in an `Effect` after hydration. Let us trace through the decision tree:

1. **Already standalone?** Do not show the banner --- the app is already installed.
2. **Previously dismissed?** Check `localStorage` for the dismiss flag. If set, do not show.
3. **iOS device?** Show the iOS-specific banner with "Add to Home Screen" instructions.
4. **Has install prompt?** Show the Android/Chrome banner with an "Install" button.
5. **Neither?** Do not show --- the browser does not support PWA installation.

Notice how signals start as `false` on both server and client. This neutral initial state prevents hydration mismatches. The `Effect` only updates the signals after hydration, when the WASM code has taken over.

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

The dismiss handler does two things: hides the banner immediately (`show.set(false)`) and persists the dismissal to `localStorage`. On the next page load, the `Effect` checks this flag and skips showing the banner. The `let _ =` prefix on `f.call2()` explicitly discards the `Result` --- if `localStorage.setItem` fails (e.g., private browsing mode with full storage), the banner still hides.

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

The iOS banner shows instructions (there is no programmatic install API). The Android banner shows an "Install" button that triggers the native prompt. Both show a dismiss button. The `.into_any()` calls are necessary because the two branches return different concrete view types --- `into_any()` erases the type to `AnyView`, which Leptos can render uniformly. This is the same pattern you saw in Chapter 9 with scoring types.

---

## The Service Worker

### Caching strategies

> **Programming Concept: What is Caching?**
>
> Caching means keeping a copy of data so you do not have to fetch it again. You do this naturally: when you memorize a phone number, you are caching it. Next time you need it, you recall from memory instead of looking it up.
>
> Web caching works the same way. The browser can save copies of pages, images, and scripts so that on the next visit, it serves them from the local copy instead of downloading them again. This makes pages load faster and enables offline use.
>
> The challenge is **cache invalidation** --- knowing when the cached copy is outdated. If you memorize a phone number and the person changes it, your "cache" is stale. Service workers solve this by using different strategies for different types of content:
>
> - **Network-first**: Always try the network. Only use the cache if the network fails. Best for content that changes frequently (HTML pages).
> - **Cache-first**: Always use the cache. Only try the network if the cache is empty. Best for content that rarely changes (fonts, large images).
> - **Stale-while-revalidate**: Use the cache immediately (fast), but also fetch a fresh copy in the background for next time. Best for assets that change occasionally (JS bundles, CSS).

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

When the service worker updates (because `sw.js` changed), the `activate` event fires. The handler deletes all caches except the current version. This solves the stale cache problem --- old cached assets are cleaned up when the new service worker takes over.

Phil Karlton's quote, "There are only two hard things in Computer Science: cache invalidation and naming things," applies directly here. The version string is a simple but effective solution --- bump the version, deploy, and old caches are purged.

### System Design: Caching strategies compared

| Strategy | Use case | Latency | Freshness |
|---|---|---|---|
| Cache-first | Static assets, fonts | Instant | May be stale |
| Network-first | HTML pages, API calls | Network delay | Always fresh when online |
| Stale-while-revalidate | JS/CSS bundles | Instant | Fresh on next load |
| Network-only | Auth endpoints, uploads | Network delay | Always fresh |

GrindIt skips caching for `/auth/` routes (`if (url.pathname.startsWith("/auth/")) return;`) because authentication responses must never be cached --- a cached login page could leak session data.

---

## Rust Gym

These drills practice the WASM interop patterns in isolation. They are simpler than the chapter exercises --- the goal is to get comfortable crossing the Rust-JS boundary.

### Drill 1: Read a value from localStorage using js_sys

<details>
<summary>Exercise: write a function that reads a string from localStorage</summary>

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

Each `?` short-circuits on failure: missing `localStorage`, missing `getItem` method, call failure, or non-string result. The function returns `None` for any of these, which is the correct behavior --- the value simply is not available.
</details>

### Drill 2: Get an element's text using web_sys

<details>
<summary>Exercise: write a function that gets an element by ID and returns its text content</summary>

```rust
fn get_element_text(id: &str) -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let element = document.get_element_by_id(id)?;
    element.text_content()
}
```

Compare this to the `js_sys::Reflect` approach --- `web_sys` is more concise when typed bindings exist. The `?` operator chains naturally because every method returns `Option`. No `unchecked_into` or `dyn_into` needed.
</details>

### Drill 3: Write an inline_js function

<details>
<summary>Exercise: write an inline_js function that returns the current page URL</summary>

```rust
#[wasm_bindgen(inline_js = "
export function get_current_url() {
    return window.location.href;
}
")]
extern "C" {
    pub fn get_current_url() -> String;
}
```

The pattern is always the same: define a JavaScript function with `export`, declare the Rust signature in `extern "C"`. The `wasm_bindgen` macro generates the bridge code.
</details>

---

## Exercises

### Exercise 1: Register service worker from Rust using js_sys and spawn_local

**Goal:** Write a `register_service_worker()` function that tells the browser to start the service worker.

**Instructions:**

1. Create `src/pwa.rs` with a `register_service_worker()` function
2. Get the global object with `js_sys::global()`
3. Navigate to `navigator.serviceWorker` using `Reflect::get`
4. Check if `serviceWorker` is undefined --- return early if the browser does not support it
5. Inside `spawn_local`, get the `register` method and check that it is a function
6. Call `register_fn.call1(&sw_container, &JsValue::from_str("/sw.js"))`
7. Convert the result to a `Promise`, wrap in `JsFuture::from()`, and await it
8. Log success or failure to the console

<details>
<summary>Hint 1: The match guard pattern</summary>

```rust
let register_fn = match js_sys::Reflect::get(&sw_container, &JsValue::from_str("register")) {
    Ok(f) if f.is_function() => f.unchecked_into::<js_sys::Function>(),
    _ => return,
};
```

The `if f.is_function()` guard ensures we only cast values that are actually functions. Without it, `unchecked_into` on a non-function value would cause undefined behavior.
</details>

<details>
<summary>Hint 2: Converting Promise to Future</summary>

```rust
let promise = result.unchecked_into::<js_sys::Promise>();
let future_result = JsFuture::from(promise).await;
```

`JsFuture::from` converts a JavaScript `Promise` into a Rust `Future`. You can then `.await` it like any other async operation.
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

**Goal:** Implement a theme toggle that switches between dark and light modes and remembers the choice.

**Instructions:**

1. Write a `toggle_theme()` function using `#[wasm_bindgen(inline_js = "...")]`
2. In the JavaScript: read the current theme from `document.documentElement.getAttribute('data-theme')`, defaulting to `'dark'`
3. Toggle: if current is `'dark'`, switch to `'light'`; if `'light'`, switch to `'dark'`
4. Write the new theme to both the DOM attribute and localStorage
5. Return the new theme name
6. Add a blocking `<script>` in `<head>` for initial theme application before CSS loads (to prevent flash of wrong theme)

<details>
<summary>Hint: The blocking script for initial theme</summary>

This script runs before any CSS or JavaScript loads, so the page always renders with the correct theme:

```html
<script>
  (function() {
    var t = localStorage.getItem('theme') || 'dark';
    document.documentElement.setAttribute('data-theme', t);
  })()
</script>
```

Place this in the Leptos shell function's `<head>` section.
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

**Goal:** Build a Leptos component that shows a platform-appropriate install prompt.

**Instructions:**

1. Create an `InstallBanner` component with `show: RwSignal<bool>` and `is_ios: RwSignal<bool>` --- both starting as `false`
2. In an `Effect` (guarded with `#[cfg(feature = "hydrate")]`), check three things:
   - Is the app already running standalone? If yes, do not show.
   - Has the user previously dismissed? Check `localStorage` for `"pwa_install_dismissed"`. If yes, do not show.
   - Is this iOS or does a `beforeinstallprompt` event exist? If yes, show the banner.
3. Create a `dismiss` handler that sets `show` to `false` and saves `"1"` to `localStorage` under `"pwa_install_dismissed"`
4. Create an `install` handler that retrieves `window.__pwaInstallPrompt` and calls `.prompt()` on it
5. Render different UI for iOS (instructions) vs Android (Install button)
6. Use `.into_any()` on both branches for type erasure

<details>
<summary>Hint 1: Reading global JavaScript variables from Rust</summary>

The `<script>` in `<head>` sets variables like `window.__isStandalone`. Read them in Rust:

```rust
let standalone = js_sys::Reflect::get(
    &global, &JsValue::from_str("__isStandalone")
).unwrap_or(JsValue::FALSE)
    .as_bool().unwrap_or(false);
```

`.as_bool()` converts a `JsValue` to `Option<bool>`. If the value is not a boolean (or is undefined), it returns `None`, and `.unwrap_or(false)` gives a safe default.
</details>

<details>
<summary>Hint 2: Why signals start as false</summary>

Both `show` and `is_ios` start as `false`. The server renders nothing (the banner is hidden). After hydration, the `Effect` runs browser checks and may set `show` to `true`. This two-phase approach prevents hydration mismatches --- the server and client agree on the initial HTML, and the `Effect` updates happen after hydration is complete.
</details>

<details>
<summary>Solution</summary>

The full implementation is in `src/app.rs` as the `InstallBanner` component. Key patterns:

1. **Neutral initial state**: Both `show` and `is_ios` start as `false`. The server renders nothing. After hydration, the `Effect` updates the signals, triggering a reactive DOM update.

2. **Three-layer decision**: standalone check, dismiss check, platform detection. Each layer can short-circuit --- if the app is standalone, no further checks run.

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

**Goal:** Write the PWA manifest and service worker files.

**Instructions:**

1. Create `public/manifest.json` with: `name`, `short_name`, `start_url: "/"`, `display: "standalone"`, `background_color` (dark theme), `theme_color` (red accent), and `icons` array with 192px and 512px sizes
2. Add `purpose: "any maskable"` to icons for adaptive icon support on Android
3. Create `public/sw.js` with three event listeners:
   - **`install`**: precache `manifest.json`, call `self.skipWaiting()` for immediate activation
   - **`activate`**: delete old versioned caches, call `self.clients.claim()` for immediate control
   - **`fetch`**: skip non-GET, cross-origin, and `/auth/` requests. Use network-first for navigation, stale-while-revalidate for everything else
4. Use a versioned cache name like `grindit-static-v6` for cache invalidation

<details>
<summary>Hint 1: Why skipWaiting and clients.claim?</summary>

Without `skipWaiting()`, a new service worker waits until all tabs are closed before activating. Without `clients.claim()`, the new worker does not control existing pages until they are refreshed. Together, these ensure the new worker takes over immediately --- important for deploying bug fixes.
</details>

<details>
<summary>Hint 2: Why clone responses?</summary>

```javascript
const clone = response.clone();
caches.open(STATIC_CACHE).then((cache) => cache.put(event.request, clone));
return response;
```

A `Response` can only be consumed once (reading its body empties it). To both cache the response AND return it to the browser, you must clone it first. One copy goes to the cache, the other goes to the browser.
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

## DSA in Context: Cache Invalidation

The service worker's versioned cache name is a solution to the **cache invalidation problem** --- knowing when a cached copy is outdated and should be replaced.

**Interview version:** "Design a caching layer for an API. How do you ensure clients always get fresh data without sacrificing performance?"

**Common strategies:**

- **TTL (Time-To-Live)**: Each cached item expires after a fixed duration. Simple but imprecise --- data might change before the TTL expires.
- **Version-based**: Bump a version number when data changes. Clients check the version and discard stale entries. This is what our service worker does.
- **Event-based**: When data changes, send a notification to invalidate relevant caches. More complex but most precise.

**Bonus challenge:** Our service worker invalidates ALL cached assets when the version bumps. How would you implement more granular invalidation --- where only changed assets are re-fetched?

---

## System Design Corner: Caching Strategies

**Interview question:** "Design an offline-first mobile app."

**What we just built:** A PWA with service worker caching, theme persistence, and platform-aware install prompting.

**Talking points:**

- **Network-first vs cache-first** --- use network-first for dynamic content (HTML pages change with new data), cache-first for static assets (CSS and JS change only on deploy). The wrong choice either shows stale data or causes unnecessary network requests.
- **Offline detection** --- the service worker's `catch` handler on navigation requests provides transparent offline support. The user does not need to know they are offline --- cached pages just work.
- **Platform detection** --- iOS and Android have fundamentally different PWA install flows. Detecting the platform and adapting the UI is essential for a good user experience.
- **Cache size management** --- browsers impose limits on cache storage (typically 50-100 MB per origin). A video-heavy app would need to implement LRU eviction.

---

## Design Insight: Neutral Initial State

> When building features that depend on client-side detection (platform, preferences, installed state), always start with a neutral initial state that works identically on server and client. Update the state in an `Effect` after hydration. This pattern --- neutral render, then client-side enrichment --- prevents hydration mismatches and works correctly even if JavaScript is slow to load or fails entirely. The user sees a reasonable default, then gets the enhanced experience once the client code runs.

---

## What You Built

This chapter bridged Rust and the browser through three WASM interop techniques:

- **`js_sys`** --- low-level access to JavaScript built-in objects via `Reflect::get` and `Function::call`. Verbose but universally available. Used for service worker registration and install banner detection.
- **`#[wasm_bindgen(inline_js)]`** --- embed JavaScript directly in the WASM module. Concise for simple browser API calls. Used for theme toggle and video upload.
- **`web_sys`** --- typed bindings to Web APIs. Best for complex DOM manipulation where Rust types add safety.

The PWA infrastructure combines these techniques:

- **Service worker**: registered from Rust via `js_sys`, implemented in plain JavaScript for the caching logic
- **Theme toggle**: inline JS function called from a Leptos `on:click` handler, with a blocking `<script>` for initial application
- **Install banner**: Leptos component with `Effect`-based browser detection, conditional rendering for iOS vs Android, and `localStorage` persistence

If you run the app now, you should see a theme toggle button in the header (sun/moon icons). Click it and the color scheme switches. Reload the page --- the theme persists. On a mobile device (or Chrome DevTools mobile simulation), you may see the install banner appear at the bottom of the screen.

The next chapter sets up configuration management and structured logging: multi-environment YAML config, environment variable overrides, Bunyan JSON formatting, and request ID propagation.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds an LRU Cache — the gym bouncer that evicts the least recently used page when storage is full from scratch in Rust — no libraries, just std.

**→ [LRU Cache PWA](../ds-narratives/ch14-lru-cache-pwa.md)**

---
