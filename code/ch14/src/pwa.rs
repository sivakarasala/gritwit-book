// Chapter 14: PWA & WASM Interop
// Spotlight: wasm_bindgen, web_sys, js_sys
//
// Service worker registration, theme toggle, JS interop from Rust.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "
export function register_sw() {
    if ('serviceWorker' in navigator) {
        navigator.serviceWorker.register('/sw.js')
            .then(reg => console.log('SW registered:', reg.scope))
            .catch(err => console.error('SW registration failed:', err));
    }
}
")]
extern "C" {
    fn register_sw();
}

pub fn init_pwa() {
    register_sw();
}

/// Toggle theme between "dark" and "light", persisted in localStorage
pub fn toggle_theme() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let storage = window.local_storage().ok().flatten().expect("no localStorage");

    let body = document.body().expect("no body");
    let current = storage.get_item("theme").ok().flatten().unwrap_or_default();

    let new_theme = if current == "light" { "dark" } else { "light" };

    body.set_attribute("data-theme", new_theme).ok();
    storage.set_item("theme", new_theme).ok();
}

/// Apply saved theme on page load
pub fn apply_saved_theme() {
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let storage = window.local_storage().ok().flatten().expect("no localStorage");

    if let Some(theme) = storage.get_item("theme").ok().flatten() {
        if let Some(body) = document.body() {
            body.set_attribute("data-theme", &theme).ok();
        }
    }
}
