// Chapter 1: Hello, GrindIt!
// Spotlight: Variables, Types & the Leptos Toolchain
//
// Basic Leptos app shell with view! macro, header, and bottom nav.

use leptos::prelude::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <main class="app">
            <header class="header">
                <h1>"GrindIt"</h1>
            </header>

            <div class="content">
                <p>"Welcome to GrindIt — your workout tracker."</p>
            </div>

            <nav class="bottom-nav">
                <a class="nav-tab active" href="/">
                    <span class="nav-icon nav-icon--home"></span>
                    <span class="nav-label">"Home"</span>
                </a>
                <a class="nav-tab" href="/exercises">
                    <span class="nav-icon nav-icon--exercises"></span>
                    <span class="nav-label">"Exercises"</span>
                </a>
                <a class="nav-tab" href="/wod">
                    <span class="nav-icon nav-icon--wod"></span>
                    <span class="nav-label">"WOD"</span>
                </a>
                <a class="nav-tab" href="/history">
                    <span class="nav-icon nav-icon--history"></span>
                    <span class="nav-label">"History"</span>
                </a>
            </nav>
        </main>
    }
}
