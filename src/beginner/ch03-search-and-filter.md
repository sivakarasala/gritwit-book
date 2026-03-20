# Chapter 3: Search & Filter

Your exercise library has 14 movements across 5 categories. That is manageable. But the real GrindIt app has dozens — weightlifting, powerlifting, gymnastics, conditioning, cardio, bodybuilding, mobility, yoga, and more. Without search and filter, scrolling becomes a chore.

This chapter makes the library interactive. You will add a search bar that filters exercises in real-time, collapsible category sections, exercise count badges, and expandable detail cards. Along the way, you will learn closures, iterators, and Leptos signals — three powerful concepts that unlock reactive, dynamic user interfaces.

By the end of this chapter, you will have:

- A search input that filters exercises by name as you type
- Collapsible category sections (click the header to toggle)
- Exercise count badges that update dynamically
- Expandable exercise cards showing the scoring type
- A solid understanding of Rust closures, iterator chains, and fine-grained reactivity

This is the chapter where GrindIt starts to feel like a real app. Let us dive in.

---

## Spotlight: Closures & Iterators

### What is a closure?

You have already used closures without knowing it. In Chapter 2, every `|ex| { ... }` in the `.map()` calls was a closure. Now let us understand what that actually means.

A **closure** is a function you can store in a variable, pass to other functions, and call later. It looks like a regular function but uses `|pipes|` instead of `fn` and parentheses:

```rust
// A regular function
fn add_one(x: i32) -> i32 {
    x + 1
}

// A closure that does the same thing
let add_one = |x: i32| x + 1;

// Both are called the same way
println!("{}", add_one(5));  // 6
```

So far, closures look like a shorter way to write functions. But closures have a superpower: they can **capture** variables from their surrounding scope.

```rust
let threshold = 3;
let is_heavy = |reps: u32| reps >= threshold;

println!("{}", is_heavy(5));  // true
println!("{}", is_heavy(2));  // false
```

`is_heavy` is a closure that uses `threshold` even though `threshold` was not passed as a parameter. The closure "captures" it — it reaches into the surrounding code and grabs the variable it needs.

> **Programming Concept: What is a Closure?**
>
> Think of a closure as a **recipe card with ingredients attached**.
>
> A regular function is like a recipe that says: "Take flour, eggs, and sugar, and make a cake." Every time you use the recipe, you need to supply all the ingredients yourself.
>
> A closure is like a recipe card that already has some ingredients taped to it. Maybe it has a bag of flour attached. Now when you use the recipe, you only need to supply the eggs and sugar — the flour is already there.
>
> In code, the "ingredients taped to the card" are the captured variables. The closure carries them wherever it goes. This is incredibly useful in UI programming: a button's click handler can capture the data it needs to work with, rather than looking it up every time it fires.
>
> ```rust
> let exercise_name = String::from("Back Squat");
>
> // This closure captures exercise_name
> let greet = || println!("Time to do {}!", exercise_name);
>
> greet();  // "Time to do Back Squat!"
> greet();  // "Time to do Back Squat!"
> ```

### The `move` keyword

There is one complication with closures that you will encounter frequently in Leptos: the `move` keyword.

```rust
let name = String::from("Back Squat");
let print_name = move || println!("{}", name);
// `name` has been moved into the closure — it is no longer available here
// println!("{}", name);  // ERROR: value used after move
print_name();  // OK: "Back Squat"
```

`move` tells the closure to **take ownership** of the variables it captures, rather than just borrowing them. Think of it as the recipe card not just having flour taped to it, but the flour being *removed from your pantry and given to the card*. The card now owns the flour; your pantry no longer has it.

Why do we need `move`? In Leptos, closures often outlive the scope where they were created. A click handler is stored by the framework and called later — potentially long after the original function has returned. If the closure only borrowed a variable, that variable might have been freed already, leaving the closure pointing at empty memory.

`move` prevents this by giving the closure ownership. The data lives as long as the closure lives. You will see `move` in front of almost every closure in Leptos code.

> **Programming Concept: What is `move`?**
>
> When a closure captures a variable, it can either:
>
> 1. **Borrow** it — "let me look at your book" (the original owner keeps it)
> 2. **Move** it — "give me your book" (the original owner no longer has it)
>
> Borrowing is the default. Rust figures out the cheapest capture mode automatically. But when a closure needs to live independently — stored somewhere, called later — it often needs to *own* its data outright.
>
> `move` forces ownership transfer. After the move, the original variable is gone. This sounds restrictive, but it prevents a dangerous class of bugs: accessing memory that has already been freed.
>
> For now, the rule of thumb is: **if Leptos wants a `move` closure, add `move`**. The compiler will tell you when it is needed.

### What is an iterator?

You used `.into_iter().map().collect_view()` in Chapter 2. Let us understand what iterators really are.

An **iterator** is a way to go through a collection one item at a time. Instead of jumping around with index numbers (`items[0]`, `items[1]`, `items[2]`), you say "give me the next one" repeatedly until there are no more.

The real power of iterators is **chaining** — connecting multiple operations together:

```rust
let exercises = seed_exercises();

let results: Vec<String> = exercises
    .iter()                                      // step onto the conveyor belt
    .filter(|ex| ex.category == "weightlifting") // keep only weightlifting
    .map(|ex| ex.name.clone())                   // extract the name
    .collect();                                  // gather results

// results: ["Back Squat", "Deadlift", "Clean & Jerk", "Snatch"]
```

> **Programming Concept: What is an Iterator?**
>
> Imagine a **conveyor belt** in a factory. Items move along the belt, and at each station, something happens:
>
> 1. **The belt starts** (`.iter()` or `.into_iter()`) — items begin moving
> 2. **Quality control** (`.filter()`) — an inspector checks each item. Items that pass continue; items that fail get removed from the belt
> 3. **Transformation** (`.map()`) — a worker takes each item and transforms it into something new (extracting just the name from a full exercise, for example)
> 4. **Packaging** (`.collect()`) — at the end of the belt, the finished items are gathered into a box
>
> Nothing on the belt moves until someone at the end asks for results. The belt is **lazy** — if nobody collects the output, no work happens. This is different from, say, a Python list comprehension where the entire new list is built immediately.
>
> ```
> [Exercise] → [Exercise] → [Exercise] → [Exercise]
>     ↓ .iter()
> [Exercise] → [Exercise] → [Exercise] → [Exercise]
>     ↓ .filter(weightlifting?)
> [Exercise] → [Exercise]                          (2 removed)
>     ↓ .map(extract name)
> ["Back Squat"] → ["Deadlift"]
>     ↓ .collect()
> Vec: ["Back Squat", "Deadlift"]
> ```

### `.iter()` vs `.into_iter()` — borrowing vs consuming

There is an important distinction between two ways to start an iterator:

| Method | What it yields | Consumes the collection? | When to use |
|--------|---------------|-------------------------|-------------|
| `.iter()` | References (`&Exercise`) | No — the Vec still exists afterward | When you need the data again later |
| `.into_iter()` | Owned values (`Exercise`) | Yes — the Vec is consumed | When you are done with the collection |

In Chapter 2, we used `.into_iter()` because we only needed the exercises once — we rendered them and were done. In this chapter, we will switch to `.iter()` because the search feature re-filters the exercises every time the user types. We need the original list to survive across multiple runs.

```rust
// .iter() — borrows, collection survives
let exercises = seed_exercises();
let names: Vec<&str> = exercises.iter()
    .map(|ex| ex.name.as_str())
    .collect();
// exercises is still available here!

// .into_iter() — consumes, collection is gone
let exercises = seed_exercises();
let names: Vec<String> = exercises.into_iter()
    .map(|ex| ex.name)
    .collect();
// exercises is GONE — it was consumed by into_iter()
```

> **Programming Concept: What is Lazy Evaluation?**
>
> When you write a chain like `.iter().filter().map()`, you might expect each step to process the entire list before passing results to the next step. That is how it works in many languages — each step creates a complete new list.
>
> Rust iterators are **lazy**. Nothing happens until something at the end of the chain asks for a result. When `.collect()` (or `.collect_view()`) says "give me the next item," the chain processes just ONE item through all the steps:
>
> 1. `.collect()` asks `.map()` for an item
> 2. `.map()` asks `.filter()` for an item
> 3. `.filter()` asks `.iter()` for an item
> 4. `.iter()` provides the first exercise
> 5. `.filter()` checks it — if it passes, hand it to `.map()`. If not, ask `.iter()` for the next one
> 6. `.map()` transforms it and hands it to `.collect()`
> 7. `.collect()` stores it and asks for the next
>
> This means: **no intermediate lists are created**. There is no "filtered list" sitting in memory between `.filter()` and `.map()`. Items flow through the pipeline one at a time. For large data sets, this saves a lot of memory.

---

## Exercise 1: Add a Search Bar

**Goal:** Add a text input bound to a reactive signal. As the user types, the exercise list filters in real time.

### Step 1: Understand signals

Before we write the search bar, we need to learn about **signals** — the way Leptos handles values that change over time.

In Chapter 1, all our data was static. The header text never changed. The nav tabs were fixed. But a search bar is different — the user types, and the displayed exercises must update to match. We need a way to say "this value can change, and when it does, update the parts of the page that depend on it."

That is exactly what a signal does.

```rust
let search = RwSignal::new(String::new());
```

`RwSignal` stands for "read-write signal." Think of it as a **special box that Leptos watches**. When you put something new in the box, Leptos notices and updates any part of the page that was looking at the box's contents.

A signal has three main operations:

- **`.get()`** — Look inside the box and read the current value. Leptos notes that you are interested in this box, so it will notify you when the contents change.
- **`.set(value)`** — Replace what is in the box with a new value. Leptos detects the change and re-runs any code that called `.get()` on this signal.
- **`.update(|v| ...)`** — Open the box and modify the contents in place through a closure.

The magic is in the automatic connection. When you call `search.get()` inside a piece of UI code, Leptos remembers "this UI element depends on the `search` signal." Later, when `search.set(...)` is called, Leptos re-runs *only* that specific UI code — not the entire page. This is called **fine-grained reactivity**.

Here is a simple analogy. Imagine a weather display that shows the temperature. The temperature sensor is the signal. When the temperature changes, only the number on the display updates — you do not rebuild the entire weather station. That is fine-grained reactivity.

### Step 2: Add the search signal and filtered list

Now let us put this into practice. Update the `ExercisesPage` component in `src/app.rs`. We need to restructure it to use a signal for the search query:

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

That is a big component. Let us break down the new pieces one at a time.

**The input binding:**

```rust
<input
    prop:value=move || search.get()
    on:input=move |ev| search.set(event_target_value(&ev))
/>
```

These two lines create a **two-way connection** between the text input and the signal:

- `prop:value=move || search.get()` — "The input's displayed text should always match whatever is in the `search` signal." The `prop:` prefix means "set a DOM property." Whenever the signal changes, the displayed text updates. The `move ||` is the closure syntax with the `move` keyword.

- `on:input=move |ev| search.set(event_target_value(&ev))` — "When the user types something, take the new text and put it in the `search` signal." `event_target_value` is a Leptos helper that extracts the current text from the input event.

Together, these create a loop: the signal drives the input display, and the input drives the signal. Type a letter, the signal updates, and everything that depends on the signal re-renders.

**The reactive closure:**

```rust
{move || {
    let q = search.get().to_lowercase();
    let filtered: Vec<&Exercise> = exercises.iter()
        .filter(|ex| {
            q.is_empty() || ex.name.to_lowercase().contains(&q)
        })
        .collect();
    // ... grouping and rendering ...
}}
```

The entire grouping and rendering logic is wrapped in `{move || { ... }}`. This is a closure inside the `view!` macro. Because it calls `search.get()`, Leptos knows this closure depends on the search signal. Every time the user types a letter, `search` changes, and this closure **re-runs automatically**.

This is where iterators come in:

```rust
let filtered: Vec<&Exercise> = exercises.iter()
    .filter(|ex| {
        q.is_empty() || ex.name.to_lowercase().contains(&q)
    })
    .collect();
```

Step by step through the conveyor belt:

1. `exercises.iter()` — Start the belt. Yield references to each exercise (not owned values — we need `exercises` to survive for the next keystroke).

2. `.filter(|ex| { ... })` — Quality control station. For each exercise, check: is the search query empty? If so, keep everything. Otherwise, does the exercise name (lowercased) contain the search query (also lowercased)? Keep it if yes, remove it if no.

3. `.collect()` — Package the survivors into a `Vec<&Exercise>`.

**Why `.iter()` and not `.into_iter()`?** Because this closure runs on every keystroke. If we used `.into_iter()`, the first keystroke would consume the exercises list, and the second keystroke would find nothing to iterate over. `.iter()` borrows the data, leaving the original list intact for next time.

**Why `Vec<&Exercise>` (references) instead of `Vec<Exercise>` (owned)?** Same reason — we are borrowing, not consuming. The `&` means "I am just looking at this data, not taking it."

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

The `font-size: 16px` is not arbitrary — on iOS Safari, any input with `font-size` below 16px triggers an automatic zoom when the user taps on it, which is disorienting. Setting it to 16px prevents this behavior.

Save and test. Type "squat" in the search box — only "Back Squat" should remain. Clear the input and all exercises return. Type "pull" and "Pull-ups" appears. The categories with no matching exercises disappear entirely.

---

## Exercise 2: Collapsible Category Sections

**Goal:** Click a category header to collapse or expand its exercise list.

Sometimes you want to hide a category you are not interested in. Collapsible sections let users focus on what matters.

### Step 1: Add a collapsed signal

We need to track which categories are currently collapsed. A `HashSet` is the right tool — it stores a collection of unique values. If a category's name is in the set, that category is collapsed.

Add this import at the top of `src/app.rs` (if not already present):

```rust
use std::collections::HashSet;
```

Add the signal inside `ExercisesPage`, next to the `search` signal:

```rust
let collapsed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
```

This creates a signal that holds a `HashSet<String>`. Initially the set is empty, meaning all categories are expanded.

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

There is a lot happening here. Let us go through the key patterns.

**The toggle closure (on click):**

```rust
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
```

When you click a category header, this code runs. `.update()` gives you a mutable reference to the signal's inner `HashSet`. If the category is already in the set (collapsed), we remove it (expand). If it is not in the set (expanded), we add it (collapse). This is a toggle.

Notice `let key = key.to_string();` before the `move` closure. Why? The `key` variable comes from `CATEGORY_ORDER` and is a `&&str` — a reference to borrowed data. The `move` closure needs to *own* its data (remember: closures in Leptos outlive the scope where they are created). We convert to an owned `String` so the closure has its own copy.

This is a pattern you will use constantly in Leptos: **clone or convert to owned data before the `move` closure**.

**Conditional rendering:**

```rust
if collapsed.get().contains(&key_owned) {
    ().into_view().into_any()
} else {
    cat_exercises.iter().map(...).collect_view().into_any()
}
```

When collapsed, we render `()` — the "unit type," which is Rust's way of saying "nothing." `.into_view()` converts it to an empty view. When expanded, we render the exercise cards.

The `.into_any()` at the end of both branches is a technical necessity. In Rust, both sides of an `if/else` must return the same type. `().into_view()` and `.collect_view()` are different types. `.into_any()` erases the specific type and wraps both in a common `AnyView` type. Think of it as putting different items in the same type of box.

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

The chevron uses the CSS `mask-image` technique from Chapter 1 — a data URI SVG styled as a CSS mask. When collapsed, it points right (`rotate(-90deg)`). When expanded, it points down (`rotate(0deg)`). The `transition` makes it animate smoothly.

Save and test. Click a category header — its exercises disappear and the chevron rotates. Click again to expand. The search still works while sections are collapsed.

<details>
<summary>Hint: If the closure captures are confusing</summary>

Here are three rules for closures in Leptos:

1. **Signals are cheap to capture.** `RwSignal`, `ReadSignal`, etc. are `Copy` — they can be used inside `move` closures freely. The signal itself is just a small ID number; the actual data lives in Leptos's internal signal graph.

2. **`String` and other owned types must be cloned** before each closure that captures them. A value can only be moved once, but you might need it in multiple closures.

3. **`&str` cannot be moved into a `move` closure** that outlives the borrow. Convert to `String` with `.to_string()` first.

When in doubt: clone before the closure, capture the clone.

</details>

---

## Exercise 3: Dynamic Count Badges

**Goal:** Show a summary bar with the total exercise count and category count, updating as the user searches.

### Step 1: Add the stats bar

Inside the reactive closure (the `{move || { ... }}` block), after computing `filtered` and before the category rendering, add a stats calculation:

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

The counts use `.len()` — the length of the filtered list and the number of groups. Because they are inside the reactive closure, they recompute on every keystroke. Type "squat" and the stats bar shows "1 movement, 1 category." Clear the search and it shows "14 movements, 5 categories."

This is the beauty of reactive programming: we did not write any code to update the counts. We just described *what* the counts should be (length of filtered list, number of groups), and Leptos takes care of *when* to update them.

### Step 2: Per-category counts update automatically

You already have `let count = cat_exercises.len();` in the section header from Chapter 2. This count automatically reflects the search filter because `cat_exercises` is built from the filtered list. Type "pull" and the Gymnastics header shows "(1)" instead of "(3)." Categories with zero matches disappear entirely thanks to `filter_map` skipping empty groups.

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

Save and test. The stats bar should appear below the search input, updating live as you type.

---

## Exercise 4: Expandable Exercise Cards

**Goal:** Click an exercise card to expand it, showing additional details. Only one card can be expanded at a time.

### Step 1: Add the expanded signal

Add another signal to track which exercise is currently expanded:

```rust
let expanded_id = RwSignal::new(Option::<String>::None);
```

Let us decode this type. `Option::<String>::None` says: "This is a `None` value that belongs to `Option<String>`." The `::<String>` part is sometimes called a **turbofish** (because it looks like a fish: `::<>`). It tells the compiler what type the `Option` holds. Without it, the compiler sees `None` and asks "None of what?" — it cannot figure out the type from `None` alone.

The signal holds `Option<String>`:
- `None` means no card is expanded
- `Some("Back Squat".to_string())` means the Back Squat card is expanded

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

This is the most complex piece of code in the chapter. Let us break down the key patterns.

**All those clones at the top:**

```rust
let ex_name = ex.name.clone();
let ex_scoring = ex.scoring_type.clone();
let ex_category = ex.category.clone();
let card_id = ex.name.clone();
let card_id_toggle = card_id.clone();
let card_id_check = card_id.clone();
```

This looks excessive, but there is a reason. We have multiple `move` closures that each need their own copy of the data:
- One closure for the click handler (`card_id_toggle`)
- One closure for the chevron class (`card_id_check`)
- One closure for the expanded details panel (`check_id`, `scoring`, `category`)

Each `move` closure takes ownership of the variables it captures. A value can only be moved once. So before each closure, we clone the values it needs. This is the "clone before the closure" pattern.

**The toggle pattern:**

```rust
expanded_id.update(|current| {
    if current.as_ref() == Some(&card_id_toggle) {
        *current = None;  // Click same card → collapse
    } else {
        *current = Some(card_id_toggle.clone());  // Click different card → expand
    }
});
```

This is similar to the category collapse toggle, but with `Option<String>` instead of `HashSet<String>`. Only one exercise can be expanded at a time — clicking a new card collapses the previous one automatically.

`*current = None` uses the dereference operator `*`. Inside `.update()`, `current` is a `&mut Option<String>` (a mutable reference). The `*` dereferences it so we can assign a new value. Think of it as "reach through the reference and change what is on the other side."

**`.then()` for conditional rendering:**

```rust
is_expanded.then(|| {
    view! { ... }
})
```

`bool.then(|| value)` returns `Some(value)` when the bool is `true`, and `None` when it is `false`. Leptos renders `Some(view)` as the view, and `None` as nothing. This is a concise alternative to `if expanded { Some(view! { ... }) } else { None }`.

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

Save and test. Click an exercise card — it expands to show category and scoring details with a small arrow indicator. Click it again to collapse. Click a different card and the first one collapses automatically. The search still works with expanded cards.

<details>
<summary>Hint: If clicking a card does nothing</summary>

Check that the `on:click` handler is on the outer `.exercise-card` div, not on a child element. Also verify that the signal name matches in both the click handler and the conditional render. A common mistake is having `expanded_id` in the handler but checking a different signal in the view.

If the expanded panel appears but the arrow does not rotate, check that the `class=move || ...` closure reads `expanded_id.get()` (with `.get()`). Without `.get()`, the closure reads the signal once during initial render and never updates.

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

Given a list of rep counts, produce a list of only the even numbers, formatted as "Set X":

```rust
fn format_even_sets(reps: Vec<i32>) -> Vec<String> {
    // Your code here — use .into_iter().filter().map().collect()
}

fn main() {
    let reps = vec![5, 10, 3, 8, 7, 12, 1, 6];
    let result = format_even_sets(reps);
    println!("{:?}", result);
    // Expected: ["Set 10", "Set 8", "Set 12", "Set 6"]
}
```

<details>
<summary>Hint: Breaking it into steps</summary>

Think of the conveyor belt:
1. Start with `reps.into_iter()` — put all numbers on the belt
2. `.filter(|r| ...)` — keep only even numbers. A number is even if `r % 2 == 0` (the remainder when divided by 2 is zero)
3. `.map(|r| ...)` — format each number as "Set X" using `format!("Set {}", r)`
4. `.collect()` — gather the results into a `Vec<String>`

Note: `.filter()` passes a reference (`&i32`) to its closure, while `.map()` passes the owned value (`i32`). This is because `filter` needs to keep the item if it passes — it cannot give it away during the check.

</details>

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

The chain: `into_iter()` consumes the vector, `filter()` keeps even numbers, `map()` formats each one, and `collect()` gathers the results. No intermediate lists are created — items flow through the pipeline one at a time.

</details>

### Drill 2: Closure Captures

Write a closure that captures an external category string and uses it to filter exercises. The closure should "remember" which category to look for:

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
<summary>Hint: Using captured variables in filter</summary>

The closure inside `.filter()` can use variables from the surrounding scope:

```rust
exercises.iter()
    .filter(|ex| ex.category == target_category)  // target_category is captured!
    .map(|ex| ex.name.clone())
    .collect()
```

The filter closure captures `target_category` automatically. It borrows it (reads without consuming) because it only needs to compare values.

</details>

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

The closure `|ex| ex.category == target_category` captures `target_category` from the surrounding scope. The Rust compiler sees that the closure only *reads* `target_category` (it compares but does not modify), so it borrows by shared reference automatically.

If you needed to move `target_category` into the closure (for example, to pass the closure to another function), you would write `move |ex| ex.category == target_category`. But here, a borrow is sufficient.

</details>

### Drill 3: Fold (Accumulating a Result)

Use `.iter().fold()` to calculate the total number of exercises across all categories in a `HashMap`:

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
<summary>Hint: How fold works</summary>

`.fold()` is like a running total. You start with an initial value (the "accumulator") and process each item, updating the accumulator:

```rust
.fold(starting_value, |accumulator, current_item| {
    // return the new accumulator value
})
```

For example, to sum a list of numbers:
```rust
let nums = vec![1, 2, 3, 4, 5];
let total = nums.iter().fold(0, |acc, n| acc + n);
// acc starts at 0, then 0+1=1, 1+2=3, 3+3=6, 6+4=10, 10+5=15
```

For our problem, you want to start with 0 and add the `.len()` of each category's exercise list.

</details>

<details>
<summary>Solution</summary>

```rust
fn total_exercises(groups: &HashMap<String, Vec<Exercise>>) -> usize {
    groups.values().fold(0, |acc, exercises| acc + exercises.len())
}
```

`.values()` iterates over the HashMap's values (each a `Vec<Exercise>`). `.fold(0, |acc, exercises| acc + exercises.len())` starts with 0 and adds each vector's length.

`fold` is a general-purpose accumulator. It can compute sums, build strings, find maximums — anything that involves reducing a collection to a single value.

An alternative that reads a bit more naturally for this specific case:

```rust
fn total_exercises(groups: &HashMap<String, Vec<Exercise>>) -> usize {
    groups.values().map(|v| v.len()).sum()
}
```

`.sum()` is a specialized fold for adding numbers. Both approaches produce the same result. Use whichever reads more clearly to you.

</details>

---

## DSA in Context: Linear Search & String Matching

The search bar uses `.contains()` to find exercises:

```rust
ex.name.to_lowercase().contains(&q)
```

How fast is this? `.contains()` uses a simple approach called **linear search**: for a text of length *n* and a search query of length *m*, the worst case checks every position in the text, which takes roughly *n * m* comparisons.

For our 14 exercises with short names, this is instantaneous — a few hundred comparisons at most. But let us think about scaling:

| Exercises | Query length | Comparisons (worst case) |
|-----------|-------------|--------------------------|
| 14 | 5 | ~70 |
| 1,000 | 10 | ~10,000 |
| 100,000 | 20 | ~2,000,000 |

At 100,000 exercises, running this on every keystroke would cause noticeable lag. In practice, GrindIt will never have 100,000 exercises — a typical gym has under 200. But it is good to think about what would happen if the data grew, because this kind of thinking is what separates a hobbyist from a professional developer.

Real-world solutions for large data sets include:
- **Server-side search**: Send the query to the server and let the database handle it. Databases have indexes optimized for text search. GrindIt does this in production — Chapter 5 introduces database queries.
- **Debouncing**: Wait until the user stops typing for 300 milliseconds before running the search. This avoids searching on every single keystroke.

For our current needs, client-side `.contains()` is perfect. The data is small, the approach is simple, and it works.

---

## Design Insight: Signals Are the Source of Truth

In this chapter, you used three signals:

```rust
let search = RwSignal::new(String::new());
let collapsed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
let expanded_id = RwSignal::new(Option::<String>::None);
```

Each signal is the **single source of truth** for a piece of state:
- `search` is the authority on what the user typed
- `collapsed` is the authority on which categories are hidden
- `expanded_id` is the authority on which card is open

Everything else is *derived* from these signals. The filtered exercise list is derived from `search`. The chevron direction is derived from `collapsed`. The detail panel visibility is derived from `expanded_id`. If you want to change the behavior, you change the signal — all the derived values update automatically.

This is a powerful pattern. In apps without signals (or similar reactive primitives), you might track state in multiple places — a flag here, a class name there — and they get out of sync. With signals, there is one truth and everything follows from it.

As your apps grow more complex, this pattern will save you from entire categories of bugs. If something looks wrong on screen, you only need to check one place: the signal. If the signal is correct, the bug is in the view logic. If the signal is wrong, the bug is in the event handler. The debugging search space shrinks dramatically.

---

## What You Built

In this chapter, you:

1. **Added a reactive search bar** — `RwSignal<String>` bound to an input with two-way data flow
2. **Filtered exercises with iterators** — `.iter().filter().collect()` inside a reactive closure, re-running on every keystroke
3. **Implemented collapsible sections** — `RwSignal<HashSet<String>>` to track collapsed state, toggled via `.update()`
4. **Added dynamic count badges** — exercise and category counts that update automatically as the search narrows results
5. **Built expandable cards** — `RwSignal<Option<String>>` for single-expansion toggling with `.then()` conditional rendering
6. **Practiced closures and iterators** — captures, `move`, lazy evaluation, `filter`/`map`/`fold`/`collect`

The exercise library is now fully interactive — searchable, collapsible, expandable. Everything updates reactively without page reloads.

But all the data is still hardcoded. In Chapter 4, we will add CRUD operations — creating, editing, and deleting exercises — introducing forms, server functions, and Leptos's action system. The exercises you create will persist, and the app will start to feel like something you could actually use.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Trie (prefix tree) for O(m) autocomplete instead of scanning every exercise name from scratch in Rust — no libraries, just std.

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
