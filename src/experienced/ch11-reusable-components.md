# Chapter 11: Reusable Components

Every page in GrindIt shares the same patterns: a confirmation modal before deleting, a dropdown for selecting categories, a multi-select for muscle groups, a card layout for exercises. So far, these exist as inline code within each page. This chapter extracts them into a shared `components/` module: `DeleteModal`, `SingleSelect`, `MultiSelect`, and `ExerciseCard`. Each component exposes a clean prop interface, manages its own internal state with signals, and communicates with its parent through callbacks.

The spotlight concept is **ownership and borrowing** — the mechanism at the heart of Rust's memory safety. You will see why closures in Leptos need `move`, when to `clone` a signal before moving it, the difference between `Copy` and `Clone`, and how `RwSignal<T>` works as shared mutable state across parent and child components. This is where the borrow checker becomes a collaborator rather than an obstacle.

By the end of this chapter, you will have:

- A `DeleteModal` component with `show: RwSignal<bool>` and `on_confirm: impl Fn() + Copy + 'static` props
- A `SingleSelect` with smart dropdown positioning that flips up or down based on viewport space, using `web_sys::Element` and `DomRect`
- A `MultiSelect` with chip display, "clear all" button, and search filtering
- A refactored `ExerciseCard` that takes all its state as props
- A `components/mod.rs` barrel file re-exporting all components

---

## Spotlight: Ownership & Borrowing Deep Dive

### The fundamental rule

Rust has one rule that governs all memory access: **at any given time, a value has either one mutable reference OR any number of immutable references, but not both.** This is enforced at compile time by the borrow checker.

In a language like JavaScript, shared mutable state is the default:

```javascript
// JavaScript: anyone can mutate this object at any time
const state = { show: false };
openModal(state);   // sets state.show = true
closeModal(state);  // sets state.show = false
// No compiler checks for conflicting mutations
```

In Rust, you cannot hand out multiple mutable references to the same data:

```rust
let mut show = false;
let r1 = &mut show;  // first mutable borrow
let r2 = &mut show;  // ERROR: cannot borrow `show` as mutable more than once
```

This prevents data races, use-after-free, and the class of bugs where one part of the code mutates data that another part is reading. But UI components *need* shared mutable state — a parent and child both need to read and write the same `show` flag.

### RwSignal: shared mutable state that satisfies the borrow checker

Leptos solves this with `RwSignal<T>`. A signal is a `Copy` smart pointer (internally backed by `Arc`) that provides runtime-checked shared mutability:

```rust
let show = RwSignal::new(false);

// Both closures capture `show` by copy — no borrow conflict
let open = move || show.set(true);
let close = move || show.set(false);
let is_open = move || show.get();
```

`RwSignal<T>` implements `Copy`, which means capturing it in a closure does not move it out of the original scope. Multiple closures can hold copies of the same signal, and all copies point to the same underlying reactive cell. This is the Leptos escape hatch from the borrow checker's exclusive-mutability rule — the signal provides interior mutability (like `RefCell`, but reactive and thread-safe).

### The clone-then-move pattern

Not everything is `Copy`. Strings, vectors, and complex types implement `Clone` but not `Copy`. When a closure needs to capture a non-`Copy` value and you also need that value elsewhere, you must clone before moving:

```rust
let log_id = entry.log.id.clone();  // String, not Copy

// This closure moves `log_id` into itself
let on_delete = move |_| {
    pending_delete_log_id.set(log_id.clone());
    show_delete.set(true);
};
```

Wait — why do we clone `log_id` *again* inside the closure? Because `on_delete` might be called multiple times (the delete button can be clicked more than once). If the closure moved `log_id` on the first call, it would be empty on the second call. The inner `.clone()` ensures the closure always has a value to give.

This is a common confusion. Let us trace ownership step by step:

```rust
let id = "abc-123".to_string();    // id owns the string

let id_for_closure = id.clone();    // cloned for the closure
let on_click = move || {            // closure takes ownership of id_for_closure
    do_something(id_for_closure.clone()); // clone again because Fn, not FnOnce
};

println!("{}", id);                 // original id is still usable
on_click();                         // first call works
on_click();                         // second call works — id_for_closure was cloned, not moved
```

> **Coming from JS?** In JavaScript, closures capture variables by reference — all closures sharing the variable see the same value. In Rust, `move` closures take *ownership* of the captured values. If a value is `Copy` (like `RwSignal`, `i32`, `bool`), the closure gets a copy and the original remains. If a value is `Clone` but not `Copy` (like `String`), the closure takes the original and you must manually clone if you need the value elsewhere. React's `useCallback` dependency arrays are solving a similar problem — stale closures in JavaScript are the equivalent of Rust's ownership errors.

### Copy vs Clone

`Copy` is a marker trait for types that can be duplicated with a simple bitwise copy. `Clone` is for types that need a custom duplication strategy (allocating new memory, incrementing reference counts, etc.).

| Type | Copy? | Clone? | Why |
|------|-------|--------|-----|
| `i32`, `f64`, `bool` | Yes | Yes | Stack-only, fixed size |
| `RwSignal<T>` | Yes | Yes | Small pointer (index + runtime ID) |
| `String` | No | Yes | Heap-allocated, needs new allocation on clone |
| `Vec<T>` | No | Yes | Heap-allocated, needs deep copy |
| `&str` | Yes | Yes | Just a pointer + length, no ownership |

Rule of thumb for Leptos: **signals are `Copy` and go anywhere freely.** Strings and vectors are `Clone` and need the clone-before-move pattern.

### impl Fn() + Copy + 'static for callback props

When a component accepts a callback prop, you need to specify the trait bounds:

```rust
#[component]
pub fn DeleteModal(
    show: RwSignal<bool>,
    on_confirm: impl Fn() + Copy + 'static,
) -> impl IntoView {
    // ...
}
```

Breaking this down:

- **`impl Fn()`** — the prop is a callable that takes no arguments and returns nothing. `Fn` (not `FnMut` or `FnOnce`) means it can be called multiple times without mutating its captured state.
- **`+ Copy`** — the closure must be `Copy` so it can be captured by multiple inner closures (the confirm button's `on:click` handler, for example). This is satisfied when all captured values are `Copy` — which is true if the closure only captures signals.
- **`+ 'static`** — the closure must own all its data (no borrowed references that could dangle). `move` closures that capture only `Copy` or owned types always satisfy `'static`.

If your callback needs to capture a `String`, it will not be `Copy`. In that case, use `impl Fn() + Clone + 'static` instead, and clone the callback before passing it to inner closures.

---

## Building the DeleteModal

The `DeleteModal` is the simplest reusable component — a confirmation overlay for destructive actions. It knows nothing about *what* it is deleting. It only knows how to show itself, display a message, and call a callback when confirmed.

```rust
#[component]
pub fn DeleteModal(
    show: RwSignal<bool>,
    #[prop(default = "Delete this item?")] title: &'static str,
    #[prop(default = "This cannot be undone.")] subtitle: &'static str,
    #[prop(default = "Delete")] confirm_label: &'static str,
    on_confirm: impl Fn() + Copy + 'static,
) -> impl IntoView {
    view! {
        <div
            class="confirm-overlay"
            style=move || if show.get() { "display:flex" } else { "display:none" }
            on:click=move |_| show.set(false)
        >
            <div class="confirm-dialog" on:click=move |ev| { ev.stop_propagation(); }>
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
                            on_confirm();
                            show.set(false);
                        }
                    >{confirm_label}</button>
                </div>
            </div>
        </div>
    }
}
```

### Design Insight: Information Hiding

John Ousterhout's *A Philosophy of Software Design* identifies **information hiding** as the most important technique for managing complexity. A module should expose a simple interface and hide its implementation details.

`DeleteModal` is a textbook example. The parent component (history page, exercises page) knows *what* to delete and *how* to delete it. `DeleteModal` only knows *how to ask for confirmation*. It does not import any database types, does not know about workout logs or exercises, and does not handle errors. This means:

- The same modal works for deleting workouts, exercises, users, and anything else
- Changes to the deletion logic never require changes to the modal
- The modal can be tested in isolation

The `on_confirm` callback is the interface contract: "when the user clicks Confirm, call this function." Everything else is hidden.

### DSA connection: Stack and modal z-index

Modal dialogs follow a **LIFO (Last In, First Out)** pattern — the most recently opened modal sits on top and must be dismissed first. This is a stack.

In CSS, z-index creates the visual stacking context. The overlay's `display:flex` with a semi-transparent background creates a modal barrier. The `ev.stop_propagation()` on the dialog prevents clicks inside the dialog from triggering the overlay's close handler. This click-event propagation model is itself a stack-like traversal: events bubble from child to parent, and `stopPropagation()` pops the current handler off the bubbling stack.

---

## Building the SingleSelect

The `SingleSelect` is a searchable dropdown that replaces the native `<select>` element. It opens a floating panel with search filtering and smart positioning — the dropdown flips upward when there is not enough space below the trigger.

### Smart positioning with web_sys

The positioning logic uses the Web API's `getBoundingClientRect()` to measure where the trigger button sits relative to the viewport:

```rust
let toggle = move |_: leptos::ev::MouseEvent| {
    if !open.get_untracked() {
        #[cfg(target_arch = "wasm32")]
        if let Some(el) = trigger_ref.get_untracked() {
            let el_ref: &web_sys::Element = el.as_ref();
            let rect = el_ref.get_bounding_client_rect();

            let vp_height = web_sys::window()
                .and_then(|w| w.inner_height().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(667.0);

            let nav_h = 70.0_f64;  // bottom nav + safe area
            let space_below = (vp_height - rect.bottom() - nav_h).max(0.0);
            let space_above = rect.top();

            let flip = space_below < 200.0 && space_above > space_below;
            opens_up.set(flip);
            dropdown_left.set(rect.left());
            dropdown_width.set(rect.width());

            if flip {
                dropdown_bottom.set(vp_height - rect.top() + 4.0);
                dropdown_max_height.set(
                    (space_above - 12.0).clamp(120.0, 360.0)
                );
            } else {
                dropdown_top.set(rect.bottom() + 4.0);
                dropdown_max_height.set(
                    (space_below - 12.0).clamp(120.0, 360.0)
                );
            }
        }
    } else {
        search.set(String::new());
    }
    open.update(|v| *v = !*v);
};
```

Key ownership observations:

1. **`trigger_ref = NodeRef::<leptos::html::Button>::new()`** — a `NodeRef` is `Copy` (it is a signal internally), so it can be captured by the toggle closure without cloning.
2. **`el.as_ref()` converts the Leptos element to `&web_sys::Element`** — this is a borrow, not a move. The element is still owned by the DOM; we only need a reference to read its bounding rect.
3. **The `#[cfg(target_arch = "wasm32")]` guard** ensures this code only compiles for the browser target. During SSR, there is no DOM and no `web_sys`.
4. **Multiple signals for position** (`dropdown_top`, `dropdown_left`, `dropdown_width`, etc.) — each is `Copy`, so the closure captures them all without ownership conflicts.

### Portal for DOM escape

The dropdown is rendered using `<Portal>`, which teleports its children to `document.body`:

```rust
<Portal>
    {move || {
        if !open.get() {
            return ().into_any();
        }

        let query = search.get().to_lowercase();
        let opts = all_options.get();
        let filtered: Vec<_> = opts.iter()
            .filter(|o| query.is_empty() || o.label.to_lowercase().contains(&query))
            .cloned()
            .collect();

        let style = if opens_up.get_untracked() {
            format!(
                "bottom: {}px; left: {}px; min-width: {}px; max-height: {}px;",
                dropdown_bottom.get_untracked(),
                dropdown_left.get_untracked(),
                dropdown_width.get_untracked(),
                dropdown_max_height.get_untracked(),
            )
        } else {
            format!(
                "top: {}px; left: {}px; min-width: {}px; max-height: {}px;",
                dropdown_top.get_untracked(),
                dropdown_left.get_untracked(),
                dropdown_width.get_untracked(),
                dropdown_max_height.get_untracked(),
            )
        };

        view! {
            <div class="single-select__backdrop" on:click=move |_| close()></div>
            <div class="single-select__dropdown" style=style>
                // search input and options list ...
            </div>
        }.into_any()
    }}
</Portal>
```

Why Portal? Without it, the dropdown would be clipped by any parent with `overflow: hidden` — which is common in card layouts, modals, and scrollable containers. Portal renders the dropdown at the top level of the DOM, then positions it absolutely using the computed `top`/`left`/`bottom` values. The backdrop (`single-select__backdrop`) covers the entire viewport and closes the dropdown when clicked.

### The SelectOption type

Both `SingleSelect` and `MultiSelect` share a common option type:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}
```

`PartialEq` enables comparison (`== ` and `!=`). `Clone` enables the option to be duplicated when building the filtered list. The `value` is what gets stored in the signal; the `label` is what the user sees. This separation is important — a category dropdown might show "Olympic Lifting" (label) but store "olympic" (value).

### System Design: Component Library API Surface

Good component APIs follow the **principle of least astonishment**: the component should behave as the caller expects, with sensible defaults and no hidden side effects.

The `SingleSelect` API demonstrates this:

```rust
#[component]
pub fn SingleSelect(
    options: Vec<SelectOption>,       // what to show
    selected: RwSignal<String>,       // two-way binding
    #[prop(default = "Select...")] placeholder: &'static str,  // sensible default
) -> impl IntoView
```

Three props. The `options` are passed by value (the component owns them). The `selected` signal is passed by copy (shared between parent and child). The `placeholder` has a default so callers can omit it.

Compare this to a bad API that exposes internal state:

```rust
// BAD: exposes internal concerns
fn SingleSelect(
    options: Vec<SelectOption>,
    selected: RwSignal<String>,
    open: RwSignal<bool>,           // caller should not control this
    search_query: RwSignal<String>, // implementation detail
    dropdown_position: (f64, f64),  // layout detail
)
```

The caller does not care about the dropdown's open state, search query, or positioning. Those are internal implementation details that should be hidden behind the component boundary.

---

## Building the MultiSelect

The `MultiSelect` extends the pattern to multiple selections, displayed as chips below the trigger:

```rust
#[component]
pub fn MultiSelect(
    options: Vec<SelectOption>,
    selected: RwSignal<Vec<String>>,
    #[prop(default = "Select...")] placeholder: &'static str,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let all_options = RwSignal::new(options);

    // ... toggle and close closures
```

The key difference from `SingleSelect`: the `selected` signal holds a `Vec<String>` instead of a `String`. Toggling a selection uses `signal.update()` with a closure that modifies the vector in place:

```rust
on:change=move |_| {
    let v = val_toggle.clone();
    selected.update(|s| {
        if s.contains(&v) {
            s.retain(|x| x != &v);  // remove if present
        } else {
            s.push(v);              // add if absent
        }
    });
}
```

`signal.update(|s| ...)` passes a mutable reference to the inner value. This is more efficient than `signal.set(new_value)` for collections because it avoids cloning the entire vector — you modify it in place.

### Chip display with clear-all

Selected values appear as chips below the trigger:

```rust
{move || {
    let sel = selected.get();
    if sel.is_empty() {
        ().into_any()
    } else {
        let opts = all_options.get();
        let chips: Vec<_> = opts.iter()
            .filter(|o| sel.contains(&o.value))
            .cloned()
            .collect();
        view! {
            <div class="multi-select__chips">
                {chips.into_iter().map(|opt| {
                    let val = opt.value.clone();
                    view! {
                        <span class="multi-select__chip">
                            {opt.label}
                            <button type="button" class="multi-select__chip-remove"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    let v = val.clone();
                                    selected.update(|s| s.retain(|x| x != &v));
                                }
                            >"x"</button>
                        </span>
                    }
                }).collect_view()}
                <button type="button" class="multi-select__clear"
                    on:click=move |ev| {
                        ev.stop_propagation();
                        selected.set(vec![]);
                    }
                >"Clear all"</button>
            </div>
        }.into_any()
    }
}}
```

Notice the clone chain: `opt.value.clone()` creates `val`, which is then cloned again inside the `on:click` closure. This is the clone-then-move pattern applied to an iterator — each chip gets its own closure, and each closure needs its own copy of the value string.

---

## Refactoring ExerciseCard

The `ExerciseCard` is the most complex reusable component. It accepts an exercise, shared signals for the expanded/editing state, and action props for update and delete:

```rust
#[component]
pub fn ExerciseCard(
    exercise: Exercise,
    expanded_id: RwSignal<Option<String>>,
    editing_exercise: RwSignal<Option<String>>,
    update_action: ServerAction<UpdateExercise>,
    pending_delete_id: RwSignal<String>,
    show_delete: RwSignal<bool>,
    is_coach: bool,
    is_admin: bool,
    current_user_id: Option<String>,
) -> impl IntoView {
    let id = exercise.id.clone();
    let can_delete = is_admin
        || current_user_id.as_deref()
            .zip(exercise.created_by.as_deref())
            .map(|(uid, owner)| uid == owner)
            .unwrap_or(false);
    // ...
}
```

### The ownership challenge of Option::zip

The `can_delete` check uses a subtle ownership pattern:

```rust
let can_delete = is_admin
    || current_user_id
        .as_deref()                              // Option<String> -> Option<&str>
        .zip(exercise.created_by.as_deref())     // Option<(&str, &str)>
        .map(|(uid, owner)| uid == owner)        // Option<bool>
        .unwrap_or(false);                       // bool
```

`as_deref()` converts `Option<String>` to `Option<&str>` — borrowing the inner string without taking ownership. `zip` combines two `Option`s into one — if either is `None`, the result is `None`. This is a clean, functional way to express "if both user ID and creator ID exist, compare them."

### Edit form signals

The card maintains its own edit state with local signals:

```rust
let edit_name = RwSignal::new(String::new());
let edit_category = RwSignal::new(String::new());
let edit_movement_type = RwSignal::new(String::new());
// ...

// When the edit button is clicked, populate signals from the exercise data
let init_name = exercise.name.clone();
let init_cat = exercise.category.clone();

// In the edit button handler:
edit_name.set(iname.clone());
edit_category.set(icat.clone());
editing_exercise.set(Some(eid_edit.clone()));
```

The init values (`init_name`, `init_cat`, etc.) are cloned from the exercise at component creation time. They are then cloned again into the click handler's closure. This double-clone is necessary because the exercise data is owned by the component function's scope, but the closure lives beyond that scope.

### Conditional rendering based on role

The card conditionally shows edit/delete buttons based on the user's role:

```rust
{is_coach.then(|| view! {
    <div class="exercise-panel-actions">
        <button class="exercise-edit-btn"
            on:click=move |_| {
                // populate edit signals
                editing_exercise.set(Some(eid_edit.clone()));
            }
        >"Edit"</button>
        {can_delete.then(|| view! {
            <button class="exercise-delete"
                on:click=move |_| {
                    pending_delete_id.set(id_del.clone());
                    show_delete.set(true);
                }
            >"Delete"</button>
        })}
    </div>
})}
```

`bool::then(|| ...)` returns `Some(view)` if true, `None` if false. Leptos renders `None` as nothing — no DOM node, no placeholder. This is the Rust equivalent of `{isCoach && <div>...</div>}` in React JSX.

---

## The Barrel File

The `components/mod.rs` re-exports all components for convenient imports:

```rust
pub mod delete_modal;
pub mod multi_select;
pub mod single_select;
pub mod video_upload;

pub use delete_modal::DeleteModal;
pub use multi_select::{MultiSelect, SelectOption};
pub use single_select::SingleSelect;
pub use video_upload::VideoUpload;
```

Consumers import from the barrel:

```rust
use crate::components::{DeleteModal, MultiSelect, SelectOption, SingleSelect};
```

This pattern hides the file structure (no one needs to know that `DeleteModal` lives in `delete_modal.rs`) and provides a stable import path even if you reorganize the internal files.

---

## Rust Gym

### Clone vs Copy exercises

```rust
fn demonstrate_copy_vs_clone() {
    // Copy types: bitwise duplication
    let signal = RwSignal::new(42);
    let s2 = signal;          // copy, both signal and s2 are valid
    signal.set(100);          // original still works
    assert_eq!(s2.get(), 100); // s2 sees the change (same underlying cell)

    // Clone types: explicit duplication
    let name = String::from("Alice");
    let name2 = name.clone();  // explicit clone
    // let name3 = name;       // this would MOVE name, making it unusable
    assert_eq!(name, "Alice"); // original still valid because we cloned, not moved
    assert_eq!(name2, "Alice");
}
```

<details>
<summary>Exercise: which of these compile?</summary>

```rust
fn test_a() {
    let s = RwSignal::new(0);
    let f1 = move || s.get();
    let f2 = move || s.get();    // Does this compile?
    println!("{} {}", f1(), f2());
}

fn test_b() {
    let name = String::from("hello");
    let f1 = move || name.len();
    let f2 = move || name.len(); // Does this compile?
}

fn test_c() {
    let name = String::from("hello");
    let name2 = name.clone();
    let f1 = move || name.len();
    let f2 = move || name2.len(); // Does this compile?
}
```

**Answers:**
- `test_a`: Compiles. `RwSignal` is `Copy`, so both closures get a copy.
- `test_b`: Does NOT compile. `String` is not `Copy`. `f1` takes ownership of `name`, and `f2` cannot take ownership of something already moved.
- `test_c`: Compiles. `name` is moved into `f1`, `name2` (a clone) is moved into `f2`. Each closure owns its own `String`.
</details>

### Borrow checker puzzles

<details>
<summary>Exercise: fix this component</summary>

This code does not compile. Fix it.

```rust
#[component]
fn Broken(items: Vec<String>) -> impl IntoView {
    let on_click = |item: &String| {
        log!("clicked {}", item);
    };
    view! {
        <ul>
            {items.iter().map(|item| {
                view! {
                    <li on:click=move |_| on_click(item)>{item}</li>
                }
            }).collect_view()}
        </ul>
    }
}
```

**Solution:**

```rust
#[component]
fn Fixed(items: Vec<String>) -> impl IntoView {
    view! {
        <ul>
            {items.into_iter().map(|item| {
                let display = item.clone();
                view! {
                    <li on:click=move |_| {
                        log!("clicked {}", item);
                    }>{display}</li>
                }
            }).collect_view()}
        </ul>
    }
}
```

Changes: `into_iter()` instead of `iter()` to take ownership of each `String`. Clone `item` into `display` for the view text (since `item` is moved into the closure). The closure owns `item` directly, no borrowing needed.
</details>

### Signal sharing patterns

<details>
<summary>Exercise: implement a counter shared between two components</summary>

```rust
#[component]
fn Parent() -> impl IntoView {
    let count = RwSignal::new(0);
    view! {
        <Display count=count/>
        <Controls count=count/>
    }
}

#[component]
fn Display(count: RwSignal<i32>) -> impl IntoView {
    view! { <p>"Count: " {move || count.get()}</p> }
}

#[component]
fn Controls(count: RwSignal<i32>) -> impl IntoView {
    view! {
        <button on:click=move |_| count.update(|n| *n += 1)>"+"</button>
        <button on:click=move |_| count.update(|n| *n -= 1)>"-"</button>
    }
}
```

Because `RwSignal` is `Copy`, passing it to both child components creates no ownership conflict. Both components read and write the same underlying reactive cell.
</details>

---

## Exercises

### Exercise 1: Build DeleteModal with signal props and callback

Build the `DeleteModal` component. It should accept `show: RwSignal<bool>` for visibility, customizable title/subtitle/confirm label with defaults, and an `on_confirm: impl Fn() + Copy + 'static` callback. The overlay should close when the backdrop is clicked, when Cancel is clicked, and after Confirm runs the callback.

<details>
<summary>Hints</summary>

- Use `#[prop(default = "...")]` for the title, subtitle, and confirm_label props
- The overlay uses `style=move || if show.get() { "display:flex" } else { "display:none" }`
- Use `ev.stop_propagation()` on the dialog div to prevent backdrop clicks from triggering when clicking inside the dialog
- After `on_confirm()`, call `show.set(false)` to close the modal
</details>

<details>
<summary>Solution</summary>

```rust
use leptos::prelude::*;

#[component]
pub fn DeleteModal(
    show: RwSignal<bool>,
    #[prop(default = "Delete this item?")] title: &'static str,
    #[prop(default = "This cannot be undone.")] subtitle: &'static str,
    #[prop(default = "Delete")] confirm_label: &'static str,
    on_confirm: impl Fn() + Copy + 'static,
) -> impl IntoView {
    view! {
        <div
            class="confirm-overlay"
            style=move || if show.get() { "display:flex" } else { "display:none" }
            on:click=move |_| show.set(false)
        >
            <div class="confirm-dialog" on:click=move |ev| { ev.stop_propagation(); }>
                <p class="confirm-msg">{title}</p>
                <p class="confirm-sub">{subtitle}</p>
                <div class="confirm-actions">
                    <button class="confirm-cancel-btn"
                        on:click=move |_| show.set(false)
                    >"Cancel"</button>
                    <button class="confirm-delete-btn"
                        on:click=move |_| {
                            on_confirm();
                            show.set(false);
                        }
                    >{confirm_label}</button>
                </div>
            </div>
        </div>
    }
}
```

Note that `on_confirm` is `Copy + 'static`, so it can be captured by the button's `on:click` closure without cloning. The `show` signal is also `Copy`. This component has zero `clone()` calls — everything is either `Copy` or `&'static str`.
</details>

### Exercise 2: Build SingleSelect with smart positioning

Build the `SingleSelect` component with a trigger button, a search input, and a list of options rendered via `<Portal>`. The dropdown should flip upward when there is not enough space below the trigger (less than 200px). Use `web_sys::Element::get_bounding_client_rect()` for measurements.

<details>
<summary>Hints</summary>

- Use `NodeRef::<leptos::html::Button>::new()` for the trigger reference
- Cast with `el.as_ref()` to get `&web_sys::Element`, then call `get_bounding_client_rect()`
- Get viewport height from `web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64())`
- Subtract 70px for the bottom navigation bar when calculating space below
- Use `clamp(120.0, 360.0)` to constrain the dropdown's max height
- Position with `position: fixed` in CSS, using top/left or bottom/left based on the flip direction
</details>

<details>
<summary>Solution</summary>

```rust
use super::SelectOption;
use leptos::portal::Portal;
use leptos::prelude::*;

#[component]
pub fn SingleSelect(
    options: Vec<SelectOption>,
    selected: RwSignal<String>,
    #[prop(default = "Select...")] placeholder: &'static str,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let all_options = RwSignal::new(options);
    let trigger_ref = NodeRef::<leptos::html::Button>::new();

    let dropdown_top = RwSignal::new(0.0_f64);
    let dropdown_bottom = RwSignal::new(0.0_f64);
    let dropdown_left = RwSignal::new(0.0_f64);
    let dropdown_width = RwSignal::new(0.0_f64);
    let dropdown_max_height = RwSignal::new(300.0_f64);
    let opens_up = RwSignal::new(false);

    let toggle = move |_: leptos::ev::MouseEvent| {
        if !open.get_untracked() {
            #[cfg(target_arch = "wasm32")]
            if let Some(el) = trigger_ref.get_untracked() {
                let el_ref: &web_sys::Element = el.as_ref();
                let rect = el_ref.get_bounding_client_rect();
                let vp_height = web_sys::window()
                    .and_then(|w| w.inner_height().ok())
                    .and_then(|v| v.as_f64())
                    .unwrap_or(667.0);
                let nav_h = 70.0_f64;
                let space_below = (vp_height - rect.bottom() - nav_h).max(0.0);
                let space_above = rect.top();
                let flip = space_below < 200.0 && space_above > space_below;
                opens_up.set(flip);
                dropdown_left.set(rect.left());
                dropdown_width.set(rect.width());
                if flip {
                    dropdown_bottom.set(vp_height - rect.top() + 4.0);
                    dropdown_max_height.set((space_above - 12.0).clamp(120.0, 360.0));
                } else {
                    dropdown_top.set(rect.bottom() + 4.0);
                    dropdown_max_height.set((space_below - 12.0).clamp(120.0, 360.0));
                }
            }
        } else {
            search.set(String::new());
        }
        open.update(|v| *v = !*v);
    };

    let close = move || {
        open.set(false);
        search.set(String::new());
    };

    view! {
        <div class="single-select">
            <button node_ref=trigger_ref type="button"
                class="single-select__trigger" on:click=toggle>
                <span class="single-select__label">
                    {move || {
                        let val = selected.get();
                        if val.is_empty() {
                            placeholder.to_string()
                        } else {
                            all_options.get().iter()
                                .find(|o| o.value == val)
                                .map(|o| o.label.clone())
                                .unwrap_or(val)
                        }
                    }}
                </span>
                <span class="single-select__arrow" class:open=move || open.get()></span>
            </button>
            <Portal>
                {move || {
                    if !open.get() { return ().into_any(); }
                    let query = search.get().to_lowercase();
                    let opts = all_options.get();
                    let filtered: Vec<_> = opts.iter()
                        .filter(|o| query.is_empty()
                            || o.label.to_lowercase().contains(&query))
                        .cloned().collect();
                    let style = if opens_up.get_untracked() {
                        format!("bottom:{}px;left:{}px;min-width:{}px;max-height:{}px;",
                            dropdown_bottom.get_untracked(),
                            dropdown_left.get_untracked(),
                            dropdown_width.get_untracked(),
                            dropdown_max_height.get_untracked())
                    } else {
                        format!("top:{}px;left:{}px;min-width:{}px;max-height:{}px;",
                            dropdown_top.get_untracked(),
                            dropdown_left.get_untracked(),
                            dropdown_width.get_untracked(),
                            dropdown_max_height.get_untracked())
                    };
                    view! {
                        <div class="single-select__backdrop"
                            on:click=move |_| close()></div>
                        <div class="single-select__dropdown" style=style>
                            <input type="text" class="single-select__search"
                                placeholder="Search..."
                                prop:value=move || search.get()
                                on:input=move |ev| search.set(event_target_value(&ev))
                                on:click=move |ev| ev.stop_propagation()/>
                            <div class="single-select__options">
                                {filtered.into_iter().map(|opt| {
                                    let val_check = opt.value.clone();
                                    let val_click = opt.value.clone();
                                    let label = opt.label.clone();
                                    view! {
                                        <div class="single-select__option"
                                            class:selected=move || selected.get() == val_check
                                            on:click=move |ev| {
                                                ev.stop_propagation();
                                                selected.set(val_click.clone());
                                                close();
                                            }>
                                            <span class="single-select__check"></span>
                                            {label}
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }.into_any()
                }}
            </Portal>
        </div>
    }
}
```

The key insight: all positioning state uses `RwSignal<f64>` (which is `Copy`), so the toggle closure captures six position signals plus the trigger ref without any ownership conflicts.
</details>

### Exercise 3: Build MultiSelect with chip display and "clear all"

Build the `MultiSelect` component. It should show a trigger button with a count label ("3 selected"), render selected values as removable chips below the trigger, provide a "Clear all" button, and offer a searchable dropdown with checkboxes.

<details>
<summary>Hints</summary>

- Use `selected: RwSignal<Vec<String>>` for the two-way binding
- Display label: `if sel.is_empty() { placeholder } else { format!("{} selected", sel.len()) }`
- Toggle logic: `selected.update(|s| { if s.contains(&v) { s.retain(|x| x != &v); } else { s.push(v); } })`
- Chip removal: `selected.update(|s| s.retain(|x| x != &v))`
- Clear all: `selected.set(vec![])`
- Use `ev.stop_propagation()` on chip remove buttons to prevent the trigger from toggling
</details>

<details>
<summary>Solution</summary>

```rust
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[component]
pub fn MultiSelect(
    options: Vec<SelectOption>,
    selected: RwSignal<Vec<String>>,
    #[prop(default = "Select...")] placeholder: &'static str,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let all_options = RwSignal::new(options);

    let toggle = move |_: leptos::ev::MouseEvent| {
        open.update(|v| *v = !*v);
        if !open.get_untracked() { search.set(String::new()); }
    };
    let close = move || { open.set(false); search.set(String::new()); };

    view! {
        <div class="multi-select">
            <button type="button" class="multi-select__trigger" on:click=toggle>
                <span class="multi-select__label">
                    {move || {
                        let sel = selected.get();
                        if sel.is_empty() { placeholder.to_string() }
                        else { format!("{} selected", sel.len()) }
                    }}
                </span>
            </button>

            // Chips
            {move || {
                let sel = selected.get();
                if sel.is_empty() { return ().into_any(); }
                let opts = all_options.get();
                let chips: Vec<_> = opts.iter()
                    .filter(|o| sel.contains(&o.value)).cloned().collect();
                view! {
                    <div class="multi-select__chips">
                        {chips.into_iter().map(|opt| {
                            let val = opt.value.clone();
                            view! {
                                <span class="multi-select__chip">
                                    {opt.label}
                                    <button type="button" on:click=move |ev| {
                                        ev.stop_propagation();
                                        let v = val.clone();
                                        selected.update(|s| s.retain(|x| x != &v));
                                    }>"x"</button>
                                </span>
                            }
                        }).collect_view()}
                        <button type="button" class="multi-select__clear"
                            on:click=move |ev| {
                                ev.stop_propagation();
                                selected.set(vec![]);
                            }
                        >"Clear all"</button>
                    </div>
                }.into_any()
            }}

            // Dropdown
            {move || {
                if !open.get() { return ().into_any(); }
                let query = search.get().to_lowercase();
                let opts = all_options.get();
                let filtered: Vec<_> = opts.iter()
                    .filter(|o| query.is_empty()
                        || o.label.to_lowercase().contains(&query))
                    .cloned().collect();
                view! {
                    <div class="multi-select__backdrop"
                        on:click=move |_| close()></div>
                    <div class="multi-select__dropdown">
                        <input type="text" class="multi-select__search"
                            placeholder="Search..."
                            prop:value=move || search.get()
                            on:input=move |ev| search.set(event_target_value(&ev))/>
                        <div class="multi-select__options">
                            {filtered.into_iter().map(|opt| {
                                let val_check = opt.value.clone();
                                let val_toggle = opt.value.clone();
                                view! {
                                    <label class="multi-select__option">
                                        <input type="checkbox"
                                            prop:checked=move || {
                                                selected.get().contains(&val_check)
                                            }
                                            on:change=move |_| {
                                                let v = val_toggle.clone();
                                                selected.update(|s| {
                                                    if s.contains(&v) {
                                                        s.retain(|x| x != &v);
                                                    } else { s.push(v); }
                                                });
                                            }/>
                                        {opt.label}
                                    </label>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                }.into_any()
            }}
        </div>
    }
}
```

The core ownership pattern: each chip's remove button closure captures a cloned copy of the value string. The `selected.update()` call modifies the vector in place without cloning the entire vector. The double-clone (`opt.value.clone()` then `val.clone()`) is required because the `on:click` closure implements `Fn`, not `FnOnce`.
</details>

### Exercise 4: Refactor ExerciseCard and wire all components together

Refactor the `ExerciseCard` as a standalone component that accepts an `Exercise`, shared signals for expanded/editing state, the update action, delete signals, and role booleans. Wire `DeleteModal`, `SingleSelect`, and `ExerciseCard` together in the exercises page.

<details>
<summary>Hints</summary>

- The card needs many props: `exercise`, `expanded_id`, `editing_exercise`, `update_action`, `pending_delete_id`, `show_delete`, `is_coach`, `is_admin`, `current_user_id`
- Use `bool::then(|| view! { ... })` for conditional rendering based on role
- Use `Option::zip` with `as_deref()` for the ownership check
- The edit form uses `SingleSelect` for category and scoring type dropdowns
- `DeleteModal` is placed once in the parent page, not inside each card
</details>

<details>
<summary>Solution</summary>

The key architectural decision: `DeleteModal` is instantiated once in the parent page and controlled by shared signals (`show_delete`, `pending_delete_id`). Each `ExerciseCard` writes to these signals when its delete button is clicked. The modal does not know which card triggered it — it only knows it should show itself and call the callback when confirmed.

This is the **inversion of control** pattern: the card does not own the deletion flow. It sets up the signals, and the parent orchestrates the modal and the server action.

```rust
// In the exercises page (parent):
let show_delete = RwSignal::new(false);
let pending_delete_id = RwSignal::new(String::new());
let delete_action = ServerAction::<DeleteExercise>::new();

view! {
    {exercises.into_iter().map(|ex| view! {
        <ExerciseCard
            exercise=ex
            expanded_id=expanded_id
            editing_exercise=editing_exercise
            update_action=update_action
            pending_delete_id=pending_delete_id
            show_delete=show_delete
            is_coach=is_coach
            is_admin=is_admin
            current_user_id=current_user_id.clone()
        />
    }).collect_view()}

    <DeleteModal
        show=show_delete
        title="Delete this exercise?"
        subtitle="This will permanently remove the exercise."
        on_confirm=move || {
            delete_action.dispatch(DeleteExercise {
                id: pending_delete_id.get_untracked(),
            });
        }
    />
}
```

See the full `ExerciseCard` implementation in `src/pages/exercises/exercise_card.rs`. The component clones exercise fields at creation time, then clones them again into closures — the double-clone pattern applied systematically across a complex component.
</details>

---

## Summary

This chapter confronted Rust's ownership system in the context of UI components:

- **`RwSignal<T>` is `Copy`** — it can be shared across components and closures without cloning, making it the primary mechanism for parent-child communication
- **The clone-before-move pattern** is required for `String` and `Vec<T>` values captured by multiple closures
- **`impl Fn() + Copy + 'static`** is the standard callback prop signature, satisfied by closures that capture only `Copy` types
- **`<Portal>`** renders children at the document body level, escaping overflow clipping for dropdowns and modals
- **`web_sys::Element::get_bounding_client_rect()`** provides viewport measurements for smart positioning

You also saw information hiding in action: `DeleteModal` knows nothing about what it deletes, `SingleSelect` hides its positioning logic behind a three-prop interface, and the barrel file hides the internal file structure.

In the next chapter, you will build the profile and admin pages, introducing role-based access control with guard functions that leverage Rust's enum ordering for permission checks.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a doubly-linked list using Rust's arena pattern, then builds Undo/Redo on top of it.

**→ [Linked List Undo](../ds-narratives/ch11-linked-list-undo.md)**

The borrow checker keeps rejecting your code? This deep dive explains what lifetimes actually ARE, why `'a` exists, and the mental model that makes the borrow checker your ally instead of your enemy.

**→ [Lifetimes & the Borrow Checker — "The Gym Membership Card"](../ds-narratives/ch11-lifetimes-borrow-checker.md)**

---
