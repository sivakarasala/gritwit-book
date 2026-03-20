# Chapter 11: Reusable Components

Every page in GrindIt shares the same patterns: a confirmation modal before deleting, a dropdown for selecting categories, a multi-select for muscle groups, a card layout for exercises. So far, these exist as inline code within each page. This chapter extracts them into a shared `components/` module: `DeleteModal`, `SingleSelect`, `MultiSelect`, and `ExerciseCard`. Each component exposes a clean prop interface, manages its own internal state with signals, and communicates with its parent through callbacks.

The spotlight concept is **ownership and borrowing** --- the mechanism at the heart of Rust's memory safety. You will see why closures in Leptos need `move`, when to `clone` a signal before moving it, the difference between `Copy` and `Clone`, and how `RwSignal<T>` works as shared mutable state across parent and child components. This is where the borrow checker becomes a collaborator rather than an obstacle.

By the end of this chapter, you will have:

- A `DeleteModal` component with `show: RwSignal<bool>` and `on_confirm: impl Fn() + Copy + 'static` props
- A `SingleSelect` with smart dropdown positioning that flips up or down based on viewport space, using `web_sys::Element` and `DomRect`
- A `MultiSelect` with chip display, "clear all" button, and search filtering
- A refactored `ExerciseCard` that takes all its state as props
- A `components/mod.rs` barrel file re-exporting all components

---

## Spotlight: Ownership & Borrowing Deep Dive

Ownership and borrowing are the concepts that make Rust unique among programming languages. Every other language either uses garbage collection (JavaScript, Python, Go) or lets you manage memory manually (C, C++). Rust takes a third path: the compiler tracks who owns each piece of data and enforces rules at compile time. No garbage collector, no manual memory management, no memory bugs.

Let us start with the core concepts.

> **Programming Concept: What is Ownership?**
>
> In Rust, every piece of data has exactly one owner. Think of a car --- you can lend it to a friend, but there is always one person on the title.
>
> ```rust
> let name = String::from("Alice");  // 'name' owns this string
> let name2 = name;                   // ownership MOVES to 'name2'
> // println!("{}", name);            // ERROR! 'name' no longer owns anything
> println!("{}", name2);              // OK --- 'name2' is the owner now
> ```
>
> When `name2 = name` happens, the string moves from `name` to `name2`. After the move, `name` is empty --- you cannot use it. This prevents two variables from trying to free the same memory.
>
> In JavaScript, `let name2 = name` creates a second reference to the same object, and the garbage collector figures out when to clean up. In Rust, there is always exactly one owner, and when that owner goes out of scope, the data is freed immediately.

> **Programming Concept: What is Borrowing?**
>
> Borrowing means temporarily using data without taking ownership. Think of borrowing a library book --- you use it and return it. The library still owns it.
>
> ```rust
> let name = String::from("Alice");
> let len = calculate_length(&name);  // borrow 'name' (the & means "borrow")
> println!("{} is {} chars", name, len); // name is still valid!
>
> fn calculate_length(s: &String) -> usize {
>     s.len()  // we can read 's' but not modify it
> }
> ```
>
> The `&` symbol means "borrow, don't take." The function receives a reference (a temporary loan) and gives it back when done. The original owner keeps their data.
>
> There are two kinds of borrows:
> - **`&T`** (shared/immutable borrow) --- you can read but not change. Multiple borrows allowed at the same time.
> - **`&mut T`** (exclusive/mutable borrow) --- you can read and change. Only one at a time.
>
> Rust's fundamental rule: **at any given time, a value has either one mutable reference OR any number of immutable references, but not both.** This prevents data races at compile time.

> **Programming Concept: What is Clone?**
>
> Clone means making a complete copy of data. Think of photocopying a document --- now there are two independent copies. Changing one does not affect the other.
>
> ```rust
> let name = String::from("Alice");
> let name2 = name.clone();  // make a full copy
> println!("{}", name);       // OK! original is still valid
> println!("{}", name2);      // the copy is also valid
> ```
>
> Clone is explicit in Rust --- you have to call `.clone()` yourself. This is a design choice: copying data has a cost (memory and time), so Rust makes you opt in rather than doing it silently behind your back.
>
> Some types are so small and cheap to copy that Rust copies them automatically. These types implement the `Copy` trait:
> - Numbers (`i32`, `f64`, `bool`) --- just a few bytes
> - References (`&T`) --- just a pointer
> - `RwSignal<T>` in Leptos --- it is just a tiny ID, not the data itself
>
> Types that are too expensive to copy automatically must use `Clone`:
> - `String` --- could be any length, needs heap allocation
> - `Vec<T>` --- could contain millions of items

> **Programming Concept: What is a Portal?**
>
> A Portal renders a component somewhere else in the DOM tree. Think of a teleporter for UI elements.
>
> Normally, a dropdown menu would be rendered inside its parent container. But if that container has `overflow: hidden` (which clips content that extends beyond its borders), the dropdown gets cut off. A Portal teleports the dropdown to the top level of the page (`document.body`), where nothing can clip it.
>
> ```rust
> // Without Portal: dropdown gets clipped by parent's overflow:hidden
> <div class="card" style="overflow: hidden;">
>     <Dropdown/>  // gets cut off!
> </div>
>
> // With Portal: dropdown renders at document.body level
> <div class="card" style="overflow: hidden;">
>     <Portal>
>         <Dropdown/>  // renders at body level, never clipped
>     </Portal>
> </div>
> ```
>
> Leptos provides `<Portal>` as a built-in component. You will use it for the `SingleSelect` dropdown.

### How ownership works in Leptos closures

In Leptos, every event handler and reactive computation is a closure --- a function that captures variables from its surrounding scope. Rust needs to know: does the closure *borrow* or *own* the captured variables?

In JavaScript, closures always capture by reference. There is no decision to make:

```javascript
// JavaScript --- closures just "see" outer variables
let count = 0;
const increment = () => { count += 1; };
const display = () => { console.log(count); };
```

In Rust, closures must be explicit. The `move` keyword says "this closure takes ownership of everything it captures":

```rust
let show = RwSignal::new(false);

// This closure MOVES 'show' into itself
let on_click = move |_| {
    show.set(true);  // 'show' is now owned by this closure
};
```

But wait --- if `on_click` takes ownership of `show`, how can another closure also use `show`? This is where the `Copy` trait saves us.

### Copy vs Clone: the practical difference

| Type | Copy? | Clone? | Why? |
|------|-------|--------|------|
| `i32`, `f64`, `bool` | Yes | Yes | Small, fixed size --- just duplicate the bits |
| `RwSignal<T>` | Yes | Yes | Just a tiny ID number, not the actual data |
| `String` | No | Yes | Lives on the heap, could be any length |
| `Vec<T>` | No | Yes | Lives on the heap, could be huge |
| `&str` | Yes | Yes | Just a pointer + length, no ownership |

The rule for Leptos: **signals are `Copy` and go anywhere freely.** Strings and vectors are not `Copy` and need the clone-before-move pattern.

Here is the pattern in action:

```rust
let show = RwSignal::new(false);  // Copy type
let name = String::from("Alice"); // Clone type (not Copy)

// Signal: just move it into multiple closures --- Copy handles it
let f1 = move || show.get();    // show is copied into f1
let f2 = move || show.set(true); // show is copied into f2 (same signal!)

// String: must clone before moving into a second closure
let name_for_f3 = name.clone();  // make a copy first
let f3 = move || println!("{}", name);           // name moves into f3
let f4 = move || println!("{}", name_for_f3);    // the clone moves into f4
```

Without the clone on line 6, the compiler would say: "error: use of moved value `name`." The `move` on f3 took ownership, so f4 cannot also take ownership of the same `name`. Cloning creates a second, independent copy.

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

Let us decode `impl Fn() + Copy + 'static`:

- **`impl Fn()`** --- the prop is a callable function that takes no arguments and returns nothing. `Fn` (not `FnMut` or `FnOnce`) means it can be called multiple times without changing its captured state.
- **`+ Copy`** --- the closure must be copyable, so it can be used by multiple inner closures (both the cancel and confirm buttons, for example). This is satisfied when the closure only captures `Copy` types --- like signals.
- **`+ 'static`** --- the closure must own all its data. No borrowed references that could become invalid. `move` closures that capture only `Copy` or owned types always satisfy this.

If your callback needs to capture a `String`, it will not be `Copy`. In that case, use `impl Fn() + Clone + 'static` instead, and clone the callback before passing it to inner closures.

---

## Building the DeleteModal

The `DeleteModal` is the simplest reusable component --- a confirmation overlay for destructive actions. It knows nothing about *what* it is deleting. It only knows how to show itself, display a message, and call a callback when confirmed.

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

Let us count how many closures capture `show` in this component: the overlay's style, the overlay's on:click, the dialog's on:click, the cancel button's on:click, and the confirm button's on:click. That is five closures, all using the same `show` signal. This works because `RwSignal` is `Copy` --- each closure gets its own copy of the signal's ID, but they all point to the same underlying data.

The `on_confirm` callback is also used inside a closure. It works because we declared it as `Copy`. If it were not `Copy`, the compiler would reject this code.

### Why `ev.stop_propagation()`?

The overlay (dark background) closes the modal when clicked. But the dialog (white box) sits inside the overlay. Without `ev.stop_propagation()`, clicking the "Cancel" button would trigger the dialog's click event, which would bubble up to the overlay's click event, which would also try to close the modal. The propagation stop says: "this click was handled here, do not pass it up to the parent."

Think of it like this: if you knock on someone's door, the person inside answers. You do not also need the entire building to respond. `stop_propagation()` says "the door answered, building can ignore it."

### Design Insight: Information Hiding

`DeleteModal` is a textbook example of information hiding. The parent component knows *what* to delete and *how* to delete it. `DeleteModal` only knows *how to ask for confirmation*. It does not import any database types, does not know about workout logs or exercises, and does not handle errors. This means:

- The same modal works for deleting workouts, exercises, users, and anything else
- Changes to the deletion logic never require changes to the modal
- The modal can be tested in isolation

The `on_confirm` callback is the interface contract: "when the user clicks Confirm, call this function." Everything else is hidden.

---

## Building the SingleSelect

The `SingleSelect` is a searchable dropdown that replaces the native `<select>` element. It opens a floating panel with search filtering and smart positioning --- the dropdown flips upward when there is not enough space below the trigger.

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

This looks complex, but the logic is straightforward. Let us walk through it:

1. **Get the trigger button's position** using `get_bounding_client_rect()`. This returns a rectangle with `top`, `bottom`, `left`, `right`, `width`, and `height`.
2. **Measure available space** above and below the button. Subtract 70px for the bottom navigation bar.
3. **Decide direction**: if there is less than 200px below and more space above, flip the dropdown upward.
4. **Position the dropdown** using the computed top/bottom/left/width values.

Key ownership observations:
- `trigger_ref` is a `NodeRef` --- it is `Copy` (a signal internally), so the closure captures it freely.
- `el.as_ref()` converts the Leptos element to `&web_sys::Element` --- a borrow, not a move. We only need a reference to read the bounding rect.
- `#[cfg(target_arch = "wasm32")]` ensures this code only compiles for the browser. During SSR, there is no DOM.
- All six position signals (`dropdown_top`, `dropdown_left`, etc.) are `RwSignal<f64>` --- `Copy`, so no ownership issues.

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

Why Portal? Without it, the dropdown would be clipped by any parent with `overflow: hidden` --- which is common in card layouts, modals, and scrollable containers. Portal renders the dropdown at the top level of the DOM, then positions it absolutely using the computed pixel values. The backdrop (an invisible full-screen div) closes the dropdown when clicked anywhere outside.

### The SelectOption type

Both `SingleSelect` and `MultiSelect` share a common option type:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}
```

Why separate `value` and `label`? A category dropdown might show "Olympic Lifting" (the label the user sees) but store "olympic" (the value in the database). This separation keeps the UI human-friendly and the data machine-friendly.

`PartialEq` enables comparison (`==` and `!=`). `Clone` enables the option to be duplicated when building the filtered list.

### Good API design: the principle of least astonishment

The `SingleSelect` API has three props:

```rust
#[component]
pub fn SingleSelect(
    options: Vec<SelectOption>,       // what to show
    selected: RwSignal<String>,       // two-way binding
    #[prop(default = "Select...")] placeholder: &'static str,  // sensible default
) -> impl IntoView
```

The `options` are passed by value (the component owns them). The `selected` signal is passed by copy (shared between parent and child --- when the user picks an option, the parent sees the change immediately). The `placeholder` has a default so callers can omit it.

Compare this to a bad API that leaks internal details:

```rust
// BAD: exposes implementation concerns to the caller
fn SingleSelect(
    options: Vec<SelectOption>,
    selected: RwSignal<String>,
    open: RwSignal<bool>,           // caller should not control this
    search_query: RwSignal<String>, // implementation detail
    dropdown_position: (f64, f64),  // layout detail
)
```

The caller does not care about whether the dropdown is open, what the search query is, or where the dropdown is positioned. Those are internal details that should be hidden inside the component.

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

Let us unpack `signal.update(|s| ...)`. Instead of replacing the entire vector with `signal.set(new_vec)`, `update` gives you a mutable reference (`&mut Vec<String>`) to the current value. You modify it directly --- no need to clone the whole vector, read it, change it, and set it back. This is more efficient for large collections.

The `s.retain(|x| x != &v)` call keeps only the elements that are *not* equal to `v` --- effectively removing `v` from the list.

### Chip display with clear-all

Selected values appear as removable chips below the trigger:

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

Notice the clone chain: `opt.value.clone()` creates `val` outside the closure, which is then cloned again as `v` inside the `on:click` closure. Why the double clone?

1. `opt.value.clone()` --- needed because `opt` is consumed by the iterator, but we need the value string to live in the closure
2. `val.clone()` inside the closure --- needed because the `on:click` closure implements `Fn` (can be called multiple times), so it cannot consume `val`. Each time the button is clicked, it clones `val` to get a fresh copy.

This is the clone-then-move pattern applied inside an iterator --- each chip gets its own closure, and each closure needs its own copy of the value string.

The "Clear all" button is simpler: `selected.set(vec![])` replaces the entire vector with an empty one. The `stop_propagation()` prevents the click from toggling the dropdown open/closed.

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

### Understanding the ownership check with Option::zip

The `can_delete` check uses a clever pattern worth understanding step by step:

```rust
let can_delete = is_admin
    || current_user_id
        .as_deref()                              // Option<String> -> Option<&str>
        .zip(exercise.created_by.as_deref())     // Option<(&str, &str)>
        .map(|(uid, owner)| uid == owner)        // Option<bool>
        .unwrap_or(false);                       // bool
```

Let us trace through this:

1. `as_deref()` converts `Option<String>` to `Option<&str>` --- borrowing the inner string without taking ownership. This avoids cloning.
2. `zip` combines two `Option`s into one. If the user ID is `Some("123")` and the creator is `Some("123")`, you get `Some(("123", "123"))`. If *either* is `None`, you get `None`.
3. `map` transforms the pair into a boolean: are the IDs equal?
4. `unwrap_or(false)` says: if either ID was missing (`None`), default to "no, cannot delete."

This reads as: "can delete if admin, OR if both user ID and creator ID exist and are equal." Clean, functional, no temporary variables.

### Conditional rendering based on role

The card conditionally shows edit/delete buttons based on the user's role:

```rust
{is_coach.then(|| view! {
    <div class="exercise-panel-actions">
        <button class="exercise-edit-btn"
            on:click=move |_| {
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

`bool::then(|| ...)` returns `Some(view)` if true, `None` if false. Leptos renders `None` as nothing --- no DOM node, no placeholder. This is the Rust equivalent of `{isCoach && <div>...</div>}` in React JSX.

Notice the nesting: `is_coach.then()` wraps both buttons, and `can_delete.then()` further gates the delete button. A coach can always edit, but can only delete exercises they created (unless they are also an admin).

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

This pattern hides the file structure. No one needs to know that `DeleteModal` lives in a file called `delete_modal.rs`. If you rename or reorganize internal files later, the import path stays stable.

---

## Rust Gym

### Copy vs Clone exercises

```rust
fn demonstrate_copy_vs_clone() {
    // Copy types: automatic duplication
    let signal = RwSignal::new(42);
    let s2 = signal;          // copy --- both signal and s2 are valid
    signal.set(100);          // original still works
    assert_eq!(s2.get(), 100); // s2 sees the change (same underlying data!)

    // Clone types: explicit duplication
    let name = String::from("Alice");
    let name2 = name.clone();  // explicit clone --- two independent strings
    // let name3 = name;       // this would MOVE name, making it unusable
    assert_eq!(name, "Alice"); // original still valid because we cloned, not moved
    assert_eq!(name2, "Alice");
}
```

The key insight: when you copy a signal, both copies point to the same underlying data. When you clone a string, the two copies are completely independent.

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
- `test_a`: **Compiles.** `RwSignal` is `Copy`, so both closures get their own copy. They both work.
- `test_b`: **Does NOT compile.** `String` is not `Copy`. The `move` on `f1` takes ownership of `name`, so `f2` cannot also take ownership. The compiler says "use of moved value."
- `test_c`: **Compiles.** We cloned `name` into `name2` before the closures. `f1` takes ownership of `name`, `f2` takes ownership of `name2`. Each closure has its own independent string.
</details>

### Borrow checker puzzles

<details>
<summary>Exercise: fix this component</summary>

This code does not compile. Can you fix it?

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

The problem: `items.iter()` borrows each item temporarily, but the `on:click` closure needs to own the data (closures in Leptos must be `'static` --- they cannot hold temporary borrows).

**Solution:**

```rust
#[component]
fn Fixed(items: Vec<String>) -> impl IntoView {
    view! {
        <ul>
            {items.into_iter().map(|item| {
                let display = item.clone();  // clone for the view text
                view! {
                    <li on:click=move |_| {
                        log!("clicked {}", item);  // item is owned by this closure
                    }>{display}</li>
                }
            }).collect_view()}
        </ul>
    }
}
```

Changes:
1. `into_iter()` instead of `iter()` --- this takes ownership of each `String` out of the vector, rather than borrowing
2. Clone `item` into `display` for the view text, because `item` will be moved into the closure
3. The `on:click` closure owns `item` directly --- no borrowing needed
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

Because `RwSignal` is `Copy`, passing it to both child components creates no ownership conflict. Both `Display` and `Controls` get their own copy of the signal ID, but they all point to the same underlying number. When `Controls` increments the count, `Display` sees the change immediately.

This is the foundation of parent-child communication in Leptos: share a signal by passing it as a prop.
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

Note that `on_confirm` is `Copy + 'static`, so it can be captured by the button's `on:click` closure without cloning. The `show` signal is also `Copy`. This component has zero `clone()` calls --- everything is either `Copy` or `&'static str`.
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

The core ownership pattern: each chip's remove button closure captures a cloned copy of the value string. The `selected.update()` call modifies the vector in place without cloning the entire vector. The double-clone (`opt.value.clone()` then `val.clone()`) is required because the `on:click` closure implements `Fn` (callable multiple times), not `FnOnce` (callable once).
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

The key architectural decision: `DeleteModal` is instantiated once in the parent page and controlled by shared signals (`show_delete`, `pending_delete_id`). Each `ExerciseCard` writes to these signals when its delete button is clicked. The modal does not know which card triggered it --- it only knows it should show itself and call the callback when confirmed.

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

See the full `ExerciseCard` implementation in `src/pages/exercises/exercise_card.rs`. The component clones exercise fields at creation time, then clones them again into closures --- the double-clone pattern applied systematically across a complex component.
</details>

---

## Summary

This chapter confronted Rust's ownership system in the context of UI components:

- **Ownership** means every value has exactly one owner. When a value moves into a closure, the original variable is gone.
- **Borrowing** means temporarily using data without taking ownership. The `&` symbol creates a borrow.
- **Clone** creates an independent copy. Use it when you need the same data in multiple closures.
- **Portals** teleport UI elements to the document body, escaping clipping containers.
- **`RwSignal<T>` is `Copy`** --- it can be shared across components and closures without cloning, making it the primary mechanism for parent-child communication
- **The clone-before-move pattern** is required for `String` and `Vec<T>` values captured by multiple closures
- **`impl Fn() + Copy + 'static`** is the standard callback prop signature, satisfied by closures that capture only `Copy` types
- **`<Portal>`** renders children at the document body level, escaping overflow clipping for dropdowns and modals

You also saw information hiding in action: `DeleteModal` knows nothing about what it deletes, `SingleSelect` hides its positioning logic behind a three-prop interface, and the barrel file hides the internal file structure.

In the next chapter, you will build the profile and admin pages, introducing role-based access control with guard functions that leverage Rust's enum ordering for permission checks.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a doubly-linked list using Rust's arena pattern, then builds Undo/Redo on top of it from scratch in Rust — no libraries, just std.

**→ [Linked List Undo](../ds-narratives/ch11-linked-list-undo.md)**

The borrow checker keeps rejecting your code? This deep dive explains what lifetimes actually ARE, why `'a` exists, and the mental model that makes the borrow checker your ally instead of your enemy.

**→ [Lifetimes & the Borrow Checker — "The Gym Membership Card"](../ds-narratives/ch11-lifetimes-borrow-checker.md)**

---
