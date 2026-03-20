// Chapter 3: Search & Filter
// Spotlight: Closures & Iterators
//
// Search, filter, and expand/collapse with signals and iterator chains.

use leptos::prelude::*;

#[component]
pub fn ExercisesPage() -> impl IntoView {
    // Signals for reactive state
    let search_query = RwSignal::new(String::new());
    let selected_category = RwSignal::new(None::<String>);
    let expanded_exercise = RwSignal::new(None::<String>);

    // Hardcoded exercises (from Ch 2's db.rs in production)
    let exercises = vec![
        ("Back Squat", "Weightlifting"),
        ("Deadlift", "Weightlifting"),
        ("Pull-up", "Gymnastics"),
        ("Running", "Monostructural"),
    ];

    // Filter logic using closures and iterators
    let filtered = move || {
        let query = search_query.get().to_lowercase();
        let category = selected_category.get();

        exercises
            .iter()
            .filter(|(name, cat)| {
                let matches_search = query.is_empty()
                    || name.to_lowercase().contains(&query);
                let matches_category = category.is_none()
                    || category.as_deref() == Some(*cat);
                matches_search && matches_category
            })
            .cloned()
            .collect::<Vec<_>>()
    };

    // Category counts for badges
    let category_count = move |cat: &str| {
        let query = search_query.get().to_lowercase();
        exercises
            .iter()
            .filter(|(name, c)| {
                *c == cat && (query.is_empty() || name.to_lowercase().contains(&query))
            })
            .count()
    };

    // Toggle expand/collapse
    let toggle_expand = move |name: String| {
        expanded_exercise.update(|current| {
            if current.as_deref() == Some(&name) {
                *current = None;
            } else {
                *current = Some(name);
            }
        });
    };

    view! {
        <div class="exercises-page">
            <input
                type="text"
                placeholder="Search exercises..."
                on:input=move |ev| {
                    search_query.set(event_target_value(&ev));
                }
            />
            // Category filter buttons and exercise cards would render here
            // using `filtered()` and `toggle_expand`
        </div>
    }
}
