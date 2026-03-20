# Lifetimes & the Borrow Checker -- "The Gym Membership Card"

The compiler just yelled at you: `lifetime may not live long enough`. You stare at the error. You add `'a`. It yells louder. You add `'static`. Now a different error. You throw `clone()` at it and it compiles, but you feel dirty. Sound familiar? Lifetimes are the #1 source of frustration for Rust learners -- but they are actually the compiler doing something extraordinary: proving at compile time that your program will never have a dangling pointer, use-after-free, or data race. Let's understand what the borrow checker actually sees when it reads your code.

Here is the analogy we will use throughout: **a lifetime is how long your gym membership lasts.** You can borrow equipment (references) only while your membership is active. If your membership expires (the value is dropped) while someone else is still using your barbell (holding a reference), that is a dangling reference -- and the gym receptionist (the borrow checker) will not let it happen.

---

## 1. What the Borrow Checker Actually Checks

The borrow checker enforces three rules. Every single lifetime error you have ever seen traces back to one of these.

### Rule 1: Every reference has a lifetime

Every time you write `&exercise`, you are creating a reference that is only valid for as long as `exercise` exists. Think of it as swiping your membership card -- the reference is stamped with an expiration date.

```rust
fn rule_one_violation() {
    let exercise_ref;
    {
        let exercise = String::from("Back Squat");
        exercise_ref = &exercise;
        // exercise is dropped here -- membership expired!
    }
    // ERROR: exercise_ref points to freed memory
    // println!("{}", exercise_ref);
}
```

The fix: make sure the data lives long enough.

```rust
fn rule_one_fix() {
    let exercise = String::from("Back Squat");
    let exercise_ref = &exercise;
    println!("{}", exercise_ref); // exercise is still alive -- membership valid
}
```

### Rule 2: One mutable reference OR any number of shared references

You can have many people *watching* you lift (shared `&T` references), or one person *adjusting the rack* (exclusive `&mut T` reference), but never both at the same time. If someone is re-racking the plates, everyone else needs to step away.

```rust
fn rule_two_violation() {
    let mut exercises = vec!["Deadlift", "Squat"];
    let first = &exercises[0];     // shared borrow
    exercises.push("Bench Press"); // mutable borrow -- ERROR!
    // println!("{}", first);      // shared borrow used after mutation
}
```

The fix: finish reading before you write.

```rust
fn rule_two_fix() {
    let mut exercises = vec!["Deadlift", "Squat"];
    let first = exercises[0]; // Copy the &str out (no borrow held)
    exercises.push("Bench Press");
    println!("{}", first); // fine -- first is an owned copy
}
```

### Rule 3: References must not outlive the data they point to

Your membership card cannot outlast the gym itself. If the gym closes (the owner is dropped), every card (reference) becomes invalid.

```rust
fn rule_three_violation() -> &'static str {
    // This is fine -- string literals live forever
    "Overhead Press"
}

// This would NOT compile:
// fn bad_ref() -> &String {
//     let s = String::from("Snatch");
//     &s // ERROR: returns a reference to data owned by the function
// }
```

The fix: return owned data instead.

```rust
fn rule_three_fix() -> String {
    let s = String::from("Snatch");
    s // move ownership to the caller
}
```

---

## 2. Lifetimes Are Already There -- You Just Don't See Them

Every `&T` in your code is actually `&'a T` -- the compiler infers the lifetime through a process called *lifetime elision*. Three rules handle roughly 90% of cases so you never need to write `'a` yourself:

1. **Each input reference gets its own lifetime.** `fn f(a: &str, b: &str)` becomes `fn f<'a, 'b>(a: &'a str, b: &'b str)`.
2. **If there is exactly one input lifetime, it is assigned to all outputs.** `fn f(a: &str) -> &str` becomes `fn f<'a>(a: &'a str) -> &'a str`.
3. **If one input is `&self` or `&mut self`, that lifetime is assigned to all outputs.** Methods almost always work without annotations.

Here is what elision looks like in practice:

```rust
struct Exercise {
    name: String,
    category: String,
    weight: f64,
}

impl Exercise {
    // What you write:
    fn name(&self) -> &str {
        &self.name
    }

    // What the compiler sees:
    // fn name<'a>(&'a self) -> &'a str {
    //     &self.name
    // }
}
```

The receptionist knows your face -- no card needed. But when there is ambiguity, you have to show your card explicitly.

---

## 3. When You NEED Explicit Lifetimes

The classic case: two input references, one output reference. Which input does the output borrow from? The compiler cannot guess.

```rust
struct Exercise {
    name: String,
    weight: f64,
}

fn pick_heavier<'a>(a: &'a Exercise, b: &'a Exercise) -> &'a Exercise {
    if a.weight > b.weight { a } else { b }
}
```

By writing `'a` on both inputs and the output, you are telling the compiler: "the returned reference is valid for as long as *both* inputs are valid." The compiler will then enforce that at every call site.

Why does it need help? Imagine if it didn't check:

```rust
# struct Exercise { name: String, weight: f64 }
# fn pick_heavier<'a>(a: &'a Exercise, b: &'a Exercise) -> &'a Exercise {
#     if a.weight > b.weight { a } else { b }
# }
fn main() {
    let heavy;
    let squat = Exercise { name: "Squat".into(), weight: 150.0 };
    {
        let curl = Exercise { name: "Curl".into(), weight: 20.0 };
        heavy = pick_heavier(&squat, &curl);
        // curl is about to be dropped...
        // If heavy points to curl, we'd have a dangling reference!
        println!("Heavier in inner scope: {}", heavy.name);
    }
    // Without the borrow checker, heavy might be a dangling pointer here.
    // The compiler prevents this by requiring both inputs share lifetime 'a.
}
```

A GrindIt example with two slices:

```rust
struct Workout {
    name: String,
    score: u32,
}

fn latest_workout<'a>(history: &'a [Workout], backup: &'a [Workout]) -> &'a Workout {
    let h = history.last();
    let b = backup.last();
    match (h, b) {
        (Some(hw), Some(bw)) => if hw.score >= bw.score { hw } else { bw },
        (Some(hw), None) => hw,
        (None, Some(bw)) => bw,
        (None, None) => panic!("No workouts found"),
    }
}
```

The lifetime `'a` ties the return value to both slices. The gym receptionist will not let you use the returned reference after either slice is dropped.

---

## 4. Lifetimes in Structs -- "Borrowing Data You Don't Own"

When a struct holds references, it *borrows* data from somewhere else. It cannot outlive that data -- just like a temporary gym pass cannot outlive the person who lent it to you.

```rust
struct ExerciseView<'a> {
    name: &'a str,
    category: &'a str,
}

fn display_exercise() {
    let name = String::from("Back Squat");
    let category = String::from("Weightlifting");

    let view = ExerciseView {
        name: &name,
        category: &category,
    };

    println!("{}: {}", view.name, view.category);
    // view, name, and category all dropped here -- fine!
}
```

If `ExerciseView` tries to outlive the data it borrows, the compiler stops you:

```rust
struct ExerciseView<'a> {
    name: &'a str,
    category: &'a str,
}

fn bad_view() -> ExerciseView<'static> {
    let name = String::from("Back Squat");
    // ERROR: cannot return ExerciseView that borrows local data
    // ExerciseView { name: &name, category: "Weightlifting" }
    //
    // Fix: use 'static data or return owned types
    ExerciseView { name: "Back Squat", category: "Weightlifting" }
}
```

**GrindIt rule of thumb:** use owned types (`String`, `Vec<Exercise>`) in database models and structs that live long. Use borrowed types (`&str`, `&[Exercise]`) in short-lived view or display structs where you are just reading data that someone else owns.

---

## 5. The `'static` Lifetime -- "Lifetime of the Entire Program"

`'static` is the longest possible lifetime. It means one of two things:

1. **The data lives for the entire program.** String literals are `&'static str` because they are baked into the binary.
2. **The type owns everything it contains (no borrows).** `String` satisfies `'static` because it owns its heap data -- no references that could dangle.

This distinction matters enormously. When you see `T: 'static`, it does *not* mean "must be a static variable." It means "must not contain any non-static references." `String`, `Vec<u32>`, and `Exercise` (with owned fields) are all `'static`.

This is why Leptos closures need `move` and `'static` bounds:

```rust,ignore
// Leptos component callback -- must be 'static because the closure
// might be called long after the component function returns.
// The component function is like a gym orientation session that ends,
// but the callback is a membership card that must remain valid.
let count = RwSignal::new(0);
let on_click = move || {
    // 'move' transfers ownership of 'count' into the closure.
    // No dangling refs -- the closure owns everything it needs.
    count.set(count.get() + 1);
};
```

If the closure *borrowed* `count` instead of moving it, the borrow would dangle as soon as the component function returned. The `move` keyword gives the closure its own gym membership instead of borrowing someone else's.

---

## 6. Visualizing Lifetimes -- NLL (Non-Lexical Lifetimes)

Lifetimes have scopes, and you can draw them:

```rust
fn nll_example() {
    let exercise = String::from("Back Squat");  // --+-- 'exercise starts
    let name_ref = &exercise;                    //   |--+-- 'name_ref starts
    println!("{}", name_ref);                    //   |--+-- 'name_ref ENDS (last use)
    drop(exercise);                              // --+-- 'exercise ends (OK!)
}
```

Before Rust 2018, lifetimes lasted until the end of their *lexical scope* (the closing brace). This meant code like the above would fail -- `name_ref` would be considered alive until the end of the function, conflicting with the `drop`. Non-Lexical Lifetimes (NLL) changed this: a lifetime now ends at its *last use*.

Here is a case that used to fail but compiles today thanks to NLL:

```rust
fn nll_wins() {
    let mut exercises = vec!["Squat", "Bench"];
    let first = &exercises[0];        // immutable borrow starts
    println!("First: {}", first);     // immutable borrow ENDS here (last use)
    exercises.push("Deadlift");       // mutable borrow -- OK! no conflict
    println!("All: {:?}", exercises);
}
```

Think of NLL as the gym cancelling your temporary membership the moment you stop coming, rather than waiting for the calendar date to expire. It is more precise and eliminates a whole class of false rejections.

---

## 7. Common Lifetime Patterns in GrindIt

### a) Returning references from methods

```rust
struct ExerciseList {
    exercises: Vec<String>,
}

impl ExerciseList {
    fn first(&self) -> Option<&str> {
        self.exercises.first().map(|s| s.as_str())
    }
    // Lifetime elision: actually fn first<'a>(&'a self) -> Option<&'a str>
    // The returned &str lives as long as &self -- as long as the list exists.
}
```

### b) Iterator returning references

```rust
struct Exercise {
    name: String,
    weight: f64,
}

fn heaviest_exercises<'a>(
    exercises: &'a [Exercise],
    min_weight: f64,
) -> impl Iterator<Item = &'a Exercise> + 'a {
    exercises.iter().filter(move |e| e.weight >= min_weight)
}
```

The `'a` on the return type tells the compiler that the iterator borrows from `exercises`. The `move` captures `min_weight` by value (it is `Copy`) so the closure does not borrow the local variable.

### c) Multiple lifetime parameters

Sometimes two references have genuinely different lifetimes:

```rust
struct Workout {
    name: String,
    score: u32,
}

struct WorkoutComparison<'a, 'b> {
    current: &'a Workout,
    previous: &'b Workout,
}

// current and previous can come from different sources with different lifetimes.
// This is like two gym members with different membership expiration dates --
// the comparison struct is only valid while BOTH memberships are active.
impl<'a, 'b> WorkoutComparison<'a, 'b> {
    fn improvement(&self) -> i64 {
        self.current.score as i64 - self.previous.score as i64
    }
}
```

---

## 8. Fighting the Borrow Checker -- Escape Hatches

When the borrow checker rejects your code, try these strategies in order -- from cleanest to most dangerous:

**1. Clone it.** If the data is small, just clone. `exercise.name.clone()` is fine for a `String`. Cloning a few kilobytes costs nothing compared to a dangling pointer bug.

**2. Own it.** Use `String` instead of `&str` in your struct. Ownership eliminates lifetime parameters entirely.

**3. Restructure.** Sometimes the borrow checker is telling you your design is wrong. If a function borrows data from two places and returns a reference to one of them, maybe it should return an owned value instead.

**4. `Rc<T>` / `Arc<T>`.** Shared ownership for when you genuinely need multiple owners. `Rc` for single-threaded, `Arc` for multi-threaded. Like a gym membership that multiple family members share.

**5. Interior mutability.** `RefCell<T>` (single-thread) or `Mutex<T>` (multi-thread) moves borrow checking to runtime. Use when the compiler cannot prove your borrowing pattern is safe but you know it is.

**6. `unsafe`.** Last resort. You are telling the compiler "I know better." You probably do not. In two years of building GrindIt, you should never need `unsafe` for application code.

A concrete GrindIt scenario: you have a component that holds `&Exercise` but needs to outlive the page render. The fix is to switch to owned `Exercise` or `Arc<Exercise>`:

```rust
use std::sync::Arc;

struct Exercise {
    name: String,
    category: String,
}

// Instead of this (lifetime trouble):
// struct ExerciseCard<'a> { exercise: &'a Exercise }

// Do this (owned, no lifetime needed):
struct ExerciseCardOwned {
    exercise: Exercise,
}

// Or this (shared ownership, multiple components can hold it):
struct ExerciseCardShared {
    exercise: Arc<Exercise>,
}
```

---

## 9. Mental Model Summary

| Concept | What it is | Gym analogy |
|---------|-----------|-------------|
| Lifetime `'a` | How long a reference is valid | Gym membership duration |
| `&T` (shared ref) | Read-only borrow | Others can watch you lift |
| `&mut T` (exclusive ref) | Read-write borrow | Everyone steps away from the rack |
| `'static` | Lives forever (or owns everything) | Lifetime membership |
| Borrow checker | Compile-time reference validator | The strict receptionist |
| NLL | Lifetime ends at last use, not scope end | Membership cancelled when you stop coming |
| Lifetime elision | Compiler infers obvious lifetimes | Receptionist knows your face -- no card needed |

The borrow checker is not your enemy. It is the strictest, most thorough code reviewer you will ever work with -- and it catches bugs at compile time that would be data races or segfaults in C, or silent corruption in a garbage-collected language. Once the mental model clicks, you will start *designing* your code to satisfy the borrow checker naturally, and the errors will become rare.

---

## 10. Try It Yourself

**Exercise 1: Longest name across two lists**

Write `fn longest_exercise_name<'a>(list_a: &'a [&str], list_b: &'a [&str]) -> &'a str` that returns the longest name from either list. Test it with two lists that have the same lifetime. Then try to call it where the two lists have *different* lifetimes (one created in an inner scope). Observe the error and fix it by either adjusting scopes or using two lifetime parameters.

**Exercise 2: Dangling reference in a struct**

Create a `WorkoutSummary<'a>` struct with `athlete: &'a str` and `exercises: &'a [String]`. Write a function that creates one from local data and returns it. Read the compiler error carefully -- it tells you exactly which lifetime is too short. Fix it by ensuring the data outlives the struct.

**Exercise 3: Clone audit**

Take this function and reduce unnecessary clones by using borrows with lifetime annotations:

```rust
fn format_exercise_list(exercises: &[String]) -> Vec<String> {
    exercises
        .iter()
        .map(|e| {
            let name = e.clone();           // <-- do we need this clone?
            let upper = name.to_uppercase();
            upper
        })
        .collect()
}
```

In this case, `e.to_uppercase()` works directly on `&String` (which derefs to `&str`), so the intermediate `clone` is unnecessary. Identify similar patterns in your own GrindIt code and eliminate them.
