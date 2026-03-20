# Chapter 2: The Exercise Library

In Chapter 1 you built the shell — a header, a bottom nav, a dark theme. It looks like an app, but it has no data. This chapter changes that. You will define the `Exercise` struct, populate it with real CrossFit movements, and render a categorized exercise library that fills the Exercises tab.

By the end of this chapter, you will have:

- An `Exercise` struct with an associated function and a method
- A hardcoded library of 12+ real exercises across 5 categories
- Category grouping with headers, counts, and color-coded left borders
- A clear understanding of how Rust structs compare to TypeScript interfaces and classes

---

## Spotlight: Structs & `impl` Blocks

Every chapter has one spotlight concept. This chapter's spotlight is **structs and `impl` blocks** — the way Rust models data and attaches behavior to it.

### Defining a struct

A struct in Rust is a custom data type that groups related fields together:

```rust
struct Exercise {
    name: String,
    category: String,
    scoring_type: String,
}
```

Each field has a name and a type. There are no default values. There are no optional fields unless you explicitly use `Option<T>`. When you create an instance of this struct, you must provide every field:

```rust
let squat = Exercise {
    name: String::from("Back Squat"),
    category: String::from("weightlifting"),
    scoring_type: String::from("weight_and_reps"),
};
```

### `impl` blocks: adding behavior

Rust does not have classes. Instead, you define data (the struct) and behavior (methods) separately. Methods live in `impl` blocks:

```rust
impl Exercise {
    // Associated function (no `self` parameter) — called with Exercise::new(...)
    fn new(name: &str, category: &str, scoring_type: &str) -> Self {
        Exercise {
            name: name.to_string(),
            category: category.to_string(),
            scoring_type: scoring_type.to_string(),
        }
    }

    // Method (takes `&self`) — called with exercise.summary()
    fn summary(&self) -> String {
        format!("{} [{}] — {}", self.name, self.category, self.scoring_type)
    }
}
```

Two kinds of functions live in `impl` blocks:

- **Associated functions** do not take `self`. They are called with `Type::function_name()`. The most common one is `new()`, which acts as a constructor. In other languages you would write `new Exercise(...)` — in Rust you write `Exercise::new(...)`.
- **Methods** take `&self` (a reference to the instance) as their first parameter. They are called with dot syntax: `squat.summary()`. The `&` means the method borrows the struct without taking ownership — it can read the fields but does not consume the value.

`Self` (capital S) is an alias for the type being implemented. Inside `impl Exercise`, `Self` means `Exercise`. This is a convenience — if you rename the struct, `Self` still works.

### Derive macros: getting behavior for free

In the GrindIt reference codebase, the `Exercise` struct has this decoration:

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Exercise { ... }
```

`#[derive(...)]` tells the compiler to auto-generate trait implementations. Think of traits as interfaces — they define behavior a type must support. The derives above give us:

| Derive | What it provides | Why we need it |
|--------|-----------------|----------------|
| `Clone` | `.clone()` method — deep copy the value | Leptos components often need owned copies of data |
| `Debug` | `{:?}` formatting for `println!` | Debugging during development |
| `Serialize` | Convert to JSON/other formats | Send data over the wire (server to client) |
| `Deserialize` | Parse from JSON/other formats | Receive data from the wire |

You do not need to understand traits deeply yet — Chapter 4 will cover them. For now, think of `derive` as "give me these standard capabilities for free."

> **Coming from JS?**
>
> A Rust struct + impl block is roughly equivalent to a TypeScript interface + class, but split in two:
>
> ```typescript
> // TypeScript: data shape and behavior live together
> interface IExercise {
>   name: string;
>   category: string;
>   scoringType: string;
> }
>
> class Exercise implements IExercise {
>   constructor(
>     public name: string,
>     public category: string,
>     public scoringType: string
>   ) {}
>
>   summary(): string {
>     return `${this.name} [${this.category}] — ${this.scoringType}`;
>   }
> }
> ```
>
> ```rust
> // Rust: data shape and behavior are separate
> struct Exercise {
>     name: String,
>     category: String,
>     scoring_type: String,
> }
>
> impl Exercise {
>     fn new(name: &str, category: &str, scoring_type: &str) -> Self {
>         Exercise {
>             name: name.to_string(),
>             category: category.to_string(),
>             scoring_type: scoring_type.to_string(),
>         }
>     }
>
>     fn summary(&self) -> String {
>         format!("{} [{}] — {}", self.name, self.category, self.scoring_type)
>     }
> }
> ```
>
> Key differences:
> - No `this` keyword — Rust uses `self` (lowercase), and it must be the first parameter
> - No `new` keyword — `Exercise::new()` is just a convention, not language syntax
> - The struct and `impl` can be in different files (though they usually are not)
> - You can have multiple `impl` blocks for the same type — useful for organizing code

---

## Exercise 1: Define the Exercise Struct

**Goal:** Create the `Exercise` struct with a `new()` associated function and a `summary()` method.

### Step 1: Create the data module

Create a new file `src/data.rs`. This will hold our Exercise type and the hardcoded exercise data. Later chapters will move this to a database — for now, everything lives in memory.

```rust
#[derive(Clone, Debug)]
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
}

impl Exercise {
    pub fn new(name: &str, category: &str, scoring_type: &str) -> Self {
        Exercise {
            name: name.to_string(),
            category: category.to_string(),
            scoring_type: scoring_type.to_string(),
        }
    }

    pub fn summary(&self) -> String {
        format!("{} [{}] — {}", self.name, self.category, self.scoring_type)
    }
}
```

### Step 2: Register the module

Open `src/lib.rs` and add the module declaration:

```rust
pub mod app;
pub mod data;
```

### Step 3: Verify it compiles

Save both files. If `cargo leptos watch` is running, it will recompile. No errors means your struct is valid.

<details>
<summary>Hint: If you see "field is never read" warnings</summary>

The compiler warns when struct fields are defined but never used. We have not rendered anything with the struct yet — these warnings will disappear once Exercise 2 reads the fields in the `view!` macro. You can suppress them temporarily with `#[allow(dead_code)]` above the struct, but you will not need to once we render the data.

</details>

### Why `pub` everywhere?

Each `pub` in the struct definition makes that item visible outside the module. Without `pub`, the fields are private to `data.rs` — other modules (like `app.rs`) could not access `exercise.name`. In Rust, everything is private by default. This is the opposite of most languages and is another example of Rust's "explicit is better than implicit" philosophy.

---

## Exercise 2: Build the Exercise Library

**Goal:** Create a hardcoded `Vec<Exercise>` with 12 real CrossFit exercises and render them as cards in a Leptos component.

### Step 1: Add the seed data function

Add this function to `src/data.rs`, below the `impl Exercise` block:

```rust
pub fn seed_exercises() -> Vec<Exercise> {
    vec![
        // Weightlifting
        Exercise::new("Back Squat", "weightlifting", "weight_and_reps"),
        Exercise::new("Deadlift", "weightlifting", "weight_and_reps"),
        Exercise::new("Clean & Jerk", "weightlifting", "weight_and_reps"),
        Exercise::new("Snatch", "weightlifting", "weight_and_reps"),
        // Gymnastics
        Exercise::new("Pull-ups", "gymnastics", "reps_only"),
        Exercise::new("Handstand Push-ups", "gymnastics", "reps_only"),
        Exercise::new("Muscle-ups", "gymnastics", "reps_only"),
        // Conditioning
        Exercise::new("Box Jumps", "conditioning", "reps_only"),
        Exercise::new("Burpees", "conditioning", "reps_only"),
        Exercise::new("Wall Balls", "conditioning", "reps_only"),
        // Cardio
        Exercise::new("400m Run", "cardio", "time"),
        Exercise::new("2000m Row", "cardio", "time"),
        // Mobility
        Exercise::new("Pigeon Stretch", "mobility", "time"),
        Exercise::new("Couch Stretch", "mobility", "time"),
    ]
}
```

Notice how `Exercise::new()` eliminates the boilerplate of writing `String::from(...)` for every field. This is the payoff of writing the associated function — constructing 14 exercises is clean and readable.

### Step 2: Create the ExercisesPage component

Open `src/app.rs`. Add an import for your data module at the top, then create the `ExercisesPage` component:

```rust
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use crate::data::{Exercise, seed_exercises};
```

Now add the component. Place it after the `BottomNav` component:

```rust
#[component]
fn ExercisesPage() -> impl IntoView {
    let exercises = seed_exercises();

    view! {
        <div class="exercises-page">
            <div class="exercises-list">
                {exercises.into_iter().map(|ex| {
                    view! {
                        <div class="exercise-card">
                            <div class="exercise-card__name">{ex.name}</div>
                            <div class="exercise-card__meta">
                                <span class="exercise-card__category">{&ex.category}</span>
                                <span class="exercise-card__scoring">{ex.scoring_type}</span>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
```

Update the `App` component to show the exercises page instead of the placeholder text:

```rust
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/gritwit.css"/>
        <Title text="GrindIt"/>
        <Header/>
        <main>
            <ExercisesPage/>
        </main>
        <BottomNav/>
    }
}
```

### Step 3: Understand the rendering pattern

The key line is:

```rust
exercises.into_iter().map(|ex| { view! { ... } }).collect_view()
```

Let us break this chain down:

1. **`.into_iter()`** — consumes the `Vec<Exercise>` and produces an iterator. Each call to `.next()` yields the next `Exercise`, transferring ownership. (This is why we do not need `&` — the iterator owns each exercise.)
2. **`.map(|ex| { ... })`** — transforms each `Exercise` into a Leptos view. The closure `|ex|` receives an owned `Exercise`.
3. **`.collect_view()`** — collects the iterator of views into a single `View` that Leptos can render. This is Leptos-specific — standard Rust would use `.collect::<Vec<_>>()`.

> **Coming from JS?**
>
> This is equivalent to React's `{exercises.map(ex => <ExerciseCard key={ex.name} ... />)}`. In React, `.map()` returns an array of JSX elements. In Leptos, `.map()` returns an iterator of views, and `.collect_view()` assembles them into a renderable unit.
>
> Note that Rust iterators are **lazy** — nothing happens until `.collect_view()` (or another consumer) drives the iteration. JavaScript's `.map()` eagerly produces an array. This laziness is a performance feature: Rust never allocates intermediate arrays unless you ask for them.

### Step 4: Add the exercise card styles

Create `style/_exercises.scss`:

```scss
.exercises-page {
  padding: 1rem;
}

.exercises-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.exercise-card {
  background: var(--bg-card);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  border-left: 3px solid var(--text-faint);

  &__name {
    font-weight: 600;
    color: var(--text-primary);
    font-size: 0.95rem;
  }

  &__meta {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.25rem;
    font-size: 0.75rem;
  }

  &__category {
    color: var(--accent);
    text-transform: capitalize;
  }

  &__scoring {
    color: var(--text-muted);
  }
}
```

Update `style/main.scss`:

```scss
@use "themes";
@use "reset";
@use "header";
@use "bottom_nav";
@use "exercises";
```

Save everything. You should see 14 exercise cards stacked vertically, each showing the name, category, and scoring type.

<details>
<summary>Hint: If you see a compile error about moved values</summary>

If you use `ex.category` twice in the view (for example, once in a class and once as text), the compiler will complain that the value was moved. The fix is to clone it before the second use:

```rust
let category = ex.category.clone();
// Now use `category` for text and `ex.category` is still available
```

Or, if you only need to display it, use a reference: `{&ex.category}`. Leptos can render `&String` and `&str` as text.

</details>

---

## Exercise 3: Group by Category

**Goal:** Group exercises under category headers with counts, like "Weightlifting (4)".

### Step 1: Add category grouping logic

We need to group our `Vec<Exercise>` into categories. This requires `HashMap` — Rust's hash map type, equivalent to JavaScript's `Map` or Python's `dict`.

Add this import at the top of `src/app.rs`:

```rust
use std::collections::HashMap;
```

Replace the `ExercisesPage` component with the grouped version:

```rust
/// Category display order and colors.
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

    // Group exercises by category
    let mut groups: HashMap<String, Vec<Exercise>> = HashMap::new();
    for ex in exercises {
        groups.entry(ex.category.clone()).or_default().push(ex);
    }

    view! {
        <div class="exercises-page">
            {CATEGORY_ORDER.iter().filter_map(|(key, label)| {
                let cat_exercises = groups.remove(*key)?;
                let count = cat_exercises.len();

                Some(view! {
                    <div class="exercises-section">
                        <div class="exercises-section__header">
                            <span class="exercises-section__label">{*label}</span>
                            <span class="exercises-section__count">
                                {format!("({})", count)}
                            </span>
                        </div>
                        <div class="exercises-section__list">
                            {cat_exercises.into_iter().map(|ex| {
                                view! {
                                    <div class="exercise-card">
                                        <div class="exercise-card__name">{ex.name}</div>
                                        <div class="exercise-card__meta">
                                            <span class="exercise-card__scoring">
                                                {ex.scoring_type}
                                            </span>
                                        </div>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                })
            }).collect_view()}
        </div>
    }
}
```

Let us break down the grouping logic:

**Building the groups:**

```rust
let mut groups: HashMap<String, Vec<Exercise>> = HashMap::new();
for ex in exercises {
    groups.entry(ex.category.clone()).or_default().push(ex);
}
```

- `groups.entry(key)` returns an `Entry` enum — either `Occupied` (key exists) or `Vacant` (key does not exist).
- `.or_default()` inserts an empty `Vec` if the key is missing, then returns a mutable reference to the value.
- `.push(ex)` appends the exercise to that category's vector.

This is the idiomatic Rust way to build a "group by" operation. In JavaScript you would use `reduce()` or a plain `for` loop with `obj[key] = obj[key] || []`.

**Rendering in order:**

```rust
CATEGORY_ORDER.iter().filter_map(|(key, label)| {
    let cat_exercises = groups.remove(*key)?;
    // ...
})
```

`HashMap` has no guaranteed iteration order. To display categories in a consistent order, we iterate over `CATEGORY_ORDER` (a constant array) and pull each category's exercises from the map with `.remove()`. The `?` operator inside `filter_map` skips categories that have no exercises — `remove` returns `None` if the key is absent, and `filter_map` discards `None` values.

### Step 2: Add section header styles

Update `style/_exercises.scss`, adding the section styles:

```scss
.exercises-section {
  margin-bottom: 1.5rem;

  &__header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0;
    margin-bottom: 0.25rem;
  }

  &__label {
    font-weight: 700;
    font-size: 0.85rem;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  &__count {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 400;
  }

  &__list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
}
```

Save and check your browser. You should see the exercises organized under bold category headers with counts.

---

## Exercise 4: Color-Coded Category Borders

**Goal:** Give each category a distinct left-border color on its exercise cards.

### Step 1: Add a category color function

Add this function to `src/data.rs`:

```rust
pub fn category_color(category: &str) -> &'static str {
    match category {
        "weightlifting" => "#3498db",
        "gymnastics"    => "#9b59b6",
        "conditioning"  => "#e74c3c",
        "cardio"        => "#e67e22",
        "mobility"      => "#1abc9c",
        _               => "#888888",
    }
}
```

This is a `match` expression — Rust's pattern matching. It is like a `switch` statement but more powerful:

- Every possible input must be handled (the `_` arm is the catch-all, like `default`)
- The compiler enforces exhaustiveness — if you forget a case, it will not compile
- Each arm returns a value (here, a `&'static str` — a string that lives for the entire program)

The `&'static str` return type deserves a moment. The string literals like `"#3498db"` are baked into the binary at compile time. They exist for the entire lifetime of the program — which Rust calls the `'static` lifetime. You cannot return a `&str` that points to a local variable (it would be a dangling reference), but returning a `&'static str` is always safe because the data never gets freed.

### Step 2: Apply colors in the template

Import `category_color` in `src/app.rs`:

```rust
use crate::data::{Exercise, seed_exercises, category_color};
```

Update the exercise card rendering inside `ExercisesPage` to set the border color dynamically:

```rust
{cat_exercises.into_iter().map(|ex| {
    let color = category_color(&ex.category);
    view! {
        <div
            class="exercise-card"
            style=format!("border-left-color: {}", color)
        >
            <div class="exercise-card__name">{ex.name}</div>
            <div class="exercise-card__meta">
                <span class="exercise-card__scoring">{ex.scoring_type}</span>
            </div>
        </div>
    }
}).collect_view()}
```

The `style=format!(...)` sets an inline style on the element. The `format!` macro is Rust's string interpolation — `{}` is a placeholder that gets replaced with the value. It works like JavaScript template literals (`` `border-left-color: ${color}` ``) or Python f-strings (`f"border-left-color: {color}"`).

### Step 3: Also color the section headers

Add a colored dot to each category header. Update the section header rendering:

```rust
Some(view! {
    <div class="exercises-section">
        <div class="exercises-section__header">
            <div
                class="exercises-section__dot"
                style=format!("background: {}", category_color(key))
            ></div>
            <span class="exercises-section__label">{*label}</span>
            <span class="exercises-section__count">
                {format!("({})", count)}
            </span>
        </div>
        <div class="exercises-section__list">
            // ... exercise cards ...
        </div>
    </div>
})
```

Add the dot style to `style/_exercises.scss`:

```scss
.exercises-section {
  // ... existing styles ...

  &__dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex-shrink: 0;
  }
}
```

Save everything. Each category section now has a colored dot in its header, and each exercise card has a matching colored left border. Weightlifting is blue, gymnastics is purple, conditioning is red, cardio is orange, and mobility is teal.

<details>
<summary>Hint: If the border color does not appear</summary>

Make sure the base `.exercise-card` style has `border-left: 3px solid var(--text-faint);` — we set it in Exercise 2. The inline `style="border-left-color: #3498db"` overrides only the color, not the width or style. If the base rule uses `border-left: none`, there is no border to color.

</details>

---

## The Complete `data.rs`

Here is the full data module after all exercises:

```rust
#[derive(Clone, Debug)]
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
}

impl Exercise {
    pub fn new(name: &str, category: &str, scoring_type: &str) -> Self {
        Exercise {
            name: name.to_string(),
            category: category.to_string(),
            scoring_type: scoring_type.to_string(),
        }
    }

    pub fn summary(&self) -> String {
        format!("{} [{}] — {}", self.name, self.category, self.scoring_type)
    }
}

pub fn category_color(category: &str) -> &'static str {
    match category {
        "weightlifting" => "#3498db",
        "gymnastics"    => "#9b59b6",
        "conditioning"  => "#e74c3c",
        "cardio"        => "#e67e22",
        "mobility"      => "#1abc9c",
        _               => "#888888",
    }
}

pub fn seed_exercises() -> Vec<Exercise> {
    vec![
        Exercise::new("Back Squat", "weightlifting", "weight_and_reps"),
        Exercise::new("Deadlift", "weightlifting", "weight_and_reps"),
        Exercise::new("Clean & Jerk", "weightlifting", "weight_and_reps"),
        Exercise::new("Snatch", "weightlifting", "weight_and_reps"),
        Exercise::new("Pull-ups", "gymnastics", "reps_only"),
        Exercise::new("Handstand Push-ups", "gymnastics", "reps_only"),
        Exercise::new("Muscle-ups", "gymnastics", "reps_only"),
        Exercise::new("Box Jumps", "conditioning", "reps_only"),
        Exercise::new("Burpees", "conditioning", "reps_only"),
        Exercise::new("Wall Balls", "conditioning", "reps_only"),
        Exercise::new("400m Run", "cardio", "time"),
        Exercise::new("2000m Row", "cardio", "time"),
        Exercise::new("Pigeon Stretch", "mobility", "time"),
        Exercise::new("Couch Stretch", "mobility", "time"),
    ]
}
```

---

## Rust Gym

Time for reps. These drills focus on structs and `impl` blocks.

### Drill 1: Struct Surgery

Define a `Workout` struct with fields `name: String`, `rounds: u32`, and `time_cap_minutes: Option<u32>`. Implement a `description()` method that returns a string. If there is a time cap, include it; if not, say "no time cap."

```rust
fn main() {
    let fran = /* your code here */;
    println!("{}", fran.description());
    // Expected: "Fran: 3 rounds, 7 min cap"

    let murph = /* your code here */;
    println!("{}", murph.description());
    // Expected: "Murph: 1 round, no time cap"
}
```

<details>
<summary>Solution</summary>

```rust
struct Workout {
    name: String,
    rounds: u32,
    time_cap_minutes: Option<u32>,
}

impl Workout {
    fn new(name: &str, rounds: u32, time_cap: Option<u32>) -> Self {
        Workout {
            name: name.to_string(),
            rounds,
            time_cap_minutes: time_cap,
        }
    }

    fn description(&self) -> String {
        let round_word = if self.rounds == 1 { "round" } else { "rounds" };
        let cap = match self.time_cap_minutes {
            Some(mins) => format!("{} min cap", mins),
            None => "no time cap".to_string(),
        };
        format!("{}: {} {}, {}", self.name, self.rounds, round_word, cap)
    }
}

fn main() {
    let fran = Workout::new("Fran", 3, Some(7));
    println!("{}", fran.description());

    let murph = Workout::new("Murph", 1, None);
    println!("{}", murph.description());
}
```

The `rounds` field in `Workout::new` uses **field init shorthand** — when the parameter name matches the field name, you can write `rounds` instead of `rounds: rounds`. This is identical to JavaScript's object shorthand (`{ rounds }` instead of `{ rounds: rounds }`).

</details>

### Drill 2: Vec Builder

Write a function `heaviest_exercises` that takes a `Vec<Exercise>` and a minimum weight threshold (as a `&str` scoring type filter — only `"weight_and_reps"` exercises qualify), and returns a `Vec<String>` of their names.

```rust
fn heaviest_exercises(exercises: Vec<Exercise>, scoring_filter: &str) -> Vec<String> {
    // Your code here
}

fn main() {
    let exercises = seed_exercises();
    let names = heaviest_exercises(exercises, "weight_and_reps");
    println!("{:?}", names);
    // Expected: ["Back Squat", "Deadlift", "Clean & Jerk", "Snatch"]
}
```

<details>
<summary>Solution</summary>

```rust
fn heaviest_exercises(exercises: Vec<Exercise>, scoring_filter: &str) -> Vec<String> {
    exercises
        .into_iter()
        .filter(|ex| ex.scoring_type == scoring_filter)
        .map(|ex| ex.name)
        .collect()
}
```

The chain: `into_iter()` consumes the vector, `filter()` keeps only exercises matching the scoring type, `map()` extracts the name from each, and `collect()` gathers the names into a `Vec<String>`. Each step is lazy — nothing executes until `collect()` pulls values through the chain.

</details>

### Drill 3: Match Practice

Write a function `scoring_label` that takes a scoring type string and returns a human-readable label. Handle at least: `"weight_and_reps"` -> `"Weight & Reps"`, `"reps_only"` -> `"Reps"`, `"time"` -> `"Time"`, and a default case.

```rust
fn scoring_label(scoring_type: &str) -> &'static str {
    // Your code here
}

fn main() {
    println!("{}", scoring_label("weight_and_reps")); // "Weight & Reps"
    println!("{}", scoring_label("reps_only"));        // "Reps"
    println!("{}", scoring_label("time"));             // "Time"
    println!("{}", scoring_label("unknown"));          // "Other"
}
```

<details>
<summary>Solution</summary>

```rust
fn scoring_label(scoring_type: &str) -> &'static str {
    match scoring_type {
        "weight_and_reps" => "Weight & Reps",
        "reps_only"       => "Reps",
        "time"            => "Time",
        "distance"        => "Distance",
        "calories"        => "Calories",
        _                 => "Other",
    }
}
```

Every `match` in Rust must be exhaustive — the `_` wildcard arm catches all unmatched values. Remove it and the compiler refuses to build. This is one of Rust's strongest guarantees: you cannot forget to handle a case.

</details>

---

## DSA in Context: Data Modeling

You just used three data structures: `struct`, `Vec`, and `HashMap`. Each maps directly to concepts in database design:

| Rust | Database equivalent | Purpose |
|------|-------------------|---------|
| `struct Exercise` | A table row schema (`CREATE TABLE exercises (...)`) | Defines the shape of a single record |
| `Vec<Exercise>` | A result set (`SELECT * FROM exercises`) | An ordered collection of records |
| `HashMap<String, Vec<Exercise>>` | A `GROUP BY` query result | Records grouped by a key |

The struct is the bridge between in-memory representation and persistent storage. When you add a field to the struct, you are implicitly declaring a column in the future database table. When you choose `String` vs `Option<String>`, you are deciding whether a column is `NOT NULL` or nullable.

This is why getting the struct right matters early. In Chapter 5, when we add SQLx and PostgreSQL, the `Exercise` struct will gain `#[derive(sqlx::FromRow)]` and the database will enforce the same shape. If the struct has `scoring_type: String`, the column will be `scoring_type TEXT NOT NULL`. If it were `Option<String>`, the column would allow `NULL`.

---

## System Design Corner: Domain Modeling

In a system design interview, "domain modeling" is the process of deciding what entities your system has, what properties they carry, and how they relate.

For a fitness tracking app, the core entities might be:

```
Exercise  ──<  WorkoutExercise  >──  WorkoutLog
                                        │
                                      User
```

An `Exercise` is a movement definition (Back Squat, Pull-ups). A `WorkoutLog` is a specific session on a specific date. The many-to-many relationship between them (a workout has multiple exercises, an exercise appears in multiple workouts) is resolved by a join table `WorkoutExercise`.

Decisions you make at the struct level ripple through the entire system:

- **`scoring_type` on Exercise:** This determines what fields the score entry form shows. `"weight_and_reps"` needs weight + reps inputs. `"time"` needs a time input. The struct field drives the UI logic.
- **`category` as a String:** Simple and flexible — adding a new category is just a new string value. But there is no compile-time guarantee that the string is valid. An alternative is an enum (`enum Category { Weightlifting, Gymnastics, ... }`), which the compiler can check but requires a code change to extend. GrindIt uses strings because categories are user-configurable in the admin panel.
- **`id` as String (UUID):** Generated client-side, so exercises can be created offline and synced later without ID collisions. An auto-incrementing integer ID would require the database to assign it, preventing offline creation.

> **Interview talking point:** *"We model the exercise as a struct with category and scoring_type as strings rather than enums because coaches can create custom categories. The trade-off is no compile-time validation, so we validate at the API boundary instead. The struct fields map directly to database columns, and we use derive macros to auto-generate serialization — the same type serves as the API response body and the database row."*

---

## Design Insight: Obvious Code

In *A Philosophy of Software Design*, John Ousterhout argues that the best code is **obvious** — a reader can understand it quickly without deep study. One technique: **choose names that eliminate the need for comments.**

Look at the Exercise struct:

```rust
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
}
```

`exercise.scoring_type` needs no comment. It is the type of scoring for this exercise. `exercise.category` is self-evident. Compare this to a version with abbreviated or generic names:

```rust
pub struct Exercise {
    pub n: String,      // name? number?
    pub cat: String,    // category? catalog?
    pub st: String,     // scoring type? start time? status?
}
```

The abbreviated version saves a few characters of typing and costs minutes of comprehension every time someone reads it. Struct field names ARE documentation. Every time you reach for a comment to explain a field, consider whether a better name would make the comment unnecessary.

This principle extends to function names (`seed_exercises()` not `get_data()`), variable names (`cat_exercises` not `items`), and module names (`data.rs` not `helpers.rs`). Naming is the cheapest form of documentation, and the only form that never goes stale.

---

## What You Built

In this chapter, you:

1. **Defined the `Exercise` struct** — with fields, an associated function (`new`), and a method (`summary`)
2. **Created a seed data function** — 14 exercises across 5 categories, using `Vec<Exercise>`
3. **Grouped exercises by category** — using `HashMap` with the entry API, rendered with `filter_map`
4. **Added color-coded borders** — using `match` expressions and inline styles
5. **Practiced structs, `impl` blocks, and `match`** — the core tools for data modeling in Rust

Your exercise library now has structure and visual hierarchy. But there is no way to find a specific exercise in a long list. In Chapter 3, we will add search and filter — introducing closures, iterators, and Leptos signals for reactive state.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive explores how Rust lays out your Exercise struct in memory — and how field ordering can save kilobytes.

**→ [Struct Memory Layout](../ds-narratives/ch02-struct-memory-layout.md)**

You use `Vec<Exercise>` everywhere — but what actually happens when you `.push()` the 9th exercise into a Vec with room for 8? This deep dive builds a dynamic array from scratch with raw memory allocation.

**→ [Vec: The Dynamic Array — "The Equipment Rack"](../ds-narratives/ch02-vec-dynamic-array.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/data.rs` | [`src/db.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/db.rs) (simplified — no SQLx, no database) |
| `src/app.rs` (ExercisesPage) | [`src/pages/exercises/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/mod.rs) |
| `category_color()` | [`src/pages/exercises/helpers.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/helpers.rs) |
| `style/_exercises.scss` | Exercises styles in the reference app |
