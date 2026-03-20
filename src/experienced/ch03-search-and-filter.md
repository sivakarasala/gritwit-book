# Chapter 3: Search & Filter

Your exercise library has 14 movements across 5 categories. That is manageable. But the real GrindIt app has dozens — weightlifting, powerlifting, gymnastics, conditioning, cardio, bodybuilding, mobility, yoga, and more. Without search and filter, scrolling becomes a chore.

This chapter makes the library interactive. You will add a search bar that filters exercises in real-time, collapsible category sections, exercise count badges, and expandable detail cards. Along the way, you will learn closures, iterators, and Leptos signals — the reactive primitives that make all of this work without page reloads.

By the end of this chapter, you will have:

- A search input that filters exercises by name as you type
- Collapsible category sections (click the header to toggle)
- Exercise count badges that update dynamically
- Expandable exercise cards showing the scoring type
- A solid understanding of Rust closures, iterator chains, and fine-grained reactivity

---

## Spotlight: Closures & Iterators

### Closures

A closure is an anonymous function that can capture variables from its surrounding scope. You have already used them — every `|ex| { ... }` in the `.map()` calls from Chapter 2 was a closure. Let us formalize what is happening.

```rust
let threshold = 3;
let is_heavy = |reps: u32| reps >= threshold;

println!("{}", is_heavy(5));  // true
println!("{}", is_heavy(2));  // false
```

`is_heavy` is a closure. It takes a `u32` parameter and returns a `bool`. It **captures** `threshold` from the enclosing scope — it can read `threshold` even though `threshold` was not passed as an argument.

Closures in Rust come in three flavors, determined by how they capture variables:

| Trait | Captures by | Can be called | Example |
|-------|------------|---------------|---------|
| `Fn` | Shared reference (`&T`) | Multiple times | Reading a signal value |
| `FnMut` | Mutable reference (`&mut T`) | Multiple times | Modifying a counter |
| `FnOnce` | Ownership (move) | Once | Consuming a value |

The compiler infers which trait a closure implements based on what it does with its captures. You rarely need to annotate this yourself. But you do need to know the `move` keyword:

```rust
let name = String::from("Back Squat");
let print_name = move || println!("{}", name);
// `name` has been moved into the closure — it is no longer accessible here
// println!("{}", name);  // ERROR: value used after move
print_name();  // OK
```

`move` forces the closure to take ownership of all captured variables. This is critical in Leptos because closures often outlive the scope where they were created — they are stored in signal callbacks that fire at unpredictable times. Without `move`, the closure would hold references to local variables that no longer exist.

> **Coming from JS?**
>
> JavaScript closures capture variables by reference automatically. There is no concept of "moving" into a closure because JavaScript has garbage collection — variables live as long as anything references them.
>
> ```javascript
> // JavaScript — capture is always by reference
> let name = "Back Squat";
> const printName = () => console.log(name);
> name = "Front Squat";  // perfectly fine
> printName();  // prints "Front Squat" — it sees the mutation
> ```
>
> ```rust
> // Rust — capture semantics depend on usage
> let name = String::from("Back Squat");
> let print_name = || println!("{}", name);  // captures &name (Fn)
> print_name();  // "Back Squat"
> // name is still usable here because it was only borrowed
> ```
>
> The Rust compiler decides the most efficient capture mode automatically. `move` overrides this to force ownership transfer. You will see `move` in almost every Leptos event handler and derived signal.

### Iterators

You used `.into_iter().map().collect_view()` in Chapter 2. Let us go deeper.

An iterator in Rust is any type that implements the `Iterator` trait — meaning it has a `.next()` method that returns `Option<Item>`. When there are no more items, it returns `None`.

The power of iterators is **chaining**. Each adapter (`.filter()`, `.map()`, `.take()`, etc.) wraps the previous iterator, producing a new iterator without allocating intermediate collections:

```rust
let exercises = seed_exercises();

let results: Vec<String> = exercises
    .iter()              // borrow each Exercise (does not consume the Vec)
    .filter(|ex| ex.category == "weightlifting")  // keep only weightlifting
    .map(|ex| ex.name.clone())                     // extract names
    .collect();                                    // materialize into Vec<String>

// exercises is still available here — .iter() only borrowed it
```

**Key distinction: `.iter()` vs `.into_iter()`**

| Method | Yields | Consumes the collection? | Use when... |
|--------|--------|--------------------------|-------------|
| `.iter()` | `&T` (references) | No | You need the collection afterward |
| `.into_iter()` | `T` (owned values) | Yes | You are done with the collection |
| `.iter_mut()` | `&mut T` (mutable refs) | No | You need to modify items in place |

In Leptos components, you often use `.iter()` inside reactive closures because the closure runs multiple times (every time a signal changes). If you used `.into_iter()`, the first invocation would consume the data and the second would fail.

> **Coming from JS?**
>
> JavaScript's `.filter().map()` is eager — each step produces a new array:
>
> ```javascript
> const names = exercises
>   .filter(ex => ex.category === "weightlifting")  // new array
>   .map(ex => ex.name);                            // another new array
> ```
>
> Two intermediate arrays are allocated. With 10,000 exercises, that is two 10,000-element allocations (even though the filter might reduce it to 50).
>
> Rust's iterators are **lazy** — no intermediate allocations happen:
>
> ```rust
> let names: Vec<String> = exercises.iter()
>     .filter(|ex| ex.category == "weightlifting")  // no allocation
>     .map(|ex| ex.name.clone())                     // no allocation
>     .collect();                                    // ONE allocation for the final Vec
> ```
>
> The chain compiles down to a single loop with no intermediate vectors. `.collect()` (or `.collect_view()` in Leptos) is the trigger that drives the entire chain. Until a consumer calls `.next()`, nothing happens.

---

## Exercise 1: Add a Search Bar

**Goal:** Add a text input bound to a reactive signal. As the user types, the exercise list filters in real time.

### Step 1: Understand signals

Leptos uses **signals** for reactive state. A signal is a value that notifies the framework when it changes, triggering re-renders of only the parts of the UI that depend on it.

```rust
let search = RwSignal::new(String::new());
```

`RwSignal` stands for "read-write signal." It provides:

- `.get()` — read the current value (and subscribe to changes)
- `.set(value)` — replace the value (and notify subscribers)
- `.update(|v| ...)` — modify the value in place via a closure

When you call `search.get()` inside a Leptos reactive context (like a closure in the `view!` macro), that closure automatically re-runs whenever `search` changes. This is **fine-grained reactivity** — only the specific DOM nodes that read the signal are updated, not the entire component tree.

### Step 2: Add the search signal and filtered list

Update the `ExercisesPage` component in `src/app.rs`. We need to restructure it to use signals:

```rust
#[component]
fn ExercisesPage() -> impl IntoView {
    let exercises = seed_exercises();
    let search = RwSignal::new(String::new());

    view! {
        <div class="exercises-page">
            <div class="exercises-search">
                <input
                    type="text"
                    class="exercises-search__input"
                    placeholder="Search movements..."
                    prop:value=move || search.get()
                    on:input=move |ev| search.set(event_target_value(&ev))
                />
            </div>
            {move || {
                let q = search.get().to_lowercase();
                let filtered: Vec<&Exercise> = exercises.iter()
                    .filter(|ex| {
                        q.is_empty() || ex.name.to_lowercase().contains(&q)
                    })
                    .collect();

                let mut groups: HashMap<&str, Vec<&Exercise>> = HashMap::new();
                for ex in &filtered {
                    groups.entry(ex.category.as_str()).or_default().push(ex);
                }

                CATEGORY_ORDER.iter().filter_map(|(key, label)| {
                    let cat_exercises = groups.remove(key)?;
                    let count = cat_exercises.len();
                    let color = category_color(key);

                    Some(view! {
                        <div class="exercises-section">
                            <div class="exercises-section__header">
                                <div
                                    class="exercises-section__dot"
                                    style=format!("background: {}", color)
                                ></div>
                                <span class="exercises-section__label">{*label}</span>
                                <span class="exercises-section__count">
                                    {format!("({})", count)}
                                </span>
                            </div>
                            <div class="exercises-section__list">
                                {cat_exercises.into_iter().map(|ex| {
                                    let color = category_color(&ex.category);
                                    view! {
                                        <div
                                            class="exercise-card"
                                            style=format!("border-left-color: {}", color)
                                        >
                                            <div class="exercise-card__name">
                                                {ex.name.clone()}
                                            </div>
                                            <div class="exercise-card__meta">
                                                <span class="exercise-card__scoring">
                                                    {ex.scoring_type.clone()}
                                                </span>
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    })
                }).collect_view()
            }}
        </div>
    }
}
```

Let us unpack the key changes:

**The input binding:**

```rust
<input
    prop:value=move || search.get()
    on:input=move |ev| search.set(event_target_value(&ev))
/>
```

- `prop:value=move || search.get()` — sets the DOM `value` property reactively. Every time `search` changes, the input's displayed text updates. The `prop:` prefix means "set a DOM property" (not an HTML attribute — the distinction matters for form elements).
- `on:input=move |ev| search.set(event_target_value(&ev))` — when the user types, extract the input's current value and store it in the signal. `event_target_value` is a Leptos helper that gets the string value from an input event.

This creates a **two-way binding**: the signal drives the input, and the input drives the signal. In React terms, this is a "controlled input."

**The reactive closure:**

The entire grouping and rendering logic is wrapped in `{move || { ... }}`. This closure re-runs every time `search.get()` is called inside it (which happens on the first line: `let q = search.get().to_lowercase();`). Leptos tracks the dependency automatically.

**References instead of ownership:**

Notice that `filtered` is `Vec<&Exercise>` (references), not `Vec<Exercise>` (owned values). We use `.iter()` instead of `.into_iter()`. This is important because the reactive closure runs on every keystroke — we cannot consume `exercises` on the first run and expect it to exist on the second.

### Step 3: Add search bar styles

Add to `style/_exercises.scss`:

```scss
.exercises-search {
  padding: 0 0 0.75rem;

  &__input {
    width: 100%;
    padding: 0.6rem 0.75rem;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 16px;
    outline: none;
    transition: border-color 0.2s;

    &::placeholder {
      color: var(--text-dim);
    }

    &:focus {
      border-color: var(--accent);
    }
  }
}
```

The `font-size: 16px` is not arbitrary — on iOS Safari, any input with `font-size` below 16px triggers an automatic zoom on focus, which is disorienting. This was mentioned in Chapter 1's reset file, and we enforce it here too.

Save and test. Type "squat" in the search box — only "Back Squat" should remain. Clear the input and all exercises return. Type "pull" and "Pull-ups" appears.

---

## Exercise 2: Collapsible Category Sections

**Goal:** Click a category header to collapse or expand its exercise list.

### Step 1: Add a collapsed signal

We need to track which categories are collapsed. A `HashSet<String>` is perfect — if a category key is in the set, it is collapsed.

Add this import at the top of `src/app.rs` (if not already present):

```rust
use std::collections::HashSet;
```

Add the signal inside `ExercisesPage`, next to the `search` signal:

```rust
let collapsed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
```

### Step 2: Make headers clickable

Update the section header rendering to toggle collapse state on click:

```rust
Some(view! {
    <div class="exercises-section">
        <div
            class="exercises-section__header"
            on:click={
                let key = key.to_string();
                move |_| {
                    collapsed.update(|set| {
                        if set.contains(&key) {
                            set.remove(&key);
                        } else {
                            set.insert(key.clone());
                        }
                    });
                }
            }
        >
            <div
                class="exercises-section__dot"
                style=format!("background: {}", color)
            ></div>
            <span class="exercises-section__label">{*label}</span>
            <span class="exercises-section__count">
                {format!("({})", count)}
            </span>
            <div class=move || {
                let key_str = key.to_string();
                if collapsed.get().contains(&key_str) {
                    "exercises-section__chevron"
                } else {
                    "exercises-section__chevron open"
                }
            }></div>
        </div>
        {
            let key_owned = key.to_string();
            move || {
                if collapsed.get().contains(&key_owned) {
                    // Section is collapsed — render nothing
                    ().into_view().into_any()
                } else {
                    cat_exercises.iter().map(|ex| {
                        let color = category_color(&ex.category);
                        view! {
                            <div
                                class="exercise-card"
                                style=format!("border-left-color: {}", color)
                            >
                                <div class="exercise-card__name">
                                    {ex.name.clone()}
                                </div>
                                <div class="exercise-card__meta">
                                    <span class="exercise-card__scoring">
                                        {ex.scoring_type.clone()}
                                    </span>
                                </div>
                            </div>
                        }
                    }).collect_view().into_any()
                }
            }
        }
    </div>
})
```

Let us break down the key patterns:

**The toggle closure:**

```rust
collapsed.update(|set| {
    if set.contains(&key) {
        set.remove(&key);
    } else {
        set.insert(key.clone());
    }
});
```

`.update()` gives you a mutable reference to the signal's inner value. You modify it in place. Leptos detects the change and re-runs any closures that called `.get()` on this signal.

**Conditional rendering:**

```rust
if collapsed.get().contains(&key_owned) {
    ().into_view().into_any()
} else {
    cat_exercises.iter().map(...).collect_view().into_any()
}
```

`()` (the unit type) rendered as a view produces nothing — an empty DOM node. `.into_any()` is needed because Rust requires both branches of an `if` to return the same type. `().into_view()` and `.collect_view()` return different types, so we erase the type with `.into_any()`.

**Why `let key_owned = key.to_string();` before the closure?**

The `key` variable is a `&&str` (a reference to a reference to a string slice), which is borrowed from `CATEGORY_ORDER`. The closure is `move`, meaning it takes ownership of everything it captures. You cannot move a borrowed reference into a `move` closure that outlives the borrow. Converting to an owned `String` with `.to_string()` gives the closure data it owns outright.

### Step 3: Add the chevron indicator

Add to `style/_exercises.scss`:

```scss
.exercises-section__header {
  cursor: pointer;
  user-select: none;
  -webkit-tap-highlight-color: transparent;
}

.exercises-section__chevron {
  margin-left: auto;
  width: 16px;
  height: 16px;
  background-color: var(--text-muted);
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  -webkit-mask-size: contain;
  mask-size: contain;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  transition: transform 0.2s;
  transform: rotate(-90deg);

  &.open {
    transform: rotate(0deg);
  }
}
```

Save and test. Click a category header — its exercises disappear and the chevron rotates. Click again to expand. The search still works while sections are collapsed.

<details>
<summary>Hint: If the closure captures are confusing</summary>

The general rule for Leptos closures:

1. Signals (`RwSignal`, `ReadSignal`, etc.) are `Copy` — they can be used inside `move` closures without worry. The signal itself is just a small ID; the actual data lives in Leptos's signal graph.
2. `String` and other owned types must be cloned before each closure that captures them, because `move` transfers ownership and a value can only be moved once.
3. `&str` and other references cannot be moved into a `move` closure that outlives them. Convert to `String` first.

When in doubt: clone before the closure, capture the clone.

</details>

---

## Exercise 3: Dynamic Count Badges

**Goal:** Show a summary bar with the total exercise count and category count, updating as the user searches.

### Step 1: Add the stats bar

Inside the reactive closure (the `{move || { ... }}` block), after computing `filtered` and before the category rendering, add a stats calculation and render it:

```rust
{move || {
    let q = search.get().to_lowercase();
    let filtered: Vec<&Exercise> = exercises.iter()
        .filter(|ex| {
            q.is_empty() || ex.name.to_lowercase().contains(&q)
        })
        .collect();

    let total = filtered.len();

    let mut groups: HashMap<&str, Vec<&Exercise>> = HashMap::new();
    for ex in &filtered {
        groups.entry(ex.category.as_str()).or_default().push(ex);
    }

    let cat_count = groups.len();

    view! {
        <div class="exercises-stats">
            <span class="exercises-stats__item">
                <span class="exercises-stats__num">{total}</span>
                " movements"
            </span>
            <span class="exercises-stats__sep">" · "</span>
            <span class="exercises-stats__item">
                <span class="exercises-stats__num">{cat_count}</span>
                " categories"
            </span>
        </div>
        // ... category sections follow ...
    }
}}
```

The counts use `.len()` (total filtered exercises) and `.len()` on the groups map (number of non-empty categories). These recompute on every keystroke because they are inside the reactive closure. When the user types "squat," the stats bar shows "1 movement, 1 category." Clear the search and it shows "14 movements, 5 categories."

### Step 2: Also show per-category counts in the headers

You already have `let count = cat_exercises.len();` in the section rendering from Chapter 2. This count automatically reflects the search filter because `cat_exercises` is built from the filtered list. Type "pull" and the Gymnastics header shows "(1)" instead of "(3)." Categories with zero matches disappear entirely thanks to `filter_map` skipping empty groups.

### Step 3: Add stats bar styles

Add to `style/_exercises.scss`:

```scss
.exercises-stats {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0 1rem;
  font-size: 0.8rem;
  color: var(--text-muted);

  &__num {
    color: var(--accent);
    font-weight: 700;
  }

  &__sep {
    color: var(--text-dim);
  }
}
```

Save and test. The stats bar should appear below the search input, updating as you type.

---

## Exercise 4: Expandable Exercise Cards

**Goal:** Click an exercise card to expand it, showing additional details. Only one card can be expanded at a time.

### Step 1: Add the expanded signal

Add another signal to track which exercise is currently expanded:

```rust
let expanded_id = RwSignal::new(Option::<String>::None);
```

`Option::<String>::None` is a **turbofish** annotation — it tells the compiler that this `None` belongs to `Option<String>`, not `Option<i32>` or `Option<anything_else>`. Without it, the compiler cannot infer the type from `None` alone.

### Step 2: Make cards clickable with an expanded panel

Update the exercise card rendering:

```rust
{cat_exercises.iter().map(|ex| {
    let color = category_color(&ex.category);
    let ex_name = ex.name.clone();
    let ex_scoring = ex.scoring_type.clone();
    let ex_category = ex.category.clone();
    let card_id = ex.name.clone();  // Using name as ID for now
    let card_id_toggle = card_id.clone();
    let card_id_check = card_id.clone();

    view! {
        <div
            class="exercise-card"
            style=format!("border-left-color: {}", color)
            on:click=move |_| {
                expanded_id.update(|current| {
                    if current.as_ref() == Some(&card_id_toggle) {
                        *current = None;
                    } else {
                        *current = Some(card_id_toggle.clone());
                    }
                });
            }
        >
            <div class="exercise-card__header">
                <div class="exercise-card__name">{ex_name}</div>
                <div class=move || {
                    if expanded_id.get().as_ref() == Some(&card_id_check) {
                        "exercise-card__arrow open"
                    } else {
                        "exercise-card__arrow"
                    }
                }></div>
            </div>
            {
                let scoring = ex_scoring.clone();
                let category = ex_category.clone();
                let check_id = card_id.clone();
                move || {
                    let is_expanded = expanded_id.get().as_ref() == Some(&check_id);
                    is_expanded.then(|| {
                        view! {
                            <div class="exercise-card__details">
                                <div class="exercise-card__detail-row">
                                    <span class="exercise-card__detail-label">
                                        "Category"
                                    </span>
                                    <span class="exercise-card__detail-value">
                                        {category.clone()}
                                    </span>
                                </div>
                                <div class="exercise-card__detail-row">
                                    <span class="exercise-card__detail-label">
                                        "Scoring"
                                    </span>
                                    <span class="exercise-card__detail-value">
                                        {scoring.clone()}
                                    </span>
                                </div>
                            </div>
                        }
                    })
                }
            }
        </div>
    }
}).collect_view().into_any()}
```

**The toggle pattern** is the same as the category collapse, but with `Option<String>` instead of `HashSet<String>`. Only one exercise can be expanded at a time. Clicking the same card collapses it; clicking a different card collapses the previous and expands the new one.

**`.then()` for conditional rendering:** `bool.then(|| view! { ... })` returns `Some(view)` when true and `None` when false. Leptos renders `None` as nothing. This is cleaner than an `if/else` when you only need to show or hide something.

### Step 3: Add expanded card styles

Add to `style/_exercises.scss`:

```scss
.exercise-card {
  // ... existing styles ...
  cursor: pointer;
  transition: background 0.2s;

  &:active {
    background: var(--bg-hover);
  }

  &__header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  &__arrow {
    width: 14px;
    height: 14px;
    background-color: var(--text-muted);
    -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
    -webkit-mask-size: contain;
    mask-size: contain;
    -webkit-mask-repeat: no-repeat;
    mask-repeat: no-repeat;
    transition: transform 0.2s;
    transform: rotate(-90deg);
    flex-shrink: 0;

    &.open {
      transform: rotate(0deg);
    }
  }

  &__details {
    margin-top: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  &__detail-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.8rem;
  }

  &__detail-label {
    color: var(--text-muted);
  }

  &__detail-value {
    color: var(--text-primary);
    text-transform: capitalize;
  }
}
```

Save and test. Click an exercise card — it expands to show category and scoring details. Click it again to collapse. Click a different card and the first one collapses automatically.

<details>
<summary>Hint: If clicking a card does nothing</summary>

Check that the `on:click` handler is on the outer `.exercise-card` div, not on a child element. Also verify that the signal name matches in both the click handler and the conditional render. A common mistake is having `expanded_id` in the handler but checking a different signal in the view.

If the expanded panel appears but the chevron does not rotate, check that the `class=move || ...` closure reads `expanded_id.get()` (with `.get()`, not just `expanded_id`). Without `.get()`, it reads the signal once and never updates.

</details>

---

## The Complete `ExercisesPage` Component

Here is the full component after all four exercises. This replaces the version from Chapter 2:

```rust
use std::collections::{HashMap, HashSet};
use leptos::prelude::*;
use crate::data::{Exercise, seed_exercises, category_color};

const CATEGORY_ORDER: &[(&str, &str)] = &[
    ("weightlifting", "Weightlifting"),
    ("gymnastics", "Gymnastics"),
    ("conditioning", "Conditioning"),
    ("cardio", "Cardio"),
    ("mobility", "Mobility"),
];

#[component]
fn ExercisesPage() -> impl IntoView {
    let exercises = seed_exercises();
    let search = RwSignal::new(String::new());
    let collapsed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let expanded_id = RwSignal::new(Option::<String>::None);

    view! {
        <div class="exercises-page">
            <div class="exercises-search">
                <input
                    type="text"
                    class="exercises-search__input"
                    placeholder="Search movements..."
                    prop:value=move || search.get()
                    on:input=move |ev| search.set(event_target_value(&ev))
                />
            </div>
            {move || {
                let q = search.get().to_lowercase();
                let filtered: Vec<&Exercise> = exercises.iter()
                    .filter(|ex| {
                        q.is_empty() || ex.name.to_lowercase().contains(&q)
                    })
                    .collect();

                let total = filtered.len();

                let mut groups: HashMap<&str, Vec<&Exercise>> = HashMap::new();
                for ex in &filtered {
                    groups.entry(ex.category.as_str()).or_default().push(ex);
                }

                let cat_count = groups.len();

                let sections = CATEGORY_ORDER.iter().filter_map(|(key, label)| {
                    let cat_exercises = groups.remove(key)?;
                    let count = cat_exercises.len();
                    let color = category_color(key);

                    Some(view! {
                        <div class="exercises-section">
                            <div
                                class="exercises-section__header"
                                on:click={
                                    let key = key.to_string();
                                    move |_| {
                                        collapsed.update(|set| {
                                            if set.contains(&key) {
                                                set.remove(&key);
                                            } else {
                                                set.insert(key.clone());
                                            }
                                        });
                                    }
                                }
                            >
                                <div
                                    class="exercises-section__dot"
                                    style=format!("background: {}", color)
                                ></div>
                                <span class="exercises-section__label">{*label}</span>
                                <span class="exercises-section__count">
                                    {format!("({})", count)}
                                </span>
                                <div class={
                                    let key = key.to_string();
                                    move || {
                                        if collapsed.get().contains(&key) {
                                            "exercises-section__chevron"
                                        } else {
                                            "exercises-section__chevron open"
                                        }
                                    }
                                }></div>
                            </div>
                            {
                                let key_owned = key.to_string();
                                move || {
                                    if collapsed.get().contains(&key_owned) {
                                        ().into_view().into_any()
                                    } else {
                                        cat_exercises.iter().map(|ex| {
                                            let color = category_color(&ex.category);
                                            let ex_name = ex.name.clone();
                                            let ex_scoring = ex.scoring_type.clone();
                                            let ex_category = ex.category.clone();
                                            let card_id = ex.name.clone();
                                            let card_id_toggle = card_id.clone();
                                            let card_id_check = card_id.clone();

                                            view! {
                                                <div
                                                    class="exercise-card"
                                                    style=format!(
                                                        "border-left-color: {}", color
                                                    )
                                                    on:click=move |_| {
                                                        expanded_id.update(|current| {
                                                            if current.as_ref()
                                                                == Some(&card_id_toggle)
                                                            {
                                                                *current = None;
                                                            } else {
                                                                *current =
                                                                    Some(card_id_toggle.clone());
                                                            }
                                                        });
                                                    }
                                                >
                                                    <div class="exercise-card__header">
                                                        <div class="exercise-card__name">
                                                            {ex_name}
                                                        </div>
                                                        <div class=move || {
                                                            if expanded_id.get().as_ref()
                                                                == Some(&card_id_check)
                                                            {
                                                                "exercise-card__arrow open"
                                                            } else {
                                                                "exercise-card__arrow"
                                                            }
                                                        }></div>
                                                    </div>
                                                    {
                                                        let scoring = ex_scoring.clone();
                                                        let category = ex_category.clone();
                                                        let check_id = card_id.clone();
                                                        move || {
                                                            (expanded_id.get().as_ref()
                                                                == Some(&check_id))
                                                            .then(|| {
                                                                view! {
                                                                    <div
                                                                        class=
                                                                        "exercise-card__details"
                                                                    >
                                                                        <div
                                                                            class=
                                                                            "exercise-card__detail-row"
                                                                        >
                                                                            <span
                                                                                class=
                                                                                "exercise-card__detail-label"
                                                                            >"Category"</span>
                                                                            <span
                                                                                class=
                                                                                "exercise-card__detail-value"
                                                                            >
                                                                                {category.clone()}
                                                                            </span>
                                                                        </div>
                                                                        <div
                                                                            class=
                                                                            "exercise-card__detail-row"
                                                                        >
                                                                            <span
                                                                                class=
                                                                                "exercise-card__detail-label"
                                                                            >"Scoring"</span>
                                                                            <span
                                                                                class=
                                                                                "exercise-card__detail-value"
                                                                            >
                                                                                {scoring.clone()}
                                                                            </span>
                                                                        </div>
                                                                    </div>
                                                                }
                                                            })
                                                        }
                                                    }
                                                </div>
                                            }
                                        }).collect_view().into_any()
                                    }
                                }
                            }
                        </div>
                    })
                }).collect_view();

                view! {
                    <div class="exercises-stats">
                        <span class="exercises-stats__item">
                            <span class="exercises-stats__num">{total}</span>
                            " movements"
                        </span>
                        <span class="exercises-stats__sep">" · "</span>
                        <span class="exercises-stats__item">
                            <span class="exercises-stats__num">{cat_count}</span>
                            " categories"
                        </span>
                    </div>
                    {sections}
                }
            }}
        </div>
    }
}
```

---

## Rust Gym

### Drill 1: Iterator Chain

Given a `Vec<i32>` of rep counts, produce a `Vec<String>` of only even numbers, formatted as "Set X":

```rust
fn format_even_sets(reps: Vec<i32>) -> Vec<String> {
    // Your code here
}

fn main() {
    let reps = vec![5, 10, 3, 8, 7, 12, 1, 6];
    let result = format_even_sets(reps);
    println!("{:?}", result);
    // Expected: ["Set 10", "Set 8", "Set 12", "Set 6"]
}
```

<details>
<summary>Solution</summary>

```rust
fn format_even_sets(reps: Vec<i32>) -> Vec<String> {
    reps.into_iter()
        .filter(|r| r % 2 == 0)
        .map(|r| format!("Set {}", r))
        .collect()
}
```

The chain: `into_iter()` consumes the vector, `filter()` keeps even numbers (the closure captures nothing — `r` is the iterator's current item), `map()` formats each as a string, and `collect()` gathers them into a `Vec<String>`.

Note that `filter` passes `&i32` (a reference) to its closure, while `map` passes `i32` (owned). This is because `filter` needs to keep the item if it passes — it cannot consume it. After the filter, `map` receives the surviving items by value.

</details>

### Drill 2: Closure Captures

Write a closure that captures an external category string and uses it to filter exercises:

```rust
fn main() {
    let exercises = seed_exercises();
    let target_category = String::from("gymnastics");

    let gymnastic_names: Vec<String> = /* your code: use a closure
        that captures target_category */;

    println!("{:?}", gymnastic_names);
    // Expected: ["Pull-ups", "Handstand Push-ups", "Muscle-ups"]
}
```

<details>
<summary>Solution</summary>

```rust
fn main() {
    let exercises = seed_exercises();
    let target_category = String::from("gymnastics");

    let gymnastic_names: Vec<String> = exercises.iter()
        .filter(|ex| ex.category == target_category)
        .map(|ex| ex.name.clone())
        .collect();

    println!("{:?}", gymnastic_names);
}
```

The closure `|ex| ex.category == target_category` captures `target_category` from the surrounding scope by shared reference (`&String`). The compiler infers this is an `Fn` closure because it only reads the captured value.

If you needed to move `target_category` into the closure (for example, to return the closure from a function), you would write `move |ex| ex.category == target_category`. But here, the closure does not outlive `target_category`, so a borrow is sufficient and the compiler chooses it automatically.

</details>

### Drill 3: Fold

Use `.iter().fold()` to calculate the total number of exercises across all categories in a `HashMap<String, Vec<Exercise>>`:

```rust
use std::collections::HashMap;

fn total_exercises(groups: &HashMap<String, Vec<Exercise>>) -> usize {
    // Your code here — use .fold(), not .len() on each individually
}

fn main() {
    let exercises = seed_exercises();
    let mut groups: HashMap<String, Vec<Exercise>> = HashMap::new();
    for ex in exercises {
        groups.entry(ex.category.clone()).or_default().push(ex);
    }

    println!("Total: {}", total_exercises(&groups));
    // Expected: 14
}
```

<details>
<summary>Solution</summary>

```rust
fn total_exercises(groups: &HashMap<String, Vec<Exercise>>) -> usize {
    groups.values().fold(0, |acc, exercises| acc + exercises.len())
}
```

`.values()` iterates over the HashMap's values (each a `Vec<Exercise>`). `.fold(0, |acc, exercises| acc + exercises.len())` starts with an accumulator of `0` and adds each vector's length.

`fold` is Rust's equivalent of JavaScript's `.reduce()`. The first argument is the initial value, the closure receives the accumulator and the current item.

An alternative without `fold`:

```rust
fn total_exercises(groups: &HashMap<String, Vec<Exercise>>) -> usize {
    groups.values().map(|v| v.len()).sum()
}
```

`.sum()` is a specialized fold that adds numbers. Both approaches produce the same result — use whichever reads more clearly to you.

</details>

---

## DSA in Context: Linear Search & String Matching

The search bar uses `.contains()` to find exercises:

```rust
ex.name.to_lowercase().contains(&q)
```

What is the time complexity? `.contains()` uses a naive string search: for a text of length *n* and a pattern of length *m*, the worst case is **O(n * m)** — it checks for a match starting at every position in the text.

For our 14 exercises with short names, this is instantaneous. But consider the scaling:

| Exercises | Pattern length | Comparisons (worst case) |
|-----------|---------------|--------------------------|
| 14 | 5 | 70 |
| 1,000 | 10 | 10,000 |
| 100,000 | 20 | 2,000,000 |

At 100,000 exercises, naive search on every keystroke becomes noticeable. Real-world solutions:

- **Server-side search**: Send the query to the server, let PostgreSQL's `ILIKE` or full-text search handle it. The database has indexes optimized for this. GrindIt does this in production — the search triggers a server function, not a client-side filter.
- **Trie or prefix tree**: O(m) lookup where *m* is the pattern length, regardless of how many exercises exist. Overkill for our use case, but common in autocomplete systems.
- **Inverted index**: Map each word to the documents containing it. This is how search engines work. Libraries like `tantivy` (Rust's equivalent of Lucene) provide this.

For GrindIt, the client-side `.contains()` approach works because the exercise library is small (typically under 200 exercises per gym). When data grows, the solution is to move the search to the server — which is exactly what Chapter 5 (Database Persistence) enables.

---

## System Design Corner: Reactive Systems

You just built a reactive UI without manually updating the DOM. Type in the search box, and the exercise list, counts, and stats bar all update. How?

### Push vs Pull

There are two fundamental approaches to propagating changes:

| Model | How it works | Example |
|-------|-------------|---------|
| **Pull** | Consumer asks for the latest value when it needs it | React's `useState` + re-render: the component re-runs and calls `state` to get the value |
| **Push** | Producer notifies consumers when the value changes | RxJS Observables: the observable pushes new values to subscribers |

Leptos uses a **push-based signal graph**. When you call `search.set("squat")`:

1. The signal stores the new value
2. Leptos walks the dependency graph to find all effects and derived computations that read `search`
3. Those computations re-run, potentially updating the DOM

This is **fine-grained reactivity** — only the closures that called `search.get()` re-run. The `Header` and `BottomNav` components are completely untouched.

### How it compares to React

React re-renders the entire component when state changes, then diffs the virtual DOM to find what actually changed. Leptos skips both steps — there is no virtual DOM and no diffing. The signal graph knows exactly which DOM nodes depend on which signals. This is why Leptos benchmarks favorably against React for update performance.

```
React:                               Leptos:
setState("squat")                    search.set("squat")
  → re-render entire component         → re-run only closures that read `search`
  → diff virtual DOM                    → update specific DOM nodes directly
  → patch real DOM                      → (no diffing step)
```

The trade-off: Leptos's approach requires you to be explicit about reactivity boundaries (wrapping things in `move || { ... }` closures). React's approach is more implicit — everything inside the component "just works" but pays for it with re-rendering cost.

### Signal rules of thumb

1. **Signals are cheap**: `RwSignal::new(...)` allocates a small slot in the signal graph. Create as many as you need.
2. **`.get()` subscribes**: Calling `.get()` inside a reactive context creates a subscription. Call it outside (like in an event handler) and it just reads the value, no subscription.
3. **Derived computations are closures**: `move || { search.get()... }` in the `view!` macro is a derived computation. Leptos re-runs it when its dependencies change.
4. **Keep closures small**: The more code inside a reactive closure, the more work happens on each update. Extract expensive computations into `Memo` (covered in Chapter 6) to cache results.

> **Interview talking point:** *"We use fine-grained reactivity instead of virtual DOM diffing. Each signal maintains a subscriber list, and updates propagate directly to the affected DOM nodes — O(subscribers) per update instead of O(component tree). For a search-as-you-type feature with many rendered items, this avoids the re-render cascade that frameworks like React would trigger on every keystroke."*

---

## What You Built

In this chapter, you:

1. **Added a reactive search bar** — `RwSignal<String>` bound to an input with two-way data flow
2. **Filtered exercises with iterators** — `.iter().filter().collect()` inside a reactive closure
3. **Implemented collapsible sections** — `RwSignal<HashSet<String>>` to track collapsed state, toggled via `.update()`
4. **Added dynamic count badges** — exercise and category counts that update as the search narrows results
5. **Built expandable cards** — `RwSignal<Option<String>>` for single-expansion toggling with `.then()` conditional rendering
6. **Practiced closures and iterators** — captures, `move`, lazy evaluation, `filter`/`map`/`fold`/`collect`

The exercise library is now fully interactive — searchable, collapsible, expandable. But all the data is hardcoded. In Chapter 4, we will add CRUD operations — creating, editing, and deleting exercises — introducing forms, server functions, and Leptos's action system.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Trie (prefix tree) for O(m) autocomplete instead of scanning every exercise name.

**→ [Trie Search](../ds-narratives/ch03-trie-search.md)**

Ever wonder how `RwSignal::set()` magically updates your UI? This deep dive builds a reactive signal system from scratch — dependency tracking, memoization, and the signal graph that makes Leptos tick.

**→ [Leptos Signals: The Reactive Engine — "The Gym Announcement System"](../ds-narratives/ch03-signals-reactivity.md)**

Your `.filter().map().collect()` chains feel like magic — but what actually happens? This deep dive builds the Iterator trait from scratch, implements custom map/filter adaptors, and proves that iterator chains compile to a single loop.

**→ [Iterators — "The Trainer with a Clipboard"](../ds-narratives/ch03-iterators-deep-dive.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/app.rs` (ExercisesPage with signals) | [`src/pages/exercises/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/mod.rs) |
| Search + filter logic | Lines 46-114 of the reference `mod.rs` — `search`, `collapsed`, `filtered`, category grouping |
| Expandable cards | [`src/pages/exercises/exercise_card.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/exercise_card.rs) — `expanded_id` signal |
| `style/_exercises.scss` | Exercises styles in the reference app |
