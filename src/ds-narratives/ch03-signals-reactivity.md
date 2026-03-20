# Leptos Signals: The Reactive Engine

## "The Gym Announcement System"

You type "back squat" into the search bar and the exercise list updates instantly. No event listeners, no manual DOM patching, no `setState`. You just wrote `let search = RwSignal::new(String::new())` and everything... reacted. How? Inside that innocent `RwSignal` is a dependency graph -- a web of observers watching for changes. When you call `search.set(...)`, a cascade fires through the graph, but ONLY to the parts of your UI that actually read that signal. Let us build a reactive signal system from scratch and see why Leptos feels like magic.

---

## 1. The Observer Pattern -- Reactivity's Foundation

Before we touch Leptos, let us understand the pattern underneath it all: **publish/subscribe**.

Think of a gym's front desk. They have a microphone and an announcement system. Different areas of the gym -- the squat racks, the cardio section, the stretching corner -- each have their own speaker. When the front desk announces "squat rack 3 is free!", only the people who signed up to hear squat rack announcements get notified. The people on the treadmills do not hear it. They did not subscribe.

That is the observer pattern: a **publisher** (the signal) holds a list of **subscribers** (closures that care about the value). When the publisher's value changes, it iterates through its subscriber list and calls each one.

In its simplest form:

```rust
let mut subscribers: Vec<Box<dyn Fn()>> = Vec::new();

// "Sign me up for squat rack announcements"
subscribers.push(Box::new(|| println!("Update the exercise list!")));
subscribers.push(Box::new(|| println!("Update the result count!")));

// Front desk makes an announcement
for notify in &subscribers {
    notify();
}
```

This is the entire idea. Everything that follows -- signals, memos, effects, dependency graphs -- is a more sophisticated version of this loop. The question is: how do we make the subscription automatic?

---

## 2. Build Signal\<T\> from Scratch

Let us build a working reactive signal using only the standard library. This is the core of the chapter.

```rust
use std::cell::RefCell;
use std::rc::Rc;

struct Signal<T> {
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            value: Rc::clone(&self.value),
            subscribers: Rc::clone(&self.subscribers),
        }
    }
}

impl<T> Signal<T> {
    fn new(value: T) -> Self {
        Signal {
            value: Rc::new(RefCell::new(value)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.borrow().clone()
    }

    fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value;
        self.notify();
    }

    fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut self.value.borrow_mut());
        self.notify();
    }

    fn subscribe(&self, callback: impl Fn() + 'static) {
        self.subscribers.borrow_mut().push(Box::new(callback));
    }

    fn notify(&self) {
        for callback in self.subscribers.borrow().iter() {
            callback();
        }
    }
}
```

Now let us use it with GrindIt data. Imagine you have a list of exercises and a search query signal:

```rust
fn main() {
    let exercises = vec![
        "Back Squat", "Front Squat", "Overhead Squat",
        "Deadlift", "Bench Press", "Pull-Up",
        "Box Jump", "Burpee", "Row",
    ];

    let search = Signal::new(String::new());

    // Subscribe: when search changes, filter and print matching exercises
    let search_for_filter = search.clone();
    let exercises_clone = exercises.clone();
    search.subscribe(move || {
        let query = search_for_filter.get().to_lowercase();
        let filtered: Vec<&&str> = exercises_clone
            .iter()
            .filter(|e| e.to_lowercase().contains(&query))
            .collect();
        println!("Matching exercises: {:?}", filtered);
    });

    // Subscribe: update the result count display
    let search_for_count = search.clone();
    let exercises_clone2 = exercises.clone();
    search.subscribe(move || {
        let query = search_for_count.get().to_lowercase();
        let count = exercises_clone2
            .iter()
            .filter(|e| e.to_lowercase().contains(&query))
            .count();
        println!("Found {} exercises", count);
    });

    // User types "squat"
    println!("--- User types 'squat' ---");
    search.set("squat".to_string());

    // User types "dead"
    println!("\n--- User types 'dead' ---");
    search.set("dead".to_string());
}
```

Output:

```text
--- User types 'squat' ---
Matching exercises: ["Back Squat", "Front Squat", "Overhead Squat"]
Found 3 exercises

--- User types 'dead' ---
Matching exercises: ["Deadlift"]
Found 1 exercises
```

The front desk (our `Signal`) made an announcement (`set`), and only the registered listeners (our two `subscribe` closures) reacted. The cardio section -- anything that did not subscribe -- was completely unaware. Zero wasted work.

---

## 3. Derived/Computed Signals -- Automatic Memoization

Our `Signal` is useful, but it has a problem: subscribers are imperative callbacks. What if we want a **value** that automatically stays in sync with other signals? That is a derived signal, or **Memo**.

A `Memo` wraps a computation. It caches the result and only recomputes when its source signals change.

```rust
struct Memo<T> {
    compute: Rc<dyn Fn() -> T>,
    value: Rc<RefCell<T>>,
}

impl<T: Clone> Memo<T> {
    fn new(compute: impl Fn() -> T + 'static) -> Self {
        let initial = compute();
        Memo {
            compute: Rc::new(compute),
            value: Rc::new(RefCell::new(initial)),
        }
    }

    fn get(&self) -> T {
        self.value.borrow().clone()
    }

    /// Recompute and cache the new value
    fn recompute(&self) {
        let new_val = (self.compute)();
        *self.value.borrow_mut() = new_val;
    }
}
```

Now wire it into the signal system. When the source signal changes, the memo recomputes:

```rust
fn main() {
    let exercises: Vec<String> = vec![
        "Back Squat", "Front Squat", "Overhead Squat",
        "Deadlift", "Bench Press", "Pull-Up",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let search = Signal::new(String::new());

    // Create a derived signal: filtered exercise count
    let search_for_memo = search.clone();
    let exercises_for_memo = exercises.clone();
    let filtered_count = Rc::new(Memo::new(move || {
        let query = search_for_memo.get().to_lowercase();
        exercises_for_memo
            .iter()
            .filter(|e| e.to_lowercase().contains(&query))
            .count()
    }));

    // When search changes, recompute the memo
    let memo_ref = Rc::clone(&filtered_count);
    search.subscribe(move || {
        memo_ref.recompute();
        println!("Filtered count: {}", memo_ref.get());
    });

    search.set("squat".to_string()); // prints: Filtered count: 3
    search.set("press".to_string());  // prints: Filtered count: 1
}
```

The key insight: a `Memo` is a **cached value that derives from signals**. It does not recompute on every `.get()` -- only when its dependencies change. In the Leptos version, this dependency tracking happens automatically. The framework runs your closure, observes which signals it calls `.get()` on, and subscribes to exactly those signals. No manual wiring needed.

**Lazy vs. eager evaluation.** Our hand-built Memo recomputes eagerly (immediately on change). Leptos memos are lazy: they mark themselves dirty when a dependency changes, but only recompute when someone calls `.get()`. This avoids wasted work if a memo's value is never read between two updates.

---

## 4. The Dependency Graph -- How Leptos Tracks Who Reads What

This is the "aha moment." In Leptos, you never call `.subscribe()`. You just write closures that call `.get()` on signals, and the framework figures out the rest. How?

When a reactive closure runs (an Effect, a Memo computation, a component render), Leptos sets a **thread-local "currently tracking" context**. Every time a signal's `.get()` is called, it checks this context: "Is someone tracking me right now?" If so, the signal registers that closure as a subscriber. When `.set()` is called later, the signal notifies exactly those subscribers.

For our GrindIt search page, the dependency graph looks like this:

```text
search_query (Signal<String>)
    |
    |---> filtered_exercises (Memo)
    |        |---> exercise_list (DOM effect)
    |        \---> result_count (DOM effect)
    \---> search_highlight (DOM effect)

category_filter (Signal<Option<String>>)
    \---> filtered_exercises (Memo)
```

Now trace what happens when the user types "squat" into the search bar:

1. `search_query.set("squat")` fires
2. Leptos checks: who subscribed to `search_query`? Two subscribers: `filtered_exercises` and `search_highlight`
3. `filtered_exercises` recomputes -- it re-runs its closure, which calls `search_query.get()` and `category_filter.get()`, producing a new filtered list
4. `filtered_exercises` changed, so Leptos checks its subscribers: `exercise_list` and `result_count`
5. Those two DOM effects re-run, updating the visible cards and the "3 results" counter
6. `search_highlight` re-runs, bolding the matching text in each card

What did NOT happen? `category_filter` was not touched. Any part of the page that only depends on `category_filter` (say, a filter dropdown's selected state) remained completely inert. No diffing. No reconciliation. No wasted work.

This is the dependency graph in action: a directed acyclic graph where signals are sources, memos are intermediate nodes, and effects are leaves.

---

## 5. Fine-Grained Reactivity vs Virtual DOM

If you are coming from React, here is the shift in mental model:

| | React (Virtual DOM) | Leptos (Fine-grained signals) |
|--|---------------------|-------------------------------|
| On state change | Re-render entire component tree, diff, patch | Only update the exact DOM nodes that depend on the changed signal |
| Granularity | Component-level | Node-level |
| Overhead | Diffing cost proportional to tree size | Zero diffing -- direct updates |
| Mental model | "Re-render everything, framework figures out what changed" | "I know exactly what changed and who cares" |

The gym analogy makes this concrete:

- **React's approach:** Someone changed something at the front desk. An employee now walks through the ENTIRE gym, checks every single person, and asks "Hey, did this affect you? No? Okay, moving on." That is virtual DOM diffing. Even with optimizations (memoization, `shouldComponentUpdate`), the walk still happens.

- **Leptos's approach:** The front desk has a targeted speaker system. "Squat rack 3 is free" goes ONLY to the speakers in the squat area. The people on treadmills never hear it. There is no walk. There is no check. The announcement goes directly to the people who care.

Why does this matter for GrindIt? Imagine the exercise library has grown to 500 exercises. The user types a letter in the search bar.

- **React:** Re-renders the entire exercise list component. Creates 500 virtual DOM nodes. Diffs them against the previous 500. Discovers that 480 are unchanged. Patches the 20 that changed. That diffing step is O(n) in the number of exercises.

- **Leptos:** The search signal fires. The `filtered_exercises` memo recomputes (one pass through the list). Only the DOM nodes for exercises whose visibility actually changed get updated. If typing "b" hid 20 exercises, exactly 20 DOM nodes are removed. The other 480 are never touched -- not re-rendered, not diffed, not even looked at.

This is not a theoretical advantage. It is the reason Leptos consistently benchmarks near the top of the JS Framework Benchmark alongside vanilla JavaScript.

---

## 6. Why RwSignal Uses Arc + RwLock

Our hand-built `Signal<T>` used `Rc<RefCell<T>>`. That is fine for single-threaded contexts. But Leptos has a challenge: the same code runs in two very different environments.

**On the server (SSR):** Your Leptos app runs inside an Axum handler. Axum is multi-threaded -- multiple HTTP requests are handled concurrently. If signals used `Rc<RefCell<T>>`, they could not be sent across threads. The compiler would refuse with "`Rc<RefCell<T>>` cannot be sent between threads safely."

**In the browser (WASM):** JavaScript is single-threaded. `Rc<RefCell<T>>` would work fine here. But Leptos cannot have two different implementations.

The solution: Leptos uses `Send + Sync` bounds on signal values, and internally uses thread-safe primitives (`Arc`, `RwLock` or similar) so the same code compiles for both targets.

This has a practical consequence you have already encountered: when you move a signal into a closure, you sometimes need to `.clone()` first. But you are cloning the **handle** (an `Arc` pointer), not the data inside. Cloning a signal is cheap -- it increments a reference count, not a deep copy.

```rust,ignore
let search = RwSignal::new(String::new());

// This works because RwSignal is Copy in Leptos 0.7+
let on_input = move |ev| {
    search.set(event_target_value(&ev));
};
```

Wait -- `Copy`? Yes. In Leptos 0.7 and later, signals implement `Copy`. They are not storing data inline. A signal is just an **ID** -- an index into a global arena that holds the actual values. Copying an ID is free. This is why you can use signals in multiple closures without `.clone()`:

```rust,ignore
// Both closures capture `search` by copy -- no clone needed
let on_input = move |ev| { search.set(event_target_value(&ev)); };
let on_clear = move |_| { search.set(String::new()); };
```

The data lives in the arena. The signal is just a lightweight key to look it up. Think of it like a gym membership card number -- copying the card number does not duplicate the member.

---

## 7. Common Signal Patterns in GrindIt

Now that you understand how signals work under the hood, here are the patterns you will use throughout the app.

### a) Signal of Vec -- the exercise list

```rust,ignore
let exercises = RwSignal::new(Vec::<Exercise>::new());

// Add an exercise without cloning the entire vec
exercises.update(|list| list.push(new_exercise));

// Remove by index
exercises.update(|list| { list.remove(idx); });
```

Use `.update()` instead of `.set()` when you want to modify in place. With `.set()`, you would need to `.get()` the whole vec, modify it, and `.set()` it back -- three operations and a full clone. `.update()` gives you a mutable reference directly.

### b) Signal of Option -- selected exercise

```rust,ignore
let selected = RwSignal::new(Option::<String>::new());

// Toggle: click again to deselect
selected.update(|s| {
    *s = if *s == Some(id.clone()) { None } else { Some(id.clone()) };
});
```

This pattern powers expandable cards. When the user taps an exercise, its ID goes into `selected`. Tapping again sets it back to `None`. In the template, each card checks `selected.get() == Some(my_id)` to decide whether to show details.

### c) Signal lifting -- individual signals vs struct signal

```rust,ignore
// Instead of one big signal (any field change rerenders everything):
// let form = RwSignal::new(FormState { name, category, weight });

// Lift each field into its own signal:
let name = RwSignal::new(String::new());
let category = RwSignal::new(String::new());
let weight = RwSignal::new(0.0f64);

// Now changing `name` does not re-render the weight input
```

This is signal lifting. By splitting a struct into individual signals, you get finer-grained reactivity. The name input and the weight input are now independent. Typing in the name field does not cause the weight field to re-render. For forms with many fields, this matters.

### d) Effect -- side effects when signals change

```rust,ignore
Effect::new(move || {
    let query = search.get();
    log!("User searched for: {}", query);
    // Could also: update the URL, send analytics, sync to localStorage
});
```

An `Effect` is a leaf in the dependency graph. It reads signals, which makes it a subscriber, but it does not produce a value. It is for side effects: logging, network requests, DOM manipulation outside of Leptos's control.

---

## 8. Pitfalls and Anti-Patterns

**Reading a signal outside a reactive context.** If you call `.get()` in a plain function (not inside an Effect, Memo, or component body), it returns the value but does not track the dependency. The signal has no idea you read it, so it will never notify you of changes. The value works once but never updates.

**Creating signals in loops.** Do not create a new `RwSignal` inside a `.map()` on every render. Each render would create fresh signals, losing all previous state. Use Leptos's `<For>` component, which manages a keyed list of items and their associated reactive state.

**Holding `.get()` across an `.await`.** Signals give you a snapshot. If you `.get()` a value, then `await` some async work, the signal may have changed by the time the future resumes. You are working with stale data. Get the value, start the async work, and re-get if you need the latest value after the await.

**Over-nesting derived signals.** A memo that reads a memo that reads a memo creates a chain. Each link adds latency and complexity. If you find yourself three or four memos deep, consider whether a single memo reading the original signals directly would be clearer and faster.

---

## 9. Mental Model Summary

| Concept | What it is | GrindIt example |
|---------|-----------|-----------------|
| Signal | Reactive container for a value | `search_query`, `exercises`, `selected_tab` |
| Memo | Cached computation derived from signals | `filtered_exercises`, `category_counts` |
| Effect | Side effect triggered by signal changes | Analytics logging, URL sync |
| Dependency tracking | Auto-detect which signals a closure reads | Search bar -> filter -> card list |
| Fine-grained | Only affected DOM nodes update | Typing updates list count, not the header |
| Signal graph | Directed acyclic graph of dependencies | The entire reactive page |

The gym announcement system, one last time: signals are the front desk microphone. Memos are area managers who listen to the front desk and relay relevant information to their section. Effects are the gym members who hear the announcement and act on it. The wiring between them -- the speaker system -- is the dependency graph. And the beauty of Leptos is that you never install the speakers yourself. You just talk into the microphone and listen where you care. The framework wires the rest.

---

## 10. Try It Yourself

### Exercise 1: Automatic Dependency Tracking

Extend the `Signal<T>` and `Memo<T>` from sections 2 and 3 so that `Memo` **automatically** tracks its dependencies. Use a thread-local `RefCell<Option<Vec<...>>>` as a "currently tracking" context. When a `Memo` is computing, it sets this context. When `Signal::get()` is called, it checks the context and registers itself. This is, in essence, how Leptos works.

### Exercise 2: Draw the Workout Logging Graph

GrindIt's workout logging page has these signals:
- `wod` -- the workout of the day (contains sections, each with movements)
- `selected_section` -- which section the user is scoring
- `scores` -- a map of movement ID to the user's input
- `is_valid` -- derived: are all required scores filled in?

Draw the dependency graph. Which signals does `is_valid` depend on? If the user changes a score for one movement, what recomputes?

### Exercise 3: Signal Lifting Refactor

Take this single-signal form:

```rust,ignore
struct ExerciseForm {
    name: String,
    category: String,
    description: String,
    scoring_type: String,
}

let form = RwSignal::new(ExerciseForm { /* ... */ });
```

Refactor it into four individual signals. Update the template so that typing in the name field does not cause the category dropdown to re-render. Verify by adding an `Effect` on each signal that logs when it fires -- you should see only the relevant effect trigger.
