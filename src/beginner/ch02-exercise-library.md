# Chapter 2: The Exercise Library

In Chapter 1, you built the shell of GrindIt — a header, a bottom nav, a dark theme. It looks like an app, but it is empty. There is no data. This chapter changes that.

You are going to define what an exercise *is* in Rust, create a list of real CrossFit movements, and render them on screen as a categorized exercise library. This is where your app starts to feel real.

By the end of this chapter, you will have:

- An `Exercise` struct with an associated function and a method
- A hardcoded library of 14 real exercises across 5 categories
- Category grouping with headers, counts, and color-coded left borders
- A solid understanding of how Rust organizes data and behavior

---

## Spotlight: Structs & `impl` Blocks

Every chapter has one spotlight concept. This chapter's spotlight is **structs and `impl` blocks** — the way Rust models data and attaches behavior to it.

### What is a data structure?

Before we write code, let us step back and talk about a fundamental idea.

A **data structure** is a way to organize related information together. Think of it as a form you fill out. A gym membership form might have fields for "name," "email," and "membership type." Each field has a name, and each field expects a specific kind of information — text, a number, a date.

You have already used data structures without realizing it. A `Vec` is a data structure — it organizes a list of similar things in order. A `String` is a data structure — it organizes a sequence of characters. These are built into Rust.

But what about information that is specific to *your* program? GrindIt needs to represent exercises, workouts, and scores. Rust does not have a built-in "Exercise" type. You need to create your own.

In Rust, the primary tool for creating custom data types is the **struct**:

```rust
struct Exercise {
    name: String,
    category: String,
    scoring_type: String,
}
```

This says: "An `Exercise` is a thing that has a name, a category, and a scoring type. All three are text." That is it. No hidden behavior, no inherited methods, no surprises. A struct is a straightforward container for related data.

> **Programming Concept: What is a Data Structure?**
>
> A data structure is a way to organize information so your program can work with it. You already know some:
>
> - A **list** (`Vec` in Rust) stores items in order, like a shopping list
> - A **string** (`String`) stores a sequence of characters, like a sentence
>
> A **struct** is different from a list. Instead of storing many *similar* things, it stores a *bundle* of different things that belong together. An exercise has a name AND a category AND a scoring type. These are different pieces of information, but they all describe the same exercise, so they belong in one package.
>
> Real-world analogy: think of a **contact card** on your phone. It has fields for name, phone number, email, and address. You would not store those as four separate lists — "all names in one list, all phone numbers in another" — because they belong together. A struct keeps them together.
>
> Other real-world structs:
> - A **recipe**: title, ingredients (a list), instructions (a list), prep time
> - A **student record**: name, ID number, grades (a list), graduation year
> - A **weather report**: location, temperature, humidity, wind speed

When you create an instance of a struct, you must provide **every** field. Rust does not guess. There are no default values. There are no optional fields unless you explicitly say so (we will learn about `Option` shortly). This strictness catches mistakes early — you cannot accidentally create an exercise without a name.

```rust
let squat = Exercise {
    name: String::from("Back Squat"),
    category: String::from("weightlifting"),
    scoring_type: String::from("weight_and_reps"),
};
```

If you forget a field, the compiler tells you immediately. In a language without this strictness, you might not discover the missing field until your app crashes in front of a user.

### `impl` blocks: adding behavior

You have written standalone functions — `fn main()`, `fn greet(name: &str)`. These are free-floating: they do not "belong to" any particular type.

A **method** is a function that is attached to a specific data type. It answers the question: "What can this type *do*?"

In Rust, you do not put methods inside the struct definition. Instead, you define data (the struct) and behavior (methods) separately, in an `impl` block (short for "implementation"):

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

> **Programming Concept: What is a Method?**
>
> A method is a function that belongs to a specific data type. Think of it this way:
>
> - A **struct** is like a noun — "an exercise," "a workout," "a user"
> - A **method** is like a verb — "summarize," "display," "calculate"
>
> When you attach verbs to nouns, you get clear and readable code:
>
> ```rust
> exercise.summary()       // "exercise, summarize yourself"
> workout.total_time()     // "workout, what is your total time?"
> user.display_name()      // "user, what is your display name?"
> ```
>
> The alternative — standalone functions — works too, but reads less naturally:
>
> ```rust
> summarize(exercise)      // "summarize this exercise" — fine, but...
> total_time(workout)      // who does this belong to?
> display_name(user)       // harder to discover when exploring a codebase
> ```
>
> Methods make code easier to read AND easier to discover. When you type `exercise.` in your editor, it can show you all the methods available on that type. With standalone functions, you would need to know the function name already.

There are two kinds of functions inside an `impl` block. Let us look at each:

**Associated functions** do not take `self`. They are called with `Type::function_name()`. The most common one is `new()`, which acts like a constructor — it creates a new instance:

```rust
let squat = Exercise::new("Back Squat", "weightlifting", "weight_and_reps");
```

The `::` syntax means "this function belongs to the `Exercise` type." Think of it as going to a factory: "Exercise factory, please make me a new one with these values."

Why write `new()` instead of creating the struct directly? Compare:

```rust
// Without new() — verbose, lots of String::from()
let squat = Exercise {
    name: String::from("Back Squat"),
    category: String::from("weightlifting"),
    scoring_type: String::from("weight_and_reps"),
};

// With new() — clean and concise
let squat = Exercise::new("Back Squat", "weightlifting", "weight_and_reps");
```

When you create 14 exercises, the difference is dramatic. The `new()` function handles the `String::from()` conversions internally, so the calling code stays clean.

**Methods** take `&self` as their first parameter. The `self` refers to the specific instance the method is called on. The `&` means "borrow" — the method can read the data without taking ownership of it:

```rust
let description = squat.summary();
// description is now: "Back Squat [weightlifting] — weight_and_reps"
```

When you write `squat.summary()`, Rust automatically passes `squat` as `self` to the method. After calling `.summary()`, `squat` is still available — the method only borrowed it to look at its fields.

`Self` (capital S) inside an `impl` block is an alias for the type being implemented. Inside `impl Exercise`, `Self` means `Exercise`. It is a convenience — if you rename the struct later, `Self` still works without changes.

### Derive macros: getting behavior for free

In the code we are about to write, the `Exercise` struct has a decoration above it:

```rust
#[derive(Clone, Debug)]
pub struct Exercise { ... }
```

This `#[derive(...)]` line tells the Rust compiler to automatically generate some common behavior for our struct. You do not have to write the code yourself — the compiler writes it for you.

| Derive | What it gives you | Why we need it |
|--------|-----------------|----------------|
| `Clone` | A `.clone()` method that makes a deep copy of the value | Leptos components often need their own copy of data |
| `Debug` | The ability to print the struct with `{:?}` in `println!` | Seeing what is inside a struct while debugging |

> **Programming Concept: What is `derive`?**
>
> Imagine you just filled out a new recipe card (created a struct). You would probably want to be able to:
>
> - **Photocopy it** — make an exact copy to give to a friend (that is `Clone`)
> - **Read it out loud** — describe its contents so someone can write them down (that is `Debug`)
>
> These are common, predictable tasks. The steps for photocopying a recipe card are always the same: copy each field, one by one. Rust knows this, so it offers to do the work automatically.
>
> `#[derive(Clone, Debug)]` says: "Rust, please auto-generate Clone and Debug for this struct." The compiler looks at each field, confirms it supports Clone and Debug (`String` supports both), and generates the code.
>
> Without `derive`, you would need to write something like:
>
> ```rust
> impl Clone for Exercise {
>     fn clone(&self) -> Self {
>         Exercise {
>             name: self.name.clone(),
>             category: self.category.clone(),
>             scoring_type: self.scoring_type.clone(),
>         }
>     }
> }
> ```
>
> That is tedious and error-prone — if you add a new field to the struct, you might forget to clone it here. `derive` handles it automatically and stays in sync when the struct changes.
>
> Later in the book, you will add more derives like `Serialize` and `Deserialize` for sending data over the network. For now, `Clone` and `Debug` are all we need.

### What `pub` means and why it matters

You will see the word `pub` in front of many items in our code:

```rust
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
}
```

`pub` is short for "public." It means "other parts of the program can see and use this."

In Rust, **everything is private by default**. If you write a struct without `pub`, only the file it lives in can see it. If you write struct fields without `pub`, other files can see that the struct exists but cannot read its fields.

Why does Rust do this? Privacy is a safety feature. Imagine you have a struct with a field called `internal_score` that should only be changed by specific logic inside the module. If it were public, any code anywhere could modify it, potentially putting the struct into an invalid state. By keeping it private, you control who can access it.

For our exercise library, we *want* the struct and its fields to be accessible from `app.rs` (where we render them), so we mark everything `pub`.

Think of it like a building. Every room is locked by default. If you want visitors to enter a room, you unlock the door and put a "Public" sign on it. `pub` is that sign. The rooms you do not mark remain locked — only people inside that part of the building (that module) can access them.

Here is how `pub` applies at different levels:

```rust
pub struct Exercise { ... }      // The struct itself is visible outside the module
    pub name: String,            // The field is readable from outside
    pub fn new(...) -> Self      // The function can be called from outside
    pub fn summary(&self) -> ... // The method can be called from outside
```

If you forget a `pub`, the compiler will tell you with a clear error message. It is one of those things you adjust as needed — start private, add `pub` when something else needs access.

### `String` vs `&str` — owned vs borrowed

You noticed that the struct fields are `String`, but the `new()` function takes `&str` parameters. Why are there two string types in Rust?

Here is an analogy that makes this click. Think of a **library book**:

- **`&str`** (a string slice) is like **reading a library book at the library**. You can look at the pages, copy passages, and enjoy the content. But you do not own the book. When the library closes (when the owner's scope ends), the book goes back on the shelf. You cannot take it home. If you try, the librarian (the compiler) stops you.

- **`String`** is like **buying your own copy of the book**. It is yours. You can highlight text, write in the margins, lend it to a friend, or keep it forever. Nobody else controls when it gets thrown away — it stays alive as long as you hold onto it.

In code:
- `&str` is a *reference* to text data owned by someone else. String literals like `"Back Squat"` are `&str` — they are baked into the compiled program binary, and you just borrow a view of them.
- `String` is *owned* text data stored on the heap. Your variable is responsible for it. When the variable goes out of scope, the String is freed.

Why does the struct use `String` for its fields? Because the struct needs to **own** its data. A struct can live for a long time — it might get passed between functions, stored in a list, cloned, and sent across threads. If its fields were `&str` (borrowed), the original data might get freed while the struct is still alive, which would be a dangling reference — a pointer to freed memory. Rust prevents this at compile time.

Why does `new()` accept `&str` parameters? Because it is the cheapest form of input. String literals are `&str`, and accepting `&str` means the caller does not need to create a `String` first. Inside `new()`, we convert to owned `String` with `.to_string()`:

```rust
fn new(name: &str, category: &str, scoring_type: &str) -> Self {
    Exercise {
        name: name.to_string(),       // borrow in, owned copy out
        category: category.to_string(),
        scoring_type: scoring_type.to_string(),
    }
}
```

The pattern is: **accept the cheapest form as input, convert to what you need inside**.

You do not need to fully master this distinction right now. The compiler will tell you when you have the wrong type and usually suggests the fix. Just remember: `&str` is borrowed (cheap, temporary), `String` is owned (yours to keep), and `.to_string()` converts from borrowed to owned.

---

## Exercise 1: Define the Exercise Struct

**Goal:** Create the `Exercise` struct with a `new()` associated function and a `summary()` method.

### Step 1: Create the data module

Create a new file called `src/data.rs`. This will hold our Exercise type and the hardcoded exercise data. Later chapters will move this to a database — for now, everything lives in memory.

Type this into `src/data.rs`:

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

Take a moment to read through this. You are defining:

1. **A struct** with three `String` fields, all `pub` (public)
2. **An associated function** `new()` that takes three `&str` arguments and returns a new `Exercise`
3. **A method** `summary()` that reads the exercise's fields and returns a formatted description

The `format!` macro works like `println!` but instead of printing to the terminal, it returns a `String`. The `{}` placeholders get replaced with the values — `self.name`, `self.category`, and `self.scoring_type`. So `format!("{} [{}]", "Squat", "weightlifting")` produces the string `"Squat [weightlifting]"`.

### Step 2: Register the module

Rust does not automatically discover new files. You need to tell it about them. Open `src/lib.rs` and add the module declaration:

```rust
pub mod app;
pub mod data;
```

This line says: "There is a module called `data`, defined in the file `src/data.rs`. Make it public so other modules can use it."

Without this line, Rust does not know `data.rs` exists. The file could sit there forever and the compiler would ignore it. Every module must be explicitly declared.

### Step 3: Verify it compiles

Save both files. If `cargo leptos watch` is running, it will recompile automatically. No errors means your struct is valid.

You might see warnings like "field `name` is never read." That is the compiler being helpful — it is telling you that you defined fields but have not used them anywhere yet. Those warnings will disappear in Exercise 2 when we render exercises in the view.

<details>
<summary>Hint: If you see "field is never read" warnings</summary>

The compiler warns when struct fields are defined but never used. We have not rendered anything with the struct yet — these warnings will disappear once Exercise 2 reads the fields in the `view!` macro. You can suppress them temporarily with `#[allow(dead_code)]` above the struct, but you will not need to once we render the data.

</details>

---

## Exercise 2: Build the Exercise Library

**Goal:** Create a hardcoded `Vec<Exercise>` with 14 real CrossFit exercises and render them as cards in a Leptos component.

### Step 1: Add the seed data function

A "seed" function provides initial data for your app — like planting seeds in a garden before anything grows. Add this function to `src/data.rs`, below the `impl Exercise` block:

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

Look at how clean this is. Without the `Exercise::new()` function, each exercise would need three `String::from(...)` calls — that is 42 calls for 14 exercises. The `new()` function handles the conversion internally, so the calling code stays readable.

Let us break down the function signature:

- `pub fn seed_exercises()` — a public function named `seed_exercises` that takes no arguments
- `-> Vec<Exercise>` — it returns a `Vec` (a growable list) of `Exercise` values

`vec![...]` is a macro that creates a `Vec` with the items you list inside. It is like writing a shopping list — you enumerate the items, and Rust packages them into a collection.

### Step 2: Create the ExercisesPage component

Open `src/app.rs`. Add an import for your data module at the top:

```rust
use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use crate::data::{Exercise, seed_exercises};
```

The `use crate::data::...` line says: "From our project's `data` module, bring in the `Exercise` type and the `seed_exercises` function so we can use them by name in this file." Without this line, you would have to write `crate::data::Exercise` and `crate::data::seed_exercises()` every time — the `use` statement is a convenient shortcut.

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

This is the most complex piece of code so far. Let us walk through it slowly.

The first line, `let exercises = seed_exercises();`, calls our seed function to get a list of 14 exercises.

Then inside the `view!` macro, we have some familiar HTML-like divs. The interesting part is this chain:

```rust
exercises.into_iter().map(|ex| { view! { ... } }).collect_view()
```

Think of this as an **assembly line** in a factory:

1. **`.into_iter()`** — The raw materials enter the line. This converts the `Vec<Exercise>` into an **iterator** — a thing that hands you one item at a time. Think of a conveyor belt: exercises line up and come to you one by one.

2. **`.map(|ex| { ... })`** — Each exercise passes through a workstation. The `|ex|` is a **closure** — a mini-function that receives one exercise and transforms it. In this case, it wraps the exercise data in HTML (the `view!` macro). So each raw exercise becomes a formatted card.

3. **`.collect_view()`** — The finished products come off the line and get packaged together. This gathers all the individual card views into one combined view that Leptos can render on screen.

The result: 14 exercises go in, 14 styled cards come out.

Do not worry if the closure syntax (`|ex| { ... }`) feels unfamiliar. We will cover closures in depth in Chapter 3. For now, just know that `|ex|` means "for each exercise, which I will call `ex`."

Now update the `App` component to show the exercises page instead of the placeholder text:

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

### Step 3: Add the exercise card styles

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

Update `style/main.scss` to include the new file:

```scss
@use "themes";
@use "reset";
@use "header";
@use "bottom_nav";
@use "exercises";
```

Save everything. You should see 14 exercise cards stacked vertically, each showing the name, category, and scoring type. Take a moment to appreciate what you built — a Rust struct, a seed function, an iterator chain, and styled cards, all working together.

<details>
<summary>Hint: If you see a compile error about moved values</summary>

If you use `ex.category` twice in the view (for example, once in a class and once as text), the compiler will complain that the value was "moved" — meaning it was used up and is no longer available. The fix is to either clone it first:

```rust
let category = ex.category.clone();
// Now use `category` for one place and `ex.category` for another
```

Or use a reference: `{&ex.category}`. Leptos can render `&String` and `&str` as text. The `&` means "borrow this value" — look at it without consuming it.

</details>

---

## Exercise 3: Group by Category

**Goal:** Group exercises under category headers with counts, like "Weightlifting (4)".

Right now all 14 exercises sit in one flat list. That works with 14, but a real gym might have 50+ exercises. Grouping by category — weightlifting, gymnastics, conditioning — makes the page scannable at a glance.

### Step 1: Understand the plan

We need to:
1. Take our flat list of exercises
2. Sort them into buckets by category (all weightlifting exercises together, all gymnastics together, etc.)
3. Display each bucket under a heading

For step 2, we need a new data structure: `HashMap`. Think of a `HashMap` as a **dictionary** or **lookup table**. You give it a key (like a word), and it gives you back a value (like a definition). Our keys will be category names, and our values will be lists of exercises:

```
"weightlifting" => [Back Squat, Deadlift, Clean & Jerk, Snatch]
"gymnastics"    => [Pull-ups, Handstand Push-ups, Muscle-ups]
"conditioning"  => [Box Jumps, Burpees, Wall Balls]
```

### Step 2: Add category grouping logic

Add this import at the top of `src/app.rs`:

```rust
use std::collections::HashMap;
```

`std::collections::HashMap` is part of Rust's standard library. The `use` statement brings it into scope so we can write `HashMap` instead of the full path.

Now replace the `ExercisesPage` component with this grouped version. It is longer, so read through it first, then we will break it down piece by piece:

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

That is a substantial chunk of code. Let us go through each piece.

**The constant:**

```rust
const CATEGORY_ORDER: &[(&str, &str)] = &[
    ("weightlifting", "Weightlifting"),
    ("gymnastics", "Gymnastics"),
    ...
];
```

This defines the order categories appear on screen. Each entry is a pair: the internal key (lowercase, matching what we store in the struct) and the display label (capitalized, for the user to see). We use a `const` because this order never changes while the program runs.

The type `&[(&str, &str)]` reads as: "a reference to a slice of pairs of string slices." That is a mouthful, but in practice it just means "a fixed list of (key, label) pairs."

**Building the groups:**

```rust
let mut groups: HashMap<String, Vec<Exercise>> = HashMap::new();
for ex in exercises {
    groups.entry(ex.category.clone()).or_default().push(ex);
}
```

This is the grouping logic. Let us trace through it with an example:

1. Start with an empty map: `{}`
2. First exercise is "Back Squat" with category "weightlifting"
3. `groups.entry("weightlifting")` — look up "weightlifting." It is not in the map yet.
4. `.or_default()` — since it is missing, insert an empty `Vec` and return a reference to it
5. `.push(ex)` — add Back Squat to the weightlifting list
6. Map is now: `{ "weightlifting" => [Back Squat] }`
7. Next exercise is "Deadlift," also "weightlifting"
8. `groups.entry("weightlifting")` — it exists now!
9. `.or_default()` — it already exists, so just return a reference to the existing `Vec`
10. `.push(ex)` — add Deadlift
11. Map is now: `{ "weightlifting" => [Back Squat, Deadlift] }`

This continues for all 14 exercises. By the end, the map has 5 keys (one per category), each pointing to a list of exercises.

The `mut` in `let mut groups` is necessary because we are modifying the map (adding entries). Without `mut`, Rust would not let us change it after creation.

**Rendering in order:**

```rust
CATEGORY_ORDER.iter().filter_map(|(key, label)| {
    let cat_exercises = groups.remove(*key)?;
    // ...
})
```

A `HashMap` does not guarantee any particular order when you iterate over it. Weightlifting might come before gymnastics, or after it — it depends on internal hashing. To display categories in our preferred order, we iterate over `CATEGORY_ORDER` instead and pull each category's data from the map.

`groups.remove(*key)` takes the exercises for that category out of the map. It returns an `Option` — either `Some(exercises)` if the category has exercises, or `None` if it does not.

> **Programming Concept: What is `Option`?**
>
> In many languages, when a function might not have a result, it returns `null` or `undefined`. This is a well-known source of bugs — you try to use a value, it turns out to be null, and your program crashes. This is so common it has a name: the "billion-dollar mistake."
>
> Rust eliminates this entire class of bugs. There is no `null` in Rust. Instead, when a value might or might not exist, you use `Option<T>`:
>
> - `Some(value)` — "here is the value you asked for"
> - `None` — "there is nothing here"
>
> The crucial difference from null: the compiler **forces** you to handle both cases. You cannot accidentally use a `None` as if it were a real value. If you try, the code does not compile.
>
> The `?` operator is a shortcut for handling `Option`. It means: "If this is `Some`, unwrap the value and continue. If this is `None`, stop here and return `None`." In our code, `groups.remove(*key)?` means: "Get the exercises for this category. If there are none, skip this category entirely." The `filter_map` function knows to discard any iterations that return `None`.

### Step 3: Add section header styles

Update `style/_exercises.scss` by adding these section styles:

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

Save and check your browser. You should see the exercises organized under bold category headers with counts — "WEIGHTLIFTING (4)", "GYMNASTICS (3)", and so on.

---

## Exercise 4: Color-Coded Category Borders

**Goal:** Give each category a distinct left-border color on its exercise cards.

A visual cue makes scanning faster. Glance at the blue border and you know it is weightlifting before reading the text. Purple means gymnastics, red means conditioning.

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

This is a `match` expression — Rust's version of a multi-way decision. Let us walk through how it works:

1. Rust looks at the value of `category`
2. It checks each arm from top to bottom: is it `"weightlifting"`? Is it `"gymnastics"`?
3. When it finds a match, it returns the value on the right side of the `=>`
4. The `_` at the bottom is a **wildcard** — it matches anything not already covered, like a "default" case

Two important things about `match`:

**It is exhaustive.** The compiler requires that every possible input is handled. Since `category` is a `&str` (which could be any text), the `_` arm catches everything not explicitly listed. If you remove the `_` arm, the compiler refuses to build. This is one of Rust's strongest safety guarantees — you cannot forget to handle a case.

**It is an expression.** The entire `match` evaluates to a value, which is why we can use it as the function's return value. Each arm must return the same type — here, `&'static str`.

The return type `&'static str` means "a string that lives for the entire duration of the program." The color codes like `"#3498db"` are baked into the binary at compile time — they never get freed, so returning a reference to them is always safe.

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

The `style=format!(...)` sets an inline style on the HTML element. `format!("border-left-color: {}", color)` produces something like `"border-left-color: #3498db"`, which the browser uses to color the left border of the card.

### Step 3: Also color the section headers

Add a colored dot to each category header for extra visual clarity. Update the section header rendering:

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

Time for reps. These drills reinforce what you learned in this chapter. Try each one yourself before looking at the solution. Struggling is part of learning — if it feels hard, that means your brain is building new connections.

### Drill 1: Create a Struct

Define a `Workout` struct with three fields:
- `name: String` — the workout name
- `rounds: u32` — how many rounds (an unsigned 32-bit integer, meaning it cannot be negative)
- `time_cap_minutes: Option<u32>` — an optional time cap. Some workouts have a time limit, others do not

Then write a `new()` associated function and a `description()` method that returns a formatted string. If there is a time cap, include it. If not, say "no time cap."

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
<summary>Hint: How to handle Option</summary>

`Option<u32>` can be either `Some(value)` or `None`. To create them:

```rust
let with_cap = Some(7);               // Some variant, holding the number 7
let no_cap: Option<u32> = None;       // None variant, no value
```

To handle both cases in the `description()` method, use `match`:

```rust
let cap = match self.time_cap_minutes {
    Some(mins) => format!("{} min cap", mins),
    None => "no time cap".to_string(),
};
```

This reads as: "Look at `time_cap_minutes`. If it contains a value (Some), format it as a time cap. If it is empty (None), use the text 'no time cap'."

</details>

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

Notice the `rounds` field in `Workout::new`. When the parameter name matches the field name exactly, you can write just `rounds` instead of `rounds: rounds`. This is called **field init shorthand** — a small convenience that saves a bit of typing.

</details>

### Drill 2: Use Debug to Print a Struct

The `Debug` derive lets you print a struct using `{:?}` in `println!`. This is incredibly useful for debugging — you can see exactly what is inside a struct without writing any formatting code.

Create a simple struct, derive `Debug`, and print it:

```rust
#[derive(Debug)]
struct Movement {
    name: String,
    is_compound: bool,
}

fn main() {
    let squat = Movement {
        name: String::from("Back Squat"),
        is_compound: true,
    };

    // Print the whole struct using Debug format
    println!("{:?}", squat);
    // Expected: Movement { name: "Back Squat", is_compound: true }

    // Pretty-print with {:#?} for multi-line output
    println!("{:#?}", squat);
}
```

Try it yourself. Then add a second movement and put both in a `Vec`. Print the entire vector with `{:?}`. It works because `Vec` is also Debug when its contents are Debug.

<details>
<summary>Solution</summary>

```rust
#[derive(Debug)]
struct Movement {
    name: String,
    is_compound: bool,
}

fn main() {
    let squat = Movement {
        name: String::from("Back Squat"),
        is_compound: true,
    };
    let curl = Movement {
        name: String::from("Bicep Curl"),
        is_compound: false,
    };

    println!("{:?}", squat);
    println!("{:#?}", curl);

    let movements = vec![squat, curl];
    println!("{:?}", movements);
}
```

`{:?}` prints a compact one-line format. `{:#?}` prints a pretty multi-line format that is easier to read for large structs. Both are provided by the `Debug` derive. You will use `{:?}` constantly during development to peek inside your data.

</details>

### Drill 3: Match Practice

Write a function `scoring_label` that takes a scoring type string and returns a human-readable label. Use a `match` expression with at least four arms plus a catch-all.

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
<summary>Hint: The structure of a match</summary>

A `match` looks at a value and picks the matching arm:

```rust
match some_value {
    "pattern_1" => result_1,
    "pattern_2" => result_2,
    _           => default_result,
}
```

The `_` catches anything not matched above. Every `match` must handle all possible inputs — the `_` arm ensures this. Each arm returns a value, and all arms must return the same type.

</details>

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

Try removing the `_` arm and compiling. The error message will tell you exactly what is wrong. Learning to read compiler errors is a skill that pays dividends in Rust.

</details>

---

## DSA in Context: Data Modeling

You just used three data structures in this chapter: `struct`, `Vec`, and `HashMap`. Each maps to a concept you will encounter when working with databases:

| Rust | Database equivalent | Purpose |
|------|-------------------|---------|
| `struct Exercise` | A table schema (`CREATE TABLE exercises (...)`) | Defines the shape of a single record |
| `Vec<Exercise>` | A result set (`SELECT * FROM exercises`) | An ordered collection of records |
| `HashMap<String, Vec<Exercise>>` | A `GROUP BY` query result | Records grouped by a key |

The struct is the bridge between what lives in memory and what lives in a database. When you add a field to the struct, you are implicitly saying "this should be a column in the database table." When you choose `String` vs `Option<String>`, you are deciding whether that column can be empty or not.

This is why getting the struct right early matters. In Chapter 5, when we add a real database, the `Exercise` struct will map directly to a database table. The decisions you make now carry forward.

---

## Design Insight: Obvious Code

Look at the Exercise struct one more time:

```rust
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
}
```

`exercise.scoring_type` needs no explanation. It is the type of scoring for this exercise. `exercise.category` is self-evident. Now compare this to a version with abbreviated names:

```rust
pub struct Exercise {
    pub n: String,      // name? number?
    pub cat: String,    // category? catalog?
    pub st: String,     // scoring type? start time? status?
}
```

The abbreviated version saves a few characters of typing and costs minutes of confusion every time someone reads it. This includes future you — six months from now, you will not remember what `st` stands for.

Good names eliminate the need for comments. Every time you reach for a comment to explain a variable, consider whether a better name would make the comment unnecessary.

This applies everywhere: function names (`seed_exercises()` not `get_data()`), variable names (`cat_exercises` not `items`), file names (`data.rs` not `helpers.rs`). Clear naming is the cheapest and most durable form of documentation.

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

Ready to go deeper? This chapter's data structure deep dive explores how Rust lays out your Exercise struct in memory — and how field ordering can save kilobytes from scratch in Rust — no libraries, just std.

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
