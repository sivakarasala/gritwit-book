// Chapter 6: Multi-Page Routing
// Spotlight: Modules & Project Structure
//
// Router setup with all page routes and active tab highlighting.

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <main class="app">
                <Routes fallback=|| "Page not found">
                    <Route path=path!("/") view=HomePage />
                    <Route path=path!("/exercises") view=ExercisesPage />
                    <Route path=path!("/wod") view=WodPage />
                    <Route path=path!("/history") view=HistoryPage />
                    <Route path=path!("/login") view=LoginPage />
                    <Route path=path!("/profile") view=ProfilePage />
                    <Route path=path!("/admin") view=AdminPage />
                </Routes>
                <BottomNav />
            </main>
        </Router>
    }
}

#[component]
fn BottomNav() -> impl IntoView {
    let location = leptos_router::hooks::use_location();

    let is_active = move |path: &str| {
        let current = location.pathname.get();
        if path == "/" {
            current == "/"
        } else {
            current.starts_with(path)
        }
    };

    view! {
        <nav class="bottom-nav">
            <a class="nav-tab" class:active=move || is_active("/") href="/">
                <span class="nav-icon nav-icon--home"></span>
                <span class="nav-label">"Home"</span>
            </a>
            <a class="nav-tab" class:active=move || is_active("/exercises") href="/exercises">
                <span class="nav-icon nav-icon--exercises"></span>
                <span class="nav-label">"Exercises"</span>
            </a>
            <a class="nav-tab" class:active=move || is_active("/wod") href="/wod">
                <span class="nav-icon nav-icon--wod"></span>
                <span class="nav-label">"WOD"</span>
            </a>
            <a class="nav-tab" class:active=move || is_active("/history") href="/history">
                <span class="nav-icon nav-icon--history"></span>
                <span class="nav-label">"History"</span>
            </a>
        </nav>
    }
}
