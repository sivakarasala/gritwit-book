# Chapter 4: Exercise CRUD

In Chapter 3, you made the exercise library searchable, collapsible, and expandable. But all 14 exercises are hardcoded in `seed_exercises()`. No one can add their own. No one can fix a typo. No one can remove a duplicate. This chapter adds the C, U, and D to our R — create, update, and delete operations that make the library a living thing.

Along the way, you will meet Rust's approach to error handling: `Result`, `Option`, and the `?` operator. Unlike languages that throw exceptions or return null, Rust forces you to deal with every possible failure at compile time. This sounds restrictive. In practice, it eliminates entire categories of bugs.

By the end of this chapter, you will have:

- A create exercise form with input validation
- An edit mode that reuses the same form (toggled by `Option<Exercise>`)
- Soft delete with an ownership check (you can only delete what you created)
- Toast notifications for error feedback
- A deep understanding of `Result<T, E>`, `Option<T>`, and the `?` operator

---

## Spotlight: Error Handling — `Result`, `Option`, and `?`

### The problem with exceptions

In JavaScript, errors are thrown and caught:

```javascript
try {
    const weight = parseFloat(input);
    if (isNaN(weight)) throw new Error("Invalid weight");
    await saveExercise(name, weight);
} catch (e) {
    showToast(e.message);
}
```

The issue: nothing in the function signature tells you that `parseFloat` can produce `NaN`, or that `saveExercise` can throw. Any function can throw at any time, and the compiler cannot enforce that you handle it. The only way to know is to read the implementation or hope the documentation is current.

Rust takes a different approach. There are no exceptions. Every function that can fail says so in its return type.

### `Result<T, E>` — success or failure

`Result` is an enum with two variants:

```rust
enum Result<T, E> {
    Ok(T),    // success — contains the value
    Err(E),   // failure — contains the error
}
```

A function that parses a weight from user input:

```rust
fn parse_weight(input: &str) -> Result<f64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("Weight cannot be empty".to_string());
    }
    trimmed.parse::<f64>()
        .map_err(|_| format!("'{}' is not a valid number", trimmed))
}
```

The caller cannot ignore the error — `Result` is not `f64`. To get the `f64` out, you must handle both cases:

```rust
match parse_weight("135.5") {
    Ok(weight) => println!("Weight: {} lbs", weight),
    Err(msg) => println!("Error: {}", msg),
}
```

### `Option<T>` — presence or absence

`Option` is the same idea, but for values that might not exist:

```rust
enum Option<T> {
    Some(T),  // the value exists
    None,     // no value
}
```

Where other languages use `null`, `nil`, or `undefined`, Rust uses `Option`. The compiler forces you to check before accessing the value:

```rust
let description: Option<String> = exercise.description;

// This won't compile — Option<String> is not String:
// println!("{}", description);

// You must handle both cases:
match description {
    Some(text) => println!("Description: {}", text),
    None => println!("No description provided"),
}
```

### The `?` operator — early return on error

Writing `match` for every `Result` and `Option` would be unbearable. The `?` operator handles the common case: if the value is `Ok`/`Some`, unwrap it and continue; if it is `Err`/`None`, return early from the function.

```rust
fn validate_exercise(name: &str, weight_input: &str) -> Result<(String, f64), String> {
    let name = validate_name(name)?;        // returns Err early if invalid
    let weight = parse_weight(weight_input)?; // returns Err early if invalid
    Ok((name, weight))
}

fn validate_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err("Name cannot be empty".to_string())
    } else if trimmed.len() > 100 {
        Err("Name is too long (max 100 characters)".to_string())
    } else {
        Ok(trimmed.to_string())
    }
}
```

Without `?`, you would write:

```rust
let name = match validate_name(name) {
    Ok(n) => n,
    Err(e) => return Err(e),
};
let weight = match parse_weight(weight_input) {
    Ok(w) => w,
    Err(e) => return Err(e),
};
```

The `?` operator is syntactic sugar for exactly this pattern. It is used on almost every line of real Rust code that calls fallible functions.

### Combinators: `map_err`, `unwrap_or`, `ok_or_else`

Rust's `Result` and `Option` have dozens of methods (called combinators) for transforming values without writing `match`:

```rust
// map_err: transform the error type
let weight: f64 = "135.5"
    .parse::<f64>()
    .map_err(|e| format!("Parse error: {}", e))?;

// unwrap_or: provide a default if None/Err
let description = exercise.description.unwrap_or("No description".to_string());

// ok_or_else: convert Option to Result (None becomes Err)
let pool = POOL.get()
    .ok_or_else(|| ServerFnError::new("Database pool not initialized"))?;

// and_then: chain operations that might fail
let weight = input.trim()
    .parse::<f64>()
    .ok()                                    // Result -> Option (discards error)
    .and_then(|w| if w > 0.0 { Some(w) } else { None });
```

### `ServerFnError` — errors across the network boundary

In Leptos, server functions (functions marked with `#[server]`) run on the server but are called from the client. When a server function fails, the error must be serialized, sent over HTTP, and deserialized on the client. `ServerFnError` is the error type designed for this:

```rust
#[server]
pub async fn create_exercise(name: String) -> Result<(), ServerFnError> {
    if name.trim().is_empty() {
        return Err(ServerFnError::new("Name cannot be empty"));
    }
    // ... database operations ...
    Ok(())
}
```

We will use `ServerFnError` extensively starting in Chapter 5 when we connect to the database. For now, just know that it is the standard error type for server functions, and it works seamlessly with `?` and `map_err`.

> **Coming from JS?**
>
> | Concept | JavaScript | Rust |
> |---------|-----------|------|
> | Function can fail | `throw new Error("...")` | Return `Result<T, E>` |
> | Value might be absent | `null`, `undefined` | `Option<T>` |
> | Handle errors | `try { ... } catch(e) { ... }` | `match result { Ok(v) => ..., Err(e) => ... }` |
> | Propagate errors | (implicit — exceptions bubble up) | `?` operator (explicit — each `?` is visible) |
> | Ignore errors | (easy — just don't catch) | (impossible — compiler forces handling) |
>
> The key difference: in JavaScript, error handling is opt-in (you choose to add `try/catch`). In Rust, error handling is opt-out (you must explicitly write `.unwrap()` to ignore a possible error, and that will panic at runtime if the value is `Err`). The compiler nudges you toward safety.

---

## Exercise 1: Write `parse_weight` and Wire It Into the Form

**Goal:** Build a weight parser that demonstrates `Result<T, E>`, validation, and error propagation with `?`.

### Step 1: Add the validation module

Create `src/validation.rs`:

```rust
/// Parse a weight string like "135", "135.5", or "225 lbs" into a float.
pub fn parse_weight(input: &str) -> Result<f64, String> {
    let cleaned = input
        .trim()
        .trim_end_matches("lbs")
        .trim_end_matches("kg")
        .trim();

    if cleaned.is_empty() {
        return Err("Weight cannot be empty".to_string());
    }

    let weight: f64 = cleaned
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", cleaned))?;

    if weight <= 0.0 {
        return Err("Weight must be positive".to_string());
    }

    if weight > 2000.0 {
        return Err("Weight exceeds maximum (2000)".to_string());
    }

    Ok(weight)
}

/// Validate an exercise name. Returns the trimmed name or an error.
pub fn validate_exercise_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        Err("Exercise name cannot be empty".to_string())
    } else if trimmed.len() > 100 {
        Err("Exercise name is too long (max 100 characters)".to_string())
    } else {
        Ok(trimmed)
    }
}
```

Register it in `src/lib.rs`:

```rust
pub mod app;
pub mod data;
pub mod validation;
```

### Step 2: Add tests

Rust has built-in testing. Add this to the bottom of `src/validation.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_weights() {
        assert_eq!(parse_weight("135"), Ok(135.0));
        assert_eq!(parse_weight("  225.5  "), Ok(225.5));
        assert_eq!(parse_weight("315 lbs"), Ok(315.0));
        assert_eq!(parse_weight("100kg"), Ok(100.0));
    }

    #[test]
    fn parse_invalid_weights() {
        assert!(parse_weight("").is_err());
        assert!(parse_weight("abc").is_err());
        assert!(parse_weight("-50").is_err());
        assert!(parse_weight("9999").is_err());
    }

    #[test]
    fn validate_names() {
        assert_eq!(
            validate_exercise_name("  Back Squat  "),
            Ok("Back Squat".to_string())
        );
        assert!(validate_exercise_name("").is_err());
        assert!(validate_exercise_name(&"a".repeat(101)).is_err());
    }
}
```

Run the tests:

```bash
cargo test
```

The `#[cfg(test)]` attribute means this module is only compiled during `cargo test`, not in production builds. Each `#[test]` function is a test case. `assert_eq!` checks for equality, `assert!` checks for truthiness. If any assertion fails, the test fails with a clear error message showing the expected vs actual value.

<details>
<summary>Hint: If cargo test fails with "cannot find crate"</summary>

`cargo test` compiles the library crate without the `ssr` or `hydrate` features. If your `lib.rs` has `#[cfg(feature = "ssr")]` on module declarations that the test depends on, the test module will not be able to see those modules. The validation module we created has no feature gates, so it should compile cleanly. If you see errors related to other modules, they are likely gated behind a feature — that is expected and not a problem for this exercise.

</details>

---

## Exercise 2: Build the Create/Edit Form with `Option<Exercise>` Toggle

**Goal:** Build a form component that handles both creating a new exercise and editing an existing one, using `Option<Exercise>` to toggle between modes.

### Step 1: Understand the pattern

The key insight is that a create form and an edit form have the same fields. The difference is:

- **Create mode:** fields start empty, submit creates a new record
- **Edit mode:** fields start with existing values, submit updates the record

We model this with `Option<Exercise>`:

```rust
// None = create mode (new exercise)
// Some(exercise) = edit mode (editing existing exercise)
let editing: Option<Exercise> = None;
```

### Step 2: Build the form component

Add this to `src/app.rs` (below your existing components):

```rust
use crate::validation::validate_exercise_name;

#[component]
fn ExerciseFormPanel(
    /// None = create mode, Some(ex) = edit mode
    exercise: Option<Exercise>,
    on_save: Callback<Exercise>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    // Initialize fields — empty for create, pre-filled for edit
    let init_name = exercise.as_ref().map(|e| e.name.clone()).unwrap_or_default();
    let init_category = exercise
        .as_ref()
        .map(|e| e.category.clone())
        .unwrap_or_else(|| "conditioning".to_string());
    let init_scoring = exercise
        .as_ref()
        .map(|e| e.scoring_type.clone())
        .unwrap_or_else(|| "weight_and_reps".to_string());

    let name_input = RwSignal::new(init_name);
    let category_input = RwSignal::new(init_category);
    let scoring_input = RwSignal::new(init_scoring);
    let error_msg = RwSignal::new(String::new());

    let is_edit = exercise.is_some();
    let button_label = if is_edit { "Save Changes" } else { "Add Exercise" };

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error_msg.set(String::new());

        // Validate the name — Result guides us
        let name = match validate_exercise_name(&name_input.get_untracked()) {
            Ok(valid_name) => valid_name,
            Err(msg) => {
                error_msg.set(msg);
                return;
            }
        };

        let new_exercise = Exercise {
            name,
            category: category_input.get_untracked(),
            scoring_type: scoring_input.get_untracked(),
        };

        on_save.run(new_exercise);
    };

    view! {
        <form class="exercise-form" on:submit=on_submit>
            // Show error banner if validation failed
            {move || {
                let msg = error_msg.get();
                (!msg.is_empty()).then(|| view! {
                    <div class="form-error">{msg}</div>
                })
            }}

            <input
                type="text"
                class="form-input"
                placeholder="Exercise name"
                prop:value=move || name_input.get()
                on:input=move |ev| {
                    name_input.set(event_target_value(&ev));
                    error_msg.set(String::new()); // clear error on input
                }
            />

            <select
                class="form-input"
                prop:value=move || category_input.get()
                on:change=move |ev| category_input.set(event_target_value(&ev))
            >
                <option value="weightlifting">"Weightlifting"</option>
                <option value="gymnastics">"Gymnastics"</option>
                <option value="conditioning">"Conditioning"</option>
                <option value="cardio">"Cardio"</option>
                <option value="mobility">"Mobility"</option>
            </select>

            <select
                class="form-input"
                prop:value=move || scoring_input.get()
                on:change=move |ev| scoring_input.set(event_target_value(&ev))
            >
                <option value="weight_and_reps">"Weight & Reps"</option>
                <option value="reps_only">"Reps Only"</option>
                <option value="time">"Time"</option>
            </select>

            <div class="form-actions">
                <button type="submit" class="form-submit">{button_label}</button>
                <button
                    type="button"
                    class="form-cancel"
                    on:click=move |_| on_cancel.run(())
                >
                    "Cancel"
                </button>
            </div>
        </form>
    }
}
```

### Step 3: Understand `Callback`

`Callback<T>` is Leptos's type-erased function wrapper. It is like a `Box<dyn Fn(T)>` but optimized for the framework. When a child component needs to notify its parent (e.g., "the form was submitted" or "the user clicked cancel"), it uses a `Callback`:

```rust
// Parent creates the callback:
let on_save = Callback::new(move |exercise: Exercise| {
    exercises.update(|list| list.push(exercise));
    show_form.set(false);
});

// Child invokes it:
on_save.run(new_exercise);
```

This is the same pattern as React's "lifting state up" — the parent owns the state and passes down handlers. The difference is that Rust's type system ensures the callback signature matches at compile time.

### Step 4: Wire it into the exercises page

Update your `ExercisesPage` to manage form visibility and the exercise list:

```rust
#[component]
fn ExercisesPage() -> impl IntoView {
    let exercises = RwSignal::new(seed_exercises());
    let show_form = RwSignal::new(false);
    let editing_exercise: RwSignal<Option<Exercise>> = RwSignal::new(None);

    // ... search and filter code from Chapter 3 ...

    let on_save = Callback::new(move |exercise: Exercise| {
        if editing_exercise.get_untracked().is_some() {
            // Edit mode — replace the existing exercise by name
            exercises.update(|list| {
                if let Some(existing) = list.iter_mut().find(|e| e.name == exercise.name) {
                    existing.category = exercise.category;
                    existing.scoring_type = exercise.scoring_type;
                }
            });
        } else {
            // Create mode — add to the list
            exercises.update(|list| list.push(exercise));
        }
        editing_exercise.set(None);
        show_form.set(false);
    });

    let on_cancel = Callback::new(move |_| {
        editing_exercise.set(None);
        show_form.set(false);
    });

    view! {
        <div class="exercises-page">
            <button
                class=move || if show_form.get() { "fab fab--active" } else { "fab" }
                on:click=move |_| {
                    editing_exercise.set(None); // ensure create mode
                    show_form.update(|v| *v = !*v);
                }
            >
                <span class="fab-icon"></span>
            </button>

            {move || {
                show_form.get().then(|| view! {
                    <ExerciseFormPanel
                        exercise=editing_exercise.get_untracked()
                        on_save=on_save
                        on_cancel=on_cancel
                    />
                })
            }}

            // ... rest of exercise list rendering ...
        </div>
    }
}
```

### Step 5: Add form styles

Add to `style/_exercises.scss`:

```scss
.exercise-form {
  background: var(--bg-card);
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.form-input {
  background: var(--bg-input);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.6rem 0.75rem;
  color: var(--text-primary);
  font-size: 16px; // prevents iOS auto-zoom

  &:focus {
    outline: none;
    border-color: var(--accent);
  }
}

.form-error {
  background: rgba(231, 76, 60, 0.15);
  color: #e74c3c;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  font-size: 0.85rem;
}

.form-actions {
  display: flex;
  gap: 0.5rem;
}

.form-submit {
  flex: 1;
  padding: 0.6rem;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;

  &:hover {
    background: var(--accent-hover);
  }
}

.form-cancel {
  padding: 0.6rem 1rem;
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--border);
  border-radius: 6px;
  cursor: pointer;
}

.fab {
  position: fixed;
  bottom: calc(var(--nav-h) + 1rem);
  right: 1rem;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--accent);
  border: none;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: transform 0.2s;

  &--active {
    transform: rotate(45deg);
  }
}

.fab-icon {
  display: block;
  width: 24px;
  height: 24px;
  background-color: white;
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.5'%3E%3Cline x1='12' y1='5' x2='12' y2='19'/%3E%3Cline x1='5' y1='12' x2='19' y2='12'/%3E%3C/svg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='black' stroke-width='2.5'%3E%3Cline x1='12' y1='5' x2='12' y2='19'/%3E%3Cline x1='5' y1='12' x2='19' y2='12'/%3E%3C/svg%3E");
  -webkit-mask-size: contain;
  mask-size: contain;
}
```

Update `style/main.scss` to include the new styles (it should already have `@use "exercises";` from Chapter 2).

<details>
<summary>Hint: If the form does not appear when you tap the FAB</summary>

The FAB (floating action button) toggles `show_form` between `true` and `false`. The form renders conditionally with `.then()`. If the form appears but immediately disappears, check that `on_cancel` is not being called on render. Make sure `on:click` is on the cancel *button*, not on the form itself.

</details>

---

## Exercise 3: Implement Soft Delete with Confirmation Modal

**Goal:** Add a delete button to exercise cards that soft-deletes (marks as deleted) rather than hard-deleting (removing permanently). Include an ownership check and a confirmation dialog.

### Step 1: Understand soft delete vs hard delete

**Hard delete:** `DELETE FROM exercises WHERE id = $1;` — the row is gone forever.

**Soft delete:** `UPDATE exercises SET deleted_at = NOW() WHERE id = $1;` — the row still exists but is marked as deleted. Your queries filter it out: `SELECT * FROM exercises WHERE deleted_at IS NULL`.

For now, we implement soft delete in memory. The pattern will transfer directly to PostgreSQL in Chapter 5.

```rust
// In your Exercise struct, add:
#[derive(Clone, Debug)]
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
    pub created_by: Option<String>,   // who created this exercise
    pub deleted_at: Option<String>,   // None = active, Some = soft-deleted
}
```

### Step 2: Add the ownership check

The delete function checks whether the requesting user owns the exercise:

```rust
/// Soft-delete an exercise. Returns Err if the user doesn't own it.
pub fn soft_delete_exercise(
    exercises: &mut Vec<Exercise>,
    exercise_name: &str,
    requesting_user: &str,
) -> Result<(), String> {
    let exercise = exercises
        .iter_mut()
        .find(|e| e.name == exercise_name && e.deleted_at.is_none())
        .ok_or_else(|| "Exercise not found".to_string())?;

    // Ownership check
    match &exercise.created_by {
        Some(owner) if owner == requesting_user => {
            exercise.deleted_at = Some("2024-01-01T00:00:00Z".to_string());
            Ok(())
        }
        Some(_) => Err("You can only delete exercises you created".to_string()),
        None => Err("This exercise has no owner and cannot be deleted".to_string()),
    }
}
```

Notice the chain of error handling:

1. `.find()` returns `Option<&mut Exercise>` — the exercise might not exist
2. `.ok_or_else()` converts `None` into a descriptive `Err`
3. `match &exercise.created_by` handles three cases: owner matches, owner doesn't match, no owner

Each step uses the type system to force handling of the failure case.

### Step 3: Build the confirmation modal

Create a `ConfirmModal` component. This follows the same pattern as the reference GrindIt app's `DeleteModal`:

```rust
#[component]
fn ConfirmModal(
    show: RwSignal<bool>,
    title: &'static str,
    subtitle: &'static str,
    on_confirm: Callback<()>,
) -> impl IntoView {
    view! {
        <div
            class="confirm-overlay"
            style=move || if show.get() { "display:flex" } else { "display:none" }
            on:click=move |_| show.set(false)
        >
            <div class="confirm-dialog" on:click=move |ev| ev.stop_propagation()>
                <p class="confirm-msg">{title}</p>
                <p class="confirm-sub">{subtitle}</p>
                <div class="confirm-actions">
                    <button
                        class="confirm-cancel-btn"
                        on:click=move |_| show.set(false)
                    >"Cancel"</button>
                    <button
                        class="confirm-delete-btn"
                        on:click=move |_| {
                            on_confirm.run(());
                            show.set(false);
                        }
                    >"Delete"</button>
                </div>
            </div>
        </div>
    }
}
```

### Step 4: Wire up delete in the exercise card

Add delete state to `ExercisesPage`:

```rust
let show_delete = RwSignal::new(false);
let pending_delete_name = RwSignal::new(String::new());
let current_user = "user_1".to_string(); // hardcoded for now — Chapter 7 adds auth

let on_delete = Callback::new(move |_: ()| {
    let name = pending_delete_name.get_untracked();
    exercises.update(|list| {
        match soft_delete_exercise(list, &name, &current_user) {
            Ok(()) => {} // success — exercise is now soft-deleted
            Err(msg) => {
                // In Exercise 4, we'll show this as a toast
                leptos::logging::log!("Delete failed: {}", msg);
            }
        }
    });
});
```

In each exercise card's expanded panel, add the delete button:

```rust
// Inside the expanded card view
{exercise.created_by.as_ref().map(|owner| {
    let can_delete = owner == &current_user;
    let ex_name = exercise.name.clone();
    can_delete.then(move || view! {
        <button
            class="exercise-delete"
            on:click=move |_| {
                pending_delete_name.set(ex_name.clone());
                show_delete.set(true);
            }
        >"Delete"</button>
    })
})}
```

When filtering exercises for display, exclude soft-deleted ones:

```rust
let active_exercises: Vec<&Exercise> = exercises
    .iter()
    .filter(|e| e.deleted_at.is_none()) // only show active exercises
    .filter(|e| q.is_empty() || e.name.to_lowercase().contains(&q))
    .collect();
```

### Step 5: Add modal styles

```scss
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.confirm-dialog {
  background: var(--bg-card);
  border-radius: 12px;
  padding: 1.5rem;
  max-width: 300px;
  width: 90%;
  text-align: center;
}

.confirm-msg {
  color: var(--text-primary);
  font-weight: 600;
  font-size: 1rem;
  margin: 0 0 0.25rem;
}

.confirm-sub {
  color: var(--text-muted);
  font-size: 0.85rem;
  margin: 0 0 1.25rem;
}

.confirm-actions {
  display: flex;
  gap: 0.5rem;
}

.confirm-cancel-btn {
  flex: 1;
  padding: 0.6rem;
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--border);
  border-radius: 6px;
  cursor: pointer;
}

.confirm-delete-btn {
  flex: 1;
  padding: 0.6rem;
  background: #e74c3c;
  color: white;
  border: none;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
}
```

<details>
<summary>Hint: If the modal overlay does not cover the bottom nav</summary>

The overlay uses `position: fixed; inset: 0;` which should cover the entire viewport. If the bottom nav peeks through, check that the overlay's `z-index: 200` is higher than the nav's `z-index: 100`. Also ensure the overlay is rendered outside the `<main>` element — inside `<main>` with `overflow: hidden` might clip it.

</details>

---

## Exercise 4: Add Toast Notification Component

**Goal:** Build a toast notification system that shows error messages from failed operations (delete failures, validation errors) and auto-dismisses after a few seconds.

### Step 1: Create the toast signal

Toast notifications are a perfect use case for signals. A `Vec<Toast>` signal holds the active toasts, and each toast has an auto-dismiss timer:

```rust
#[derive(Clone, Debug)]
struct Toast {
    id: u32,
    message: String,
    is_error: bool,
}

// At the top of ExercisesPage:
let toasts: RwSignal<Vec<Toast>> = RwSignal::new(Vec::new());
let next_toast_id = RwSignal::new(0_u32);

let show_toast = Callback::new(move |message: String| {
    let id = next_toast_id.get_untracked();
    next_toast_id.set(id + 1);

    toasts.update(|list| {
        list.push(Toast {
            id,
            message,
            is_error: true,
        });
    });

    // Auto-dismiss after 3 seconds
    #[cfg(feature = "hydrate")]
    {
        let id_to_remove = id;
        set_timeout(
            move || {
                toasts.update(|list| list.retain(|t| t.id != id_to_remove));
            },
            std::time::Duration::from_secs(3),
        );
    }
});
```

### Step 2: Render toasts

Add the toast container to your `ExercisesPage` view:

```rust
// Inside the ExercisesPage view, at the top:
<div class="toast-container">
    {move || {
        toasts.get().into_iter().map(|toast| {
            let tid = toast.id;
            view! {
                <div class="toast toast--error">
                    <span class="toast-msg">{toast.message}</span>
                    <button
                        class="toast-close"
                        on:click=move |_| {
                            toasts.update(|list| list.retain(|t| t.id != tid));
                        }
                    >"x"</button>
                </div>
            }
        }).collect_view()
    }}
</div>
```

### Step 3: Connect toasts to the delete flow

Update the delete callback to show a toast on failure:

```rust
let on_delete = Callback::new(move |_: ()| {
    let name = pending_delete_name.get_untracked();
    exercises.update(|list| {
        match soft_delete_exercise(list, &name, &current_user) {
            Ok(()) => {
                // Optionally show success toast
            }
            Err(msg) => {
                show_toast.run(msg);
            }
        }
    });
});
```

### Step 4: Toast styles

```scss
.toast-container {
  position: fixed;
  top: calc(var(--header-h) + 0.5rem);
  left: 50%;
  transform: translateX(-50%);
  z-index: 300;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 90%;
  max-width: 400px;
  pointer-events: none;
}

.toast {
  background: var(--bg-card);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  pointer-events: auto;
  animation: toast-in 0.3s ease-out;
  border-left: 3px solid var(--text-muted);

  &--error {
    border-left-color: #e74c3c;
  }

  &--success {
    border-left-color: #2ecc71;
  }
}

.toast-msg {
  color: var(--text-primary);
  font-size: 0.85rem;
}

.toast-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 0 0.25rem;
  font-size: 0.85rem;
}

@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

<details>
<summary>Hint: If set_timeout is not found</summary>

`set_timeout` is provided by Leptos. Make sure you have `use leptos::prelude::*;` at the top of your file. The function takes a closure and a `Duration`, and runs the closure after the specified delay. It only works in the browser (hence the `#[cfg(feature = "hydrate")]` gate). On the server, the timeout would never fire because there is no event loop running for individual page renders.

</details>

---

## Rust Gym

### Drill 1: The `?` Chain

This function has nested `match` statements. Rewrite it using `?` and combinators to reduce it to 3-4 lines.

```rust
fn get_exercise_weight(exercises: &[Exercise], name: &str) -> Result<f64, String> {
    let exercise = match exercises.iter().find(|e| e.name == name) {
        Some(e) => e,
        None => return Err(format!("Exercise '{}' not found", name)),
    };
    let scoring = match exercise.scoring_type.as_str() {
        "weight_and_reps" => exercise.scoring_type.clone(),
        _ => return Err("Not a weighted exercise".to_string()),
    };
    Ok(0.0) // placeholder — real app would look up the score
}
```

<details>
<summary>Solution</summary>

```rust
fn get_exercise_weight(exercises: &[Exercise], name: &str) -> Result<f64, String> {
    let exercise = exercises
        .iter()
        .find(|e| e.name == name)
        .ok_or_else(|| format!("Exercise '{}' not found", name))?;

    (exercise.scoring_type == "weight_and_reps")
        .then_some(0.0)
        .ok_or_else(|| "Not a weighted exercise".to_string())
}
```

The key transformations:
- `.find().ok_or_else()?` replaces the first `match` — converts `Option` to `Result` and propagates the error
- `.then_some().ok_or_else()` replaces the second `match` — `bool::then_some` returns `Some(value)` if true, `None` if false

</details>

### Drill 2: `map_err` Chains

Convert this code that uses `.unwrap()` (which panics on error) into proper error handling using `map_err`:

```rust
fn parse_exercise_input(json: &str) -> Exercise {
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let name = value["name"].as_str().unwrap().to_string();
    let category = value["category"].as_str().unwrap().to_string();
    Exercise::new(&name, &category, "weight_and_reps")
}
```

<details>
<summary>Solution</summary>

```rust
fn parse_exercise_input(json: &str) -> Result<Exercise, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    let name = value["name"]
        .as_str()
        .ok_or("Missing 'name' field")?
        .to_string();

    let category = value["category"]
        .as_str()
        .ok_or("Missing 'category' field")?
        .to_string();

    Ok(Exercise::new(&name, &category, "weight_and_reps"))
}
```

Each `.unwrap()` was a potential panic. Now each failure point returns a descriptive error:
- `serde_json::from_str` returns `Result` — use `map_err` to convert the JSON error to a `String`
- `.as_str()` returns `Option` — use `ok_or` to convert `None` to an error message
- The function now returns `Result<Exercise, String>` so the caller can handle failures gracefully

</details>

### Drill 3: Option Combinators

Given this struct, use `Option` combinators to extract a formatted display string:

```rust
struct Profile {
    display_name: Option<String>,
    email: Option<String>,
}
```

Write a function `greeting(profile: &Profile) -> String` that returns:
- `"Hello, {display_name}!"` if display_name is Some
- `"Hello, {email}!"` if display_name is None but email is Some
- `"Hello, athlete!"` if both are None

<details>
<summary>Solution</summary>

```rust
fn greeting(profile: &Profile) -> String {
    let name = profile
        .display_name
        .as_ref()
        .or(profile.email.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("athlete");

    format!("Hello, {}!", name)
}
```

The chain:
- `.as_ref()` converts `&Option<String>` to `Option<&String>` (avoids moving the String out)
- `.or()` falls back to the second Option if the first is None
- `.map(|s| s.as_str())` converts `Option<&String>` to `Option<&str>`
- `.unwrap_or("athlete")` provides the final default

Alternative using `match` (equally valid, more explicit):

```rust
fn greeting(profile: &Profile) -> String {
    match (&profile.display_name, &profile.email) {
        (Some(name), _) => format!("Hello, {}!", name),
        (None, Some(email)) => format!("Hello, {}!", email),
        (None, None) => "Hello, athlete!".to_string(),
    }
}
```

</details>

---

## DSA in Context: Null Safety

Tony Hoare, who invented null references in 1965, later called it his "billion-dollar mistake." Null pointer exceptions are the most common runtime crash in Java, C#, JavaScript, Python, and Go. They happen because the type system does not distinguish "this value exists" from "this value might not exist."

Rust eliminates null entirely. There is no `null`, no `nil`, no `None` floating around untyped. If a value might be absent, the type says so: `Option<String>`. If it is always present, the type says so: `String`. The compiler enforces the distinction.

```
// In Java (or JS, Python, C#, Go):
String name = exercise.getDescription();  // might be null — no way to know from the type
name.length();                             // NullPointerException at runtime

// In Rust:
let name: Option<String> = exercise.description;  // the type tells you it might be None
name.len();   // COMPILE ERROR — Option<String> has no .len() method
name.unwrap().len();  // compiles, but panics at runtime if None
name.map(|s| s.len()).unwrap_or(0);  // safe — handles both cases
```

This matters for your GrindIt exercises: `description`, `demo_video_url`, and `created_by` are all `Option<String>`. The compiler forces you to handle the "not provided" case everywhere these fields are used. You cannot accidentally call `.to_uppercase()` on a None description — the code would not compile.

---

## System Design Corner: Soft Delete vs Hard Delete

In a system design interview, "how do you handle deletes?" is a common question. Here are the tradeoffs:

| Aspect | Hard Delete | Soft Delete |
|--------|------------|-------------|
| Implementation | `DELETE FROM table WHERE id = $1` | `UPDATE table SET deleted_at = NOW() WHERE id = $1` |
| Recovery | Impossible (without backups) | Trivial: `SET deleted_at = NULL` |
| Storage | Frees disk space | Rows accumulate forever |
| Query complexity | Simple: `SELECT * FROM table` | Every query needs `WHERE deleted_at IS NULL` |
| Referential integrity | Must cascade deletes or fail | Soft-deleted rows still referenced by foreign keys |
| Compliance | GDPR right-to-erasure requires eventual hard delete | Soft delete alone may not satisfy erasure requirements |
| Audit trail | None (row is gone) | Complete (row persists with timestamp) |

GrindIt uses soft delete for exercises because:
1. **Workout history** references exercises by ID. Hard-deleting an exercise would orphan those references.
2. **Undo mistakes.** A coach accidentally deleting "Back Squat" can be restored.
3. **Audit trail.** We can see who deleted what and when.

The ownership check (`created_by = $2`) is an authorization pattern — it ensures users can only delete their own content. Admins bypass this check, which is why the reference code has `if is_admin { ... } else { ... AND created_by = $2 }`.

> **Interview talking point:** *"We use soft delete with a deleted_at timestamp column. All read queries filter on deleted_at IS NULL, which we enforce through a database view or a query helper function. This preserves referential integrity with workout logs and supports undo. For GDPR compliance, we run a periodic job that hard-deletes records older than 30 days past their deleted_at timestamp."*

---

## Design Insight: Define Errors Out of Existence

In *A Philosophy of Software Design*, Ousterhout's boldest claim is that the best way to handle errors is to **define them out of existence**. Rather than detecting and reporting invalid states, design your API so those states cannot occur.

Consider the exercise name validation. We could validate at display time:

```rust
// Bad: error can occur anywhere the name is used
fn display_exercise(name: &str) -> Result<String, String> {
    if name.is_empty() { return Err("Empty name".to_string()); }
    Ok(format!("Exercise: {}", name))
}
```

Or we could validate at creation time and use a type that guarantees validity:

```rust
// Good: impossible to have an invalid ExerciseName
struct ExerciseName(String);

impl ExerciseName {
    fn new(name: &str) -> Result<Self, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            Err("Name cannot be empty".to_string())
        } else {
            Ok(ExerciseName(trimmed.to_string()))
        }
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

// Now display_exercise cannot fail:
fn display_exercise(name: &ExerciseName) -> String {
    format!("Exercise: {}", name.as_str())
}
```

By validating at the boundary (form submission, API endpoint) and using the type system to carry the guarantee forward, we eliminate error-handling code from every downstream function. The `Result` only appears once — at the creation boundary.

This is the "parse, don't validate" pattern. The `validate_exercise_name` function we wrote in Exercise 1 is the boundary. Everything downstream receives a known-good `String`. The compiler will not let you skip the validation step because the types are different.

---

## What You Built

In this chapter, you:

1. **Built a validation module** — `parse_weight` and `validate_exercise_name` with `Result<T, E>` return types and tests
2. **Created a dual-mode form** — `Option<Exercise>` toggles between create and edit mode, with `Callback` for parent communication
3. **Implemented soft delete** — ownership check, `deleted_at` timestamp, confirmation modal with overlay
4. **Added toast notifications** — auto-dismissing error messages driven by a `Vec<Toast>` signal
5. **Practiced `Result`, `Option`, and `?`** — the foundation of all error handling in Rust

All of this is still in-memory — reloading the page loses your changes. In Chapter 5, we will connect to PostgreSQL with SQLx, making exercises persist across sessions. The `Result` and `?` patterns you learned here will appear on every database call.

---

### 🧬 DS Deep Dive

`Result<T, String>` works but falls apart at scale. This deep dive builds a real error system — custom types, automatic From conversion, the ? operator's internals, error chains, and the thiserror/anyhow ecosystem.

**→ [Error Handling Ecosystem — "The Incident Report Form"](../ds-narratives/ch04-error-handling-ecosystem.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/validation.rs` | Validation logic embedded in [`src/pages/exercises/exercise_form.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/exercise_form.rs) |
| `ExerciseFormPanel` component | [`src/pages/exercises/exercise_form.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/exercise_form.rs) |
| `ConfirmModal` component | [`src/components/delete_modal.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/components/delete_modal.rs) |
| Soft delete logic | [`src/db.rs` — `delete_exercise_db()`](https://github.com/sivakarasala/gritwit/blob/main/src/db.rs) |
| Toast notifications | Custom for the book — the reference app shows errors inline |
