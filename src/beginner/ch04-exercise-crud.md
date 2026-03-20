# Chapter 4: Exercise CRUD

In Chapter 3, you made the exercise library searchable, collapsible, and interactive. But every exercise is still hardcoded inside `seed_exercises()`. If someone wants to add a new movement, fix a typo, or remove a duplicate, they have to edit Rust source code and recompile. That is not an app. That is a text file with extra steps.

This chapter changes that. You will add the ability to **create**, **update**, and **delete** exercises right from the browser. Together with the **read** capability you built in Chapter 2, this completes the full CRUD cycle: Create, Read, Update, Delete.

Along the way, you will learn how Rust handles things that can go wrong. Every programming language has to deal with errors --- a user types garbage into a form, a file does not exist, a network request times out. Rust's approach is different from most languages, and arguably better. By the end of this chapter, you will understand why.

By the end of this chapter, you will have:

- A create exercise form with input validation
- An edit mode that reuses the same form (toggled by `Option<Exercise>`)
- Soft delete with an ownership check (you can only delete what you created)
- Toast notifications for error feedback
- A solid understanding of `Result<T, E>`, `Option<T>`, and the `?` operator

---

## Spotlight: Error Handling --- `Result`, `Option`, and `?`

### Programs can fail. What do we do about it?

Think about ordering food at a restaurant. Most of the time, everything goes smoothly: you order, you eat, you pay, you leave. But sometimes things go wrong. The kitchen runs out of an ingredient. Your credit card is declined. The waiter brings the wrong dish.

A restaurant handles these failures *gracefully*. The waiter tells you about the missing ingredient and suggests an alternative. The card terminal shows a clear error message. Nobody panics. Nobody pretends nothing happened.

Programs work the same way. A user might type "abc" into a weight field. They might try to delete an exercise they did not create. The database might be down. **Error handling** is how we deal with these situations without crashing.

> **Programming Concept: What is Error Handling?**
>
> Error handling means writing code that anticipates things going wrong and responds in a controlled way. Instead of your program crashing with a mysterious message, it shows the user a helpful explanation and keeps running.
>
> Think of it like a safety net at a circus. The acrobats (your code) do impressive things up high. Sometimes they slip. The net (error handling) catches them so the show can continue.

### The problem with exceptions

Most programming languages use a system called **exceptions**. When something goes wrong, you "throw" an error, and somewhere else in your code, you "catch" it:

```javascript
// JavaScript's approach
try {
    const weight = parseFloat(input);
    if (isNaN(weight)) throw new Error("Invalid weight");
    await saveExercise(name, weight);
} catch (e) {
    showToast(e.message);
}
```

This looks reasonable. But there is a hidden problem: *nothing in the function signature tells you it can fail*. Any function can throw at any time, and the compiler cannot warn you if you forget to handle the error. The only way to know is to read the source code or hope the documentation is up to date.

Rust takes a completely different approach. There are no exceptions. Instead, every function that can fail **says so in its return type**.

### `Result<T, E>` --- success or failure

> **Programming Concept: What is Result?**
>
> Imagine you order a package online. When the delivery person arrives, there are two possibilities:
> - The package arrived safely (success)
> - The package got lost or damaged (failure)
>
> `Result` is like that delivery. It is a container that holds *either* a success value *or* an error value. You cannot open the package without checking which one it is.
>
> In Rust, `Result` has two variants:
> - `Ok(value)` --- the operation succeeded, here is the value
> - `Err(error)` --- the operation failed, here is what went wrong

Here is `Result` defined in Rust:

```rust
enum Result<T, E> {
    Ok(T),    // success --- contains the value of type T
    Err(E),   // failure --- contains the error of type E
}
```

The `T` and `E` are **type parameters** --- placeholders for the actual types you choose. For example, a function that parses a weight from user input:

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

Here, `T` is `f64` (a floating-point number --- the weight) and `E` is `String` (a human-readable error message). The function returns `Ok(135.0)` for valid input and `Err("'abc' is not a valid number")` for invalid input.

The caller **must** handle both cases. They cannot just pretend the value is an `f64`:

```rust
// This will NOT compile --- Result<f64, String> is not f64:
// let weight: f64 = parse_weight("abc");

// You must handle both possibilities:
match parse_weight("135.5") {
    Ok(weight) => println!("Weight: {} lbs", weight),
    Err(msg) => println!("Error: {}", msg),
}
```

This is Rust's superpower: the compiler *forces* you to think about failure. You cannot accidentally ignore an error. Compare this to JavaScript, where `parseFloat("abc")` silently returns `NaN` --- a bug waiting to happen at 2 AM in production.

### `Option<T>` --- presence or absence

`Option` is `Result`'s sibling. While `Result` represents success or failure, `Option` represents "something is there" or "nothing is there":

```rust
enum Option<T> {
    Some(T),  // the value exists
    None,     // no value
}
```

Where other languages use `null`, `nil`, or `undefined`, Rust uses `Option`. And just like `Result`, you must handle both cases:

```rust
let description: Option<String> = exercise.description;

// This will NOT compile --- Option<String> is not String:
// println!("{}", description);

// You must check:
match description {
    Some(text) => println!("Description: {}", text),
    None => println!("No description provided"),
}
```

In GrindIt, we will use `Option` for things like:
- `created_by: Option<String>` --- an exercise might have been created by the system (no owner) or by a user
- `deleted_at: Option<String>` --- `None` means active, `Some(timestamp)` means soft-deleted
- The form component uses `Option<Exercise>` --- `None` means "create mode," `Some(exercise)` means "edit mode"

### The `?` operator --- a shortcut for "stop if this failed"

> **Programming Concept: What is the `?` Operator?**
>
> Imagine you are following a recipe with multiple steps. At each step, something could go wrong:
> 1. Crack an egg (might get shell in the bowl)
> 2. Whisk it (might not have a whisk)
> 3. Pour into pan (pan might not be hot enough)
>
> At every step, if something goes wrong, you stop cooking and deal with the problem. You do not keep going and hope for the best.
>
> The `?` operator does exactly this. After any operation that can fail, adding `?` says: "If this succeeded, give me the value and keep going. If it failed, stop here and return the error to whoever called me."

Without `?`, you would write `match` for every fallible operation:

```rust
fn validate_exercise(name: &str, weight_input: &str) -> Result<(String, f64), String> {
    let name = match validate_name(name) {
        Ok(n) => n,
        Err(e) => return Err(e),
    };
    let weight = match parse_weight(weight_input) {
        Ok(w) => w,
        Err(e) => return Err(e),
    };
    Ok((name, weight))
}
```

With `?`, the same code becomes:

```rust
fn validate_exercise(name: &str, weight_input: &str) -> Result<(String, f64), String> {
    let name = validate_name(name)?;        // returns Err early if invalid
    let weight = parse_weight(weight_input)?; // returns Err early if invalid
    Ok((name, weight))
}
```

Each `?` is syntactic sugar for the `match` pattern. It says: "If this is `Ok`, unwrap the value. If this is `Err`, return it from the function immediately."

There is one important rule: `?` can only be used inside a function that returns `Result` (or `Option`). The error type of the `?` expression must match (or be convertible to) the error type in the function's return type.

### Useful methods on `Result` and `Option`

Rust provides many helper methods so you do not need to write `match` everywhere:

```rust
// map_err: change the error type
let weight: f64 = "135.5"
    .parse::<f64>()
    .map_err(|e| format!("Parse error: {}", e))?;

// unwrap_or: provide a default if None/Err
let description = exercise.description.unwrap_or("No description".to_string());

// ok_or_else: convert Option to Result (None becomes Err)
let pool = POOL.get()
    .ok_or_else(|| ServerFnError::new("Database pool not initialized"))?;

// is_some() / is_none(): check without consuming
if exercise.deleted_at.is_some() {
    println!("This exercise has been deleted");
}
```

Do not worry about memorizing all of these. You will learn them naturally as we use them. The important thing to understand right now is the *pattern*: Rust makes you handle every possible failure, and `Result`, `Option`, and `?` are the tools for doing it cleanly.

> **Coming from JS?**
>
> | Concept | JavaScript | Rust |
> |---------|-----------|------|
> | Function can fail | `throw new Error("...")` | Return `Result<T, E>` |
> | Value might be absent | `null`, `undefined` | `Option<T>` |
> | Handle errors | `try { ... } catch(e) { ... }` | `match result { Ok(v) => ..., Err(e) => ... }` |
> | Propagate errors | (implicit --- exceptions bubble up) | `?` operator (explicit --- each `?` is visible) |
> | Ignore errors | (easy --- just don't catch) | (impossible --- compiler forces handling) |
>
> The key difference: in JavaScript, error handling is opt-in (you choose to add `try/catch`). In Rust, error handling is opt-out (you must explicitly write `.unwrap()` to skip handling, and that will panic at runtime if the value is `Err`). The compiler nudges you toward safety.

---

## Exercise 1: Write `parse_weight` and Wire It Into the Form

**Goal:** Build a weight parser that demonstrates `Result<T, E>`, validation, and error propagation with `?`.

### Step 1: Add the validation module

> **Programming Concept: What is Validation?**
>
> Validation is checking that data is correct before using it. Think of it like a bouncer at a club checking IDs. The bouncer (validation) stands at the door (before data enters the system) and turns away anyone who does not meet the rules (invalid input). This keeps the inside (your program logic) safe and predictable.
>
> Without validation, garbage data flows through your system and causes problems far from where it entered --- making bugs incredibly hard to find.

Create a new file called `src/validation.rs`. This module will hold all our input checking logic, separate from the UI code:

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

Let us trace through `parse_weight("225 lbs")` step by step:

1. `input.trim()` removes any whitespace from both ends: `"225 lbs"`
2. `.trim_end_matches("lbs")` removes the "lbs" suffix: `"225 "`
3. `.trim_end_matches("kg")` does nothing (no "kg" to remove): `"225 "`
4. `.trim()` removes trailing whitespace: `"225"`
5. `cleaned.is_empty()` is `false`, so we skip the error
6. `cleaned.parse()` converts `"225"` to `225.0` --- this returns `Result<f64, ParseFloatError>`
7. `.map_err(...)` is not needed (it was `Ok`), so we get `Ok(225.0)`
8. The `?` unwraps `Ok(225.0)` into `225.0` and stores it in `weight`
9. `225.0 > 0.0` is `true`, so we skip the "must be positive" error
10. `225.0 > 2000.0` is `false`, so we skip the "exceeds maximum" error
11. We return `Ok(225.0)`

Now trace through `parse_weight("abc")`:

1. Steps 1-4 produce `"abc"`
2. `cleaned.parse()` fails --- `"abc"` is not a number, so it returns `Err(ParseFloatError)`
3. `.map_err(|_| format!("'{}' is not a valid number", cleaned))` transforms the error into a friendly message: `Err("'abc' is not a valid number")`
4. The `?` sees `Err(...)`, so it **stops here** and returns `Err("'abc' is not a valid number")` from the function
5. Lines 6-11 never run

This is the `?` operator in action. It is an early exit on failure.

### Step 2: Register the module

Tell Rust about the new file by adding it to `src/lib.rs`:

```rust
pub mod app;
pub mod data;
pub mod validation;
```

Remember: in Rust, simply creating a file is not enough. You must declare it with `mod` in the parent module. Think of `lib.rs` as a table of contents --- if a chapter is not listed, the compiler does not know it exists.

### Step 3: Add tests

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

Let us break this down piece by piece:

- **`#[cfg(test)]`** --- this entire module only exists when you run `cargo test`. It is not included in your production build. This means your tests live right next to the code they test, but they add zero overhead to the final binary.
- **`mod tests`** --- a module named `tests` (a convention, not a requirement).
- **`use super::*;`** --- import everything from the parent module (i.e., `validation.rs`). The `super` keyword means "the module above me."
- **`#[test]`** --- marks a function as a test case. `cargo test` finds and runs all functions with this attribute.
- **`assert_eq!(a, b)`** --- checks that `a` equals `b`. If they differ, the test fails with a message showing both values.
- **`assert!(condition)`** --- checks that the condition is true. `.is_err()` returns `true` if the `Result` is `Err`.

Run the tests:

```bash
cargo test
```

You should see output like:

```
running 3 tests
test validation::tests::parse_valid_weights ... ok
test validation::tests::parse_invalid_weights ... ok
test validation::tests::validate_names ... ok

test result: ok. 3 passed; 0 failed
```

Three tests, three passes. Your validation logic is verified. From now on, if someone accidentally changes `parse_weight` in a way that breaks existing behavior, `cargo test` will catch it.

<details>
<summary>Hint: If cargo test fails with "cannot find crate"</summary>

`cargo test` compiles the library crate without the `ssr` or `hydrate` features. If your `lib.rs` has `#[cfg(feature = "ssr")]` on module declarations that the test depends on, the test module will not be able to see those modules. The validation module we created has no feature gates, so it should compile cleanly. If you see errors related to other modules, they are likely gated behind a feature --- that is expected and not a problem for this exercise.

</details>

---

## Exercise 2: Build the Create/Edit Form with `Option<Exercise>` Toggle

**Goal:** Build a form component that handles both creating a new exercise and editing an existing one, using `Option<Exercise>` to toggle between modes.

### Step 1: Understand the pattern

Here is a design decision that will save you from writing duplicate code. A create form and an edit form have *the same fields*. The only difference is:

- **Create mode:** fields start empty, submit creates a new record
- **Edit mode:** fields start with existing values, submit updates the record

We can use `Option<Exercise>` to represent both:

```rust
// None = create mode (new exercise, empty form)
// Some(exercise) = edit mode (editing existing exercise, pre-filled form)
let editing: Option<Exercise> = None;
```

When `editing` is `None`, the form knows to start with blank fields. When it is `Some(exercise)`, the form pre-fills from the exercise's data. One component, two modes --- controlled by a single `Option`.

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
    // Initialize fields --- empty for create, pre-filled for edit
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

        // Validate the name --- Result guides us
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

Let us slow down and understand the three initialization lines, because they demonstrate `Option` combinators beautifully:

```rust
let init_name = exercise.as_ref().map(|e| e.name.clone()).unwrap_or_default();
```

Reading this left to right:

1. **`exercise.as_ref()`** --- converts `Option<Exercise>` to `Option<&Exercise>` (a reference, so we do not consume the exercise)
2. **`.map(|e| e.name.clone())`** --- if `Some(&exercise)`, extract and clone the name. If `None`, stay `None`. This transforms `Option<&Exercise>` into `Option<String>`.
3. **`.unwrap_or_default()`** --- if `Some(name)`, use it. If `None`, use the default for `String`, which is `""` (empty string).

So in create mode (`exercise` is `None`): `None` -> `None` -> `""`.
In edit mode (`exercise` is `Some(back_squat)`): `Some(&back_squat)` -> `Some("Back Squat")` -> `"Back Squat"`.

One line of code, both modes handled. No `if/else`. No `null` checks. The `Option` type guides the logic.

### Step 3: Understand `Callback`

`Callback<T>` is Leptos's way for child components to communicate with their parents. It is like an event handler you pass down:

```rust
// Parent creates the callback:
let on_save = Callback::new(move |exercise: Exercise| {
    exercises.update(|list| list.push(exercise));
    show_form.set(false);
});

// Child invokes it when the form is submitted:
on_save.run(new_exercise);
```

This is the same pattern as React's "lifting state up." The parent owns the data and passes down handlers. The child does not know or care what happens after it calls `on_save.run(...)` --- it just reports the event.

The key difference from JavaScript: Rust's type system ensures the callback signature matches at compile time. If the child tries to call `on_save.run("a string")` when the parent expects `Callback<Exercise>`, the code will not compile.

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
            // Edit mode --- replace the existing exercise by name
            exercises.update(|list| {
                if let Some(existing) = list.iter_mut().find(|e| e.name == exercise.name) {
                    existing.category = exercise.category;
                    existing.scoring_type = exercise.scoring_type;
                }
            });
        } else {
            // Create mode --- add to the list
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

            // ... rest of exercise list rendering from Chapter 3 ...
        </div>
    }
}
```

Notice the `if let Some(existing)` pattern inside the edit branch:

```rust
if let Some(existing) = list.iter_mut().find(|e| e.name == exercise.name) {
    existing.category = exercise.category;
    existing.scoring_type = exercise.scoring_type;
}
```

`if let Some(x) = expression` is a shorthand for matching only the `Some` case. It says: "If this Option contains a value, bind it to `existing` and run this block. If it is `None`, skip silently." This is perfect when you want to handle `Some` and do nothing for `None`.

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

Test it now: tap the FAB (floating action button), fill in an exercise name, and submit. The new exercise should appear in the list. Try submitting with an empty name --- you should see the red error banner.

<details>
<summary>Hint: If the form does not appear when you tap the FAB</summary>

The FAB toggles `show_form` between `true` and `false`. The form renders conditionally with `.then()`. If the form appears but immediately disappears, check that `on_cancel` is not being called on render. Make sure `on:click` is on the cancel *button*, not on the form itself.

</details>

---

## Exercise 3: Implement Soft Delete with Confirmation Modal

**Goal:** Add a delete button to exercise cards that soft-deletes (marks as deleted) rather than hard-deleting (removing permanently). Include an ownership check and a confirmation dialog.

### Step 1: Understand soft delete vs hard delete

When you delete something from your phone, it usually goes to a "Recently Deleted" folder. You can recover it within 30 days. After that, it is gone forever.

Software uses the same idea:

**Hard delete:** The data is gone. Permanently. Like shredding a paper document.

**Soft delete:** The data is still there, but marked as "deleted." Your queries skip over it. Like putting a paper in the recycling bin --- it is out of sight, but recoverable.

For now, we implement soft delete in memory. The pattern will transfer directly to PostgreSQL in Chapter 5.

```rust
// In your Exercise struct, add two new fields:
#[derive(Clone, Debug)]
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
    pub created_by: Option<String>,   // who created this exercise
    pub deleted_at: Option<String>,   // None = active, Some = soft-deleted
}
```

Notice how both new fields use `Option`. The `created_by` field is `None` for system-created exercises (the ones from `seed_exercises()`) and `Some("user_1")` for user-created ones. The `deleted_at` field is `None` for active exercises and `Some(timestamp)` for deleted ones.

### Step 2: Add the ownership check

We do not want anyone deleting exercises they did not create. The built-in exercises (like "Back Squat") have no owner --- they should not be deletable. User-created exercises should only be deletable by their creator.

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

Let us trace through the error handling chain:

1. **`.find()`** returns `Option<&mut Exercise>` --- the exercise might not exist in the list.
2. **`.ok_or_else()`** converts `None` into a descriptive `Err("Exercise not found")`. If it was `Some`, the value passes through unchanged.
3. **`?`** unwraps the `Ok` value or returns the `Err` early.
4. **`match &exercise.created_by`** handles three cases with a pattern match:
   - `Some(owner) if owner == requesting_user` --- the owner matches, proceed with delete
   - `Some(_)` --- there is an owner, but it is someone else
   - `None` --- no owner (system exercise)

The `if owner == requesting_user` part is a **match guard** --- an extra condition on a pattern. The pattern `Some(owner)` matches any `Some`, and the `if` clause narrows it further.

### Step 3: Build the confirmation modal

Before actually deleting, we should ask the user to confirm. Nobody wants to accidentally delete an exercise. This is a common UI pattern --- show a dialog that says "Are you sure?" with Cancel and Delete buttons.

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

A few things worth noting:

- **`on:click=move |ev| ev.stop_propagation()`** on the inner dialog prevents clicks on the dialog from closing the modal. Without this, clicking the "Cancel" button would also trigger the overlay's click handler (which closes the modal). `stop_propagation()` says "do not let this click event bubble up to the parent."
- **`&'static str`** for `title` and `subtitle` means these strings must live for the entire program. String literals like `"Delete Exercise?"` are `&'static str` by default, so this is fine for hardcoded UI text.

### Step 4: Wire up delete in the exercise card

Add delete state to `ExercisesPage`:

```rust
let show_delete = RwSignal::new(false);
let pending_delete_name = RwSignal::new(String::new());
let current_user = "user_1".to_string(); // hardcoded for now --- Chapter 7 adds auth

let on_delete = Callback::new(move |_: ()| {
    let name = pending_delete_name.get_untracked();
    exercises.update(|list| {
        match soft_delete_exercise(list, &name, &current_user) {
            Ok(()) => {} // success --- exercise is now soft-deleted
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

This code reads as: "If the exercise has an owner, and the owner is the current user, show a Delete button." The chain of `.map()` and `.then()` means the button only renders when both conditions are met. No `if/else` spaghetti --- just `Option` methods composing naturally.

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

The overlay uses `position: fixed; inset: 0;` which should cover the entire viewport. If the bottom nav peeks through, check that the overlay's `z-index: 200` is higher than the nav's `z-index: 100`. Also ensure the overlay is rendered outside the `<main>` element --- inside `<main>` with `overflow: hidden` might clip it.

</details>

---

## Exercise 4: Add Toast Notification Component

**Goal:** Build a toast notification system that shows error messages from failed operations (delete failures, validation errors) and auto-dismisses after a few seconds.

### Step 1: Create the toast signal

A toast notification is one of those small messages that slides in at the top of the screen, stays for a few seconds, and disappears. Like a piece of toast popping up from a toaster --- hence the name.

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

Let us walk through how this works:

1. Each toast gets a unique `id` (0, 1, 2, ...) so we can remove specific toasts later.
2. `toasts.update(|list| list.push(...))` adds the new toast to the end of the list.
3. `set_timeout` schedules a function to run after 3 seconds. That function removes the toast with the matching `id`.
4. The `#[cfg(feature = "hydrate")]` gate means the auto-dismiss only runs in the browser. On the server (during SSR), there is no event loop for timeouts.

The `.retain()` method on `Vec` keeps only elements that match the condition. `list.retain(|t| t.id != id_to_remove)` keeps every toast *except* the one we want to remove.

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

Now when a delete fails (wrong owner, no owner, exercise not found), the user sees a clear error message that auto-dismisses after 3 seconds.

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

Save everything, run `cargo leptos watch`, and try deleting a system exercise. You should see a toast pop up saying "This exercise has no owner and cannot be deleted," which fades away after 3 seconds.

<details>
<summary>Hint: If set_timeout is not found</summary>

`set_timeout` is provided by Leptos. Make sure you have `use leptos::prelude::*;` at the top of your file. The function takes a closure and a `Duration`, and runs the closure after the specified delay. It only works in the browser (hence the `#[cfg(feature = "hydrate")]` gate). On the server, the timeout would never fire because there is no event loop running for individual page renders.

</details>

---

## Rust Gym

These drills focus on the concepts from this chapter. They are simpler than the experienced track drills --- the goal is building confidence with `Result`, `Option`, and `?`. Try each one before looking at the solution.

### Drill 1: Reading `Result` and `Option`

For each expression, predict what the result will be. Write your prediction, then check.

```rust
// 1. What does this return?
"42".parse::<i32>()

// 2. What does this return?
"hello".parse::<i32>()

// 3. What does this return?
let names = vec!["Alice", "Bob", "Carol"];
names.iter().find(|n| **n == "Bob")

// 4. What does this return?
let names = vec!["Alice", "Bob", "Carol"];
names.iter().find(|n| **n == "Dave")

// 5. What does this evaluate to?
Some(5).unwrap_or(10)

// 6. What does this evaluate to?
None::<i32>.unwrap_or(10)
```

<details>
<summary>Solution</summary>

```rust
// 1. Ok(42)     --- the string "42" is a valid integer
// 2. Err(...)   --- "hello" cannot be parsed as an integer
// 3. Some(&"Bob")  --- found "Bob" in the list
// 4. None       --- "Dave" is not in the list
// 5. 5          --- Some(5) has a value, so unwrap_or ignores the default
// 6. 10         --- None has no value, so unwrap_or uses the default
```

Key takeaways:
- `.parse()` returns `Result` --- it can succeed or fail
- `.find()` returns `Option` --- the element might or might not exist
- `.unwrap_or(default)` extracts the value from `Some`/`Ok`, or uses the default for `None`/`Err`

</details>

### Drill 2: Using `?` to Clean Up Code

This function uses `match` everywhere. Rewrite it using `?` to make it shorter:

```rust
fn get_first_name(names: &[String], index: usize) -> Result<String, String> {
    let name = match names.get(index) {
        Some(n) => n,
        None => return Err(format!("Index {} out of bounds", index)),
    };

    let first = match name.split_whitespace().next() {
        Some(f) => f,
        None => return Err("Name is empty".to_string()),
    };

    Ok(first.to_string())
}
```

<details>
<summary>Hint</summary>

`.get(index)` returns `Option`. You can convert `Option` to `Result` using `.ok_or_else(|| ...)`. Once it is a `Result`, you can use `?` on it.

</details>

<details>
<summary>Solution</summary>

```rust
fn get_first_name(names: &[String], index: usize) -> Result<String, String> {
    let name = names
        .get(index)
        .ok_or_else(|| format!("Index {} out of bounds", index))?;

    let first = name
        .split_whitespace()
        .next()
        .ok_or_else(|| "Name is empty".to_string())?;

    Ok(first.to_string())
}
```

Each `match` block collapsed into a single line with `.ok_or_else()?`. The pattern is:
1. Start with an `Option` (from `.get()` or `.next()`)
2. Convert it to `Result` with `.ok_or_else()`
3. Propagate the error with `?`

</details>

### Drill 3: Building a Small Validator

Write a function `validate_age(input: &str) -> Result<u32, String>` that:
- Trims whitespace
- Returns `Err` if the input is empty
- Parses the string as `u32` (use `.parse()` and `.map_err()`)
- Returns `Err` if the age is less than 13 or greater than 120
- Returns `Ok(age)` if everything is valid

<details>
<summary>Hint</summary>

The structure is identical to `parse_weight`. Start by trimming, check for empty, parse with `?`, check the range.

</details>

<details>
<summary>Solution</summary>

```rust
fn validate_age(input: &str) -> Result<u32, String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err("Age cannot be empty".to_string());
    }

    let age: u32 = trimmed
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", trimmed))?;

    if age < 13 {
        return Err("Must be at least 13 years old".to_string());
    }

    if age > 120 {
        return Err("Age seems unrealistic".to_string());
    }

    Ok(age)
}
```

This follows the exact same pattern as `parse_weight`:
1. Clean the input
2. Early return on empty
3. Parse with `.map_err()` to give a friendly error message, then `?` to propagate
4. Range checks with early returns
5. Return the valid value wrapped in `Ok`

</details>

---

## What You Built

Take a moment to appreciate what you accomplished in this chapter:

1. **Input validation with `Result`** --- `parse_weight` and `validate_exercise_name` demonstrate how Rust functions communicate success and failure through their return types
2. **A create/edit form with `Option`** --- `ExerciseFormPanel` uses `Option<Exercise>` to elegantly handle two modes in one component
3. **Soft delete with ownership** --- `soft_delete_exercise` chains `Option` and `Result` methods with `?` for clean, readable error handling
4. **Toast notifications** --- a reactive signal-based system that shows error messages and auto-dismisses them
5. **Tests** --- `cargo test` verifies your validation logic works correctly

The big picture: Rust does not let you ignore errors. Where other languages silently produce `null`, `NaN`, or uncaught exceptions, Rust makes every possible failure visible in the type system. This feels restrictive at first. But once you internalize the pattern --- `Result` for things that can fail, `Option` for things that might not exist, `?` to propagate errors cleanly --- you will find that your code has fewer bugs and is easier to understand.

Right now, all this data lives in memory. Reload the page and your exercises vanish. In Chapter 5, we will connect GrindIt to a real database so your data survives.

---

### 🧬 DS Deep Dive

`Result<T, String>` works but falls apart at scale. This deep dive builds a real error system — custom types, automatic From conversion, the ? operator's internals, error chains, and the thiserror/anyhow ecosystem.

**→ [Error Handling Ecosystem — "The Incident Report Form"](../ds-narratives/ch04-error-handling-ecosystem.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| `src/validation.rs` | [`src/validation.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/validation.rs) |
| `ExerciseFormPanel` in `src/app.rs` | [`src/pages/exercises/exercise_form.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/exercise_form.rs) |
| `ConfirmModal` in `src/app.rs` | [`src/pages/exercises/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/mod.rs) |
| `soft_delete_exercise` | [`src/pages/exercises/server_fns.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/server_fns.rs) |
| Toast component | [`src/pages/exercises/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/exercises/mod.rs) |
