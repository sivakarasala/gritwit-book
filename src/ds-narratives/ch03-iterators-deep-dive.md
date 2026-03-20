# Iterators Deep Dive: "The Trainer with a Clipboard"

You have been writing `.iter().filter().map().collect()` all chapter like it is a magic incantation. It works beautifully -- but what is actually happening? When you chain `.filter().map()`, does Rust create TWO intermediate arrays? (No.) Does it loop through the data twice? (No.) Iterator chains in Rust are zero-cost abstractions -- the compiler fuses them into a single loop that is as fast as hand-written C. Let us prove it by building the Iterator trait from scratch, implementing a custom iterator for our exercise library, and seeing exactly how lazy evaluation turns chains into single passes.

Think of an iterator as a **personal trainer with a clipboard**. They go through your workout plan one exercise at a time. They do not photocopy the entire plan first (no intermediate allocation). They just point at the next exercise when you say `next()`. Lazy evaluation means the trainer only looks at the next exercise when you ask. If you say "stop after 5" (`.take(5)`), they never even look at exercise 6. An iterator chain is stacking instructions: "Skip warm-ups, then only show me exercises over 100kg, then give me just the names." The trainer does ALL of this in ONE pass through the clipboard.

---

## 1. The Iterator Trait -- Just One Method

Everything in Rust's iterator system rests on a single trait with a single required method:

```rust
trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}
```

That is it. Everything else -- `.map()`, `.filter()`, `.collect()`, `.fold()`, `.take()`, `.enumerate()` -- is built on top of `next()`. The standard library provides over 70 adaptor and consumer methods as default implementations, all powered by that one method.

`Option<Self::Item>` is the key: `Some(value)` means "here is the next item," and `None` means "we are done." The trainer either points at the next exercise on the clipboard, or shrugs and says "that was the last one."

Let us call `.next()` manually:

```rust
fn main() {
    let exercises = vec!["Back Squat", "Deadlift", "Clean", "Snatch", "Jerk"];
    let mut iter = exercises.iter();

    println!("{:?}", iter.next()); // Some("Back Squat")
    println!("{:?}", iter.next()); // Some("Deadlift")
    println!("{:?}", iter.next()); // Some("Clean")
    println!("{:?}", iter.next()); // Some("Snatch")
    println!("{:?}", iter.next()); // Some("Jerk")
    println!("{:?}", iter.next()); // None -- clipboard is empty
    println!("{:?}", iter.next()); // None -- still empty, always None from here
}
```

No magic. The iterator holds a position, advances it on each `next()`, and returns `None` when exhausted. Every `for` loop, every `.filter().map().collect()` chain, every `.sum()` -- all of them are just calling `next()` in a loop until they get `None`.

---

## 2. Build a Custom Iterator -- ExerciseRotator

For WOD programming, coaches often rotate through a pool of movements: Monday is squats, Tuesday is pulls, Wednesday is presses, Thursday back to squats... forever. Let us build an iterator that cycles through exercises infinitely:

```rust
struct ExerciseRotator {
    exercises: Vec<String>,
    index: usize,
}

impl ExerciseRotator {
    fn new(exercises: Vec<String>) -> Self {
        Self { exercises, index: 0 }
    }
}

impl Iterator for ExerciseRotator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exercises.is_empty() {
            return None;
        }
        let exercise = self.exercises[self.index % self.exercises.len()].clone();
        self.index += 1;
        Some(exercise)
    }
}

fn main() {
    let rotator = ExerciseRotator::new(vec![
        "Back Squat".to_string(),
        "Deadlift".to_string(),
        "Bench Press".to_string(),
    ]);

    // Take 10 items from an infinite iterator
    let plan: Vec<String> = rotator.take(10).collect();
    println!("{:#?}", plan);
    // ["Back Squat", "Deadlift", "Bench Press",
    //  "Back Squat", "Deadlift", "Bench Press",
    //  "Back Squat", "Deadlift", "Bench Press",
    //  "Back Squat"]
}
```

The trainer has a clipboard with three exercises and just loops back to the top when they hit the end. `.take(10)` tells them "stop after 10, I do not care that there are more." Because the iterator is lazy, it never generates exercise 11. The clipboard analogy holds: the trainer only reads the next line when you ask.

---

## 3. Lazy Evaluation -- Nothing Happens Until You Consume

This is the part that surprises people coming from Python or JavaScript. Building an iterator chain does *nothing*. It just stacks up instructions. The trainer writes notes on the clipboard -- "skip short names, uppercase the rest" -- but does not start walking through the list until you say "go."

```rust
fn main() {
    let exercises = vec!["Back Squat", "Deadlift", "Clean", "Snatch", "Jerk"];

    // This does NOTHING -- just builds a chain of instructions
    let chain = exercises.iter()
        .filter(|e| {
            println!("  filtering: {}", e);
            e.len() > 5
        })
        .map(|e| {
            println!("  mapping: {}", e);
            e.to_uppercase()
        });

    println!("Chain created, nothing printed yet!");
    println!("---");

    // NOW it executes -- and interleaves filter + map per element
    let results: Vec<String> = chain.collect();
    println!("---");
    println!("Results: {:?}", results);
}
```

Output:

```text
Chain created, nothing printed yet!
---
  filtering: Back Squat
  mapping: Back Squat
  filtering: Deadlift
  mapping: Deadlift
  filtering: Clean
  filtering: Snatch
  mapping: Snatch
  filtering: Jerk
---
Results: ["BACK SQUAT", "DEADLIFT", "SNATCH"]
```

Look at the interleaving. The trainer does not check ALL exercises for the filter, THEN go back and uppercase them. For each exercise, they check the filter. If it passes, they immediately uppercase it and move on. "Clean" gets filtered out and never reaches `map`. This is a single pass through the data. No intermediate `Vec` is created between `filter` and `map`.

---

## 4. How Iterator Chains Fuse into One Loop

Here is the equivalence that makes iterators a zero-cost abstraction:

```rust
struct Exercise {
    name: String,
    weight: f64,
}

fn main() {
    let exercises = vec![
        Exercise { name: "Back Squat".into(), weight: 140.0 },
        Exercise { name: "Curl".into(), weight: 25.0 },
        Exercise { name: "Deadlift".into(), weight: 180.0 },
        Exercise { name: "Lateral Raise".into(), weight: 10.0 },
    ];

    // Iterator chain (idiomatic Rust)
    let result: Vec<String> = exercises.iter()
        .filter(|e| e.weight > 100.0)
        .map(|e| e.name.clone())
        .collect();

    // What the compiler ACTUALLY generates (conceptually):
    let mut result_manual = Vec::new();
    for e in &exercises {
        if e.weight > 100.0 {
            result_manual.push(e.name.clone());
        }
    }

    // Same result, same assembly. Zero overhead.
    assert_eq!(result, result_manual);
    println!("Heavy lifts: {:?}", result);
}
```

The compiler inlines every `next()` call, eliminates the adaptor structs, and produces the same machine code as the hand-written loop. This is why Rust iterators are faster than Python list comprehensions or JavaScript `.filter().map()` -- those languages DO create intermediate arrays at each step.

---

## 5. The Adaptor Zoo -- Build map() and filter() Yourself

To understand why chains fuse, let us build `map` and `filter` from scratch. Each adaptor is just a struct that wraps another iterator and implements `Iterator` itself:

```rust
struct MyMap<I, F> {
    iter: I,
    f: F,
}

impl<I, F, B> Iterator for MyMap<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> B,
{
    type Item = B;

    fn next(&mut self) -> Option<B> {
        // Pull one item from the inner iterator, apply the function
        self.iter.next().map(&mut self.f)
    }
}

struct MyFilter<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P> Iterator for MyFilter<I, P>
where
    I: Iterator,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        // Keep pulling until we find one that passes, or run out
        while let Some(item) = self.iter.next() {
            if (self.predicate)(&item) {
                return Some(item);
            }
        }
        None
    }
}

fn main() {
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Build the chain manually: filter evens, then double them
    let iter = numbers.into_iter();
    let filtered = MyFilter { iter, predicate: |x: &i32| x % 2 == 0 };
    let mut mapped = MyMap { iter: filtered, f: |x: i32| x * 2 };

    // Drive it manually
    while let Some(val) = mapped.next() {
        print!("{} ", val); // 4 8 12 16 20
    }
    println!();
}
```

Key insight: a chain like `.iter().filter().map()` creates a nested struct `MyMap<MyFilter<Iter<...>>>`. When you call `next()` on the outermost `MyMap`, it calls `next()` on `MyFilter`, which calls `next()` on the inner `Iter`. The compiler sees through all these layers and inlines them into a single tight loop. No heap allocation, no virtual dispatch, no overhead.

---

## 6. Consumers -- What Actually Drives the Chain

Adaptors are lazy -- they just set up instructions. *Consumers* are the methods that actually call `next()` in a loop and produce a final value. The trainer does not move until a consumer says "go."

```rust
fn main() {
    // Sample data
    let reps = vec![5, 3, 8, 12, 6, 15, 2, 9];
    let weights = vec![100.0, 140.0, 80.0, 60.0];
    let names = vec!["Back Squat", "Deadlift", "Clean", "Snatch"];

    // collect() -- pull all items into a collection
    let heavy: Vec<&f64> = weights.iter().filter(|w| **w > 90.0).collect();
    println!("Heavy weights: {:?}", heavy); // [100.0, 140.0]

    // sum() -- total volume
    let total_reps: i32 = reps.iter().sum();
    println!("Total reps: {}", total_reps); // 60

    // fold() -- total weight x reps (accumulate into a single value)
    let volume: f64 = weights.iter().zip(reps.iter())
        .fold(0.0, |acc, (w, r)| acc + w * (*r as f64));
    println!("Total volume: {}", volume); // 500 + 420 + 640 + 720 = 2280

    // any() -- did anyone hit double digits? (short-circuits!)
    let has_high_reps = reps.iter().any(|&r| r >= 10);
    println!("High rep set exists: {}", has_high_reps); // true

    // find() -- first match (short-circuits!)
    let first_heavy = weights.iter().find(|&&w| w > 100.0);
    println!("First heavy weight: {:?}", first_heavy); // Some(140.0)

    // count() -- how many exercises
    let count = names.iter().filter(|n| n.len() > 5).count();
    println!("Long names: {}", count); // 3

    // max() / min()
    let max_reps = reps.iter().max();
    println!("Max reps: {:?}", max_reps); // Some(15)
}
```

Short-circuiting consumers like `any()`, `all()`, and `find()` are especially powerful. If `any()` finds a match on the second element, it stops immediately -- the trainer does not bother reading the rest of the clipboard.

---

## 7. IntoIterator -- The for Loop's Secret

Every `for` loop in Rust is syntactic sugar for `IntoIterator`:

```rust
fn main() {
    let exercises = vec!["Back Squat", "Deadlift", "Clean"];

    // This:
    for exercise in &exercises {
        println!("{}", exercise);
    }

    // Is sugar for:
    let mut iter = (&exercises).into_iter();
    while let Some(exercise) = iter.next() {
        println!("{}", exercise);
    }
}
```

Three forms, three ownership levels:

| Syntax | Iterates over | Ownership |
|--------|-------------|-----------|
| `for x in &collection` | `&T` | Borrows (collection survives) |
| `for x in &mut collection` | `&mut T` | Mutable borrows |
| `for x in collection` | `T` | Takes ownership (collection consumed) |

Let us implement `IntoIterator` for a custom `WorkoutPlan`:

```rust
struct WorkoutPlan {
    movements: Vec<String>,
}

impl IntoIterator for WorkoutPlan {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.movements.into_iter()
    }
}

fn main() {
    let plan = WorkoutPlan {
        movements: vec![
            "Run 400m".to_string(),
            "21 Thrusters".to_string(),
            "21 Pull-ups".to_string(),
        ],
    };

    // Now WorkoutPlan works in for loops directly
    for movement in plan {
        println!("Do: {}", movement);
    }
}
```

---

## 8. Performance: Iterator vs Loop vs Index

All three of these compile to essentially the same machine code -- but the index version is actually *slower*:

```rust
struct Exercise {
    name: String,
    weight: f64,
}

fn main() {
    let exercises = vec![
        Exercise { name: "Back Squat".into(), weight: 140.0 },
        Exercise { name: "Curl".into(), weight: 25.0 },
        Exercise { name: "Deadlift".into(), weight: 180.0 },
    ];

    // 1. Iterator chain (idiomatic) -- fastest, no bounds checks
    let names_1: Vec<&str> = exercises.iter()
        .filter(|e| e.weight > 100.0)
        .map(|e| &*e.name)
        .collect();

    // 2. For loop -- same speed as iterators
    let mut names_2 = Vec::new();
    for e in &exercises {
        if e.weight > 100.0 {
            names_2.push(&*e.name);
        }
    }

    // 3. Index loop (C-style) -- DO NOT do this in Rust
    let mut names_3 = Vec::new();
    for i in 0..exercises.len() {
        if exercises[i].weight > 100.0 {
            names_3.push(&*exercises[i].name);
        }
    }
    // The index loop adds a bounds check on EVERY access to exercises[i].
    // The iterator version eliminates bounds checks because the compiler
    // can prove the iterator never goes out of bounds.

    assert_eq!(names_1, names_2);
    assert_eq!(names_2, names_3);
    println!("All three produce: {:?}", names_1);
}
```

The iterator version is not just more readable -- it is genuinely faster than the index loop. The compiler knows an iterator cannot go out of bounds, so it eliminates the runtime bounds checks. This is a case where the high-level abstraction is faster than the "low-level" approach.

---

## 9. Common Patterns in GrindIt

Here are iterator patterns you will use repeatedly as you build GrindIt:

```rust
use std::collections::HashMap;

fn main() {
    // --- (a) Group by category with fold ---
    let exercises = vec![
        ("Back Squat", "Weightlifting"),
        ("Deadlift", "Powerlifting"),
        ("Clean", "Weightlifting"),
        ("Bench Press", "Powerlifting"),
        ("Box Jump", "Conditioning"),
    ];

    let grouped: HashMap<&str, Vec<&str>> = exercises.iter()
        .fold(HashMap::new(), |mut acc, (name, cat)| {
            acc.entry(*cat).or_insert_with(Vec::new).push(*name);
            acc
        });
    println!("Grouped: {:?}", grouped);

    // --- (b) Running total with scan ---
    let daily_reps = vec![50, 75, 60, 90, 45];
    let running_total: Vec<i32> = daily_reps.iter()
        .scan(0, |acc, &x| { *acc += x; Some(*acc) })
        .collect();
    println!("Running total: {:?}", running_total); // [50, 125, 185, 275, 320]

    // --- (c) Zip: pair workout plan with actual scores ---
    let planned = vec![100, 110, 120];
    let actual = vec![95, 112, 118];
    let comparison: Vec<String> = planned.iter().zip(actual.iter())
        .map(|(p, a)| format!("Planned {}kg, Hit {}kg ({})", p, a,
            if a >= p { "made it" } else { "missed" }))
        .collect();
    for line in &comparison { println!("{}", line); }

    // --- (d) Flatten nested: WOD sections into movements ---
    let wod_sections = vec![
        vec!["Run 400m", "Row 500m"],
        vec!["21 Thrusters", "21 Pull-ups"],
        vec!["Stretch", "Cool down"],
    ];
    let all_movements: Vec<&&str> = wod_sections.iter()
        .flat_map(|section| section.iter())
        .collect();
    println!("All movements: {:?}", all_movements);

    // --- (e) Windows: compare consecutive workout days ---
    let daily_volume = vec![5000.0, 6200.0, 4800.0, 7100.0, 5500.0];
    let changes: Vec<String> = daily_volume.windows(2)
        .map(|pair| {
            let diff = pair[1] - pair[0];
            let arrow = if diff > 0.0 { "up" } else { "down" };
            format!("{:.0} -> {:.0} ({} {:.0})", pair[0], pair[1], arrow, diff.abs())
        })
        .collect();
    for c in &changes { println!("{}", c); }
}
```

---

## 10. Mental Model

| Concept | What it is | Gym analogy |
|---------|-----------|-------------|
| Iterator | Produces items one at a time | Trainer with clipboard |
| `next()` | Get the next item | "What is next on the plan?" |
| `None` | No more items | "That was the last exercise" |
| Adaptor (`.map`, `.filter`) | Transform the stream | "Skip warm-ups, only heavy lifts" |
| Consumer (`.collect`, `.sum`) | Drive the iteration | "OK, actually DO the workout" |
| Lazy evaluation | Nothing runs until consumed | Trainer waits until you are ready |
| Zero-cost | Compiles to a single loop | As fast as doing it yourself |
| `IntoIterator` | Anything a `for` loop can use | Anything with a workout plan |

---

## 11. Try It Yourself

**Exercise 1: SetCounter**

Implement `Iterator` for a `SetCounter` that yields set numbers paired with reps. Given reps `[5, 5, 5, 3, 3]`, it yields `(1, 5), (2, 5), (3, 5), (4, 3), (5, 3)`.

```rust
struct SetCounter {
    reps: Vec<u32>,
    index: usize,
}

impl SetCounter {
    fn new(reps: Vec<u32>) -> Self {
        Self { reps, index: 0 }
    }
}

// TODO: implement Iterator for SetCounter
// type Item = (usize, u32)  -- (set_number, reps)
// set_number starts at 1, not 0

fn main() {
    let counter = SetCounter::new(vec![5, 5, 5, 3, 3]);
    let sets: Vec<(usize, u32)> = counter.collect();
    assert_eq!(sets, vec![(1, 5), (2, 5), (3, 5), (4, 3), (5, 3)]);
    println!("Sets: {:?}", sets);
}
```

**Exercise 2: MyTake adaptor**

Build `MyTake<I>` that wraps an iterator and yields at most N items, then returns `None` forever. This is how the real `.take(n)` works under the hood.

```rust
struct MyTake<I> {
    iter: I,
    remaining: usize,
}

// TODO: implement Iterator for MyTake<I> where I: Iterator
// Decrement remaining on each next(), return None when remaining == 0

fn main() {
    let numbers = vec![10, 20, 30, 40, 50];
    let taken: Vec<&i32> = MyTake { iter: numbers.iter(), remaining: 3 }.collect();
    assert_eq!(taken, vec![&10, &20, &30]);
    println!("Taken: {:?}", taken);
}
```

**Exercise 3: Highest volume day**

Given workout logs, use an iterator chain to find the day with the highest total volume (weight x reps). Use `.map()` and `.max_by_key()`.

```rust
struct WorkoutLog {
    date: String,
    weight: f64,
    reps: u32,
}

fn main() {
    let logs = vec![
        WorkoutLog { date: "Monday".into(), weight: 100.0, reps: 5 },
        WorkoutLog { date: "Tuesday".into(), weight: 60.0, reps: 12 },
        WorkoutLog { date: "Wednesday".into(), weight: 140.0, reps: 3 },
        WorkoutLog { date: "Thursday".into(), weight: 80.0, reps: 10 },
    ];

    // TODO: use .iter().max_by_key() to find the log with highest volume
    // volume = weight * reps as f64
    // Then print the date
    // Hint: max_by_key needs Ord, but f64 is not Ord.
    //       Multiply by 100 and cast to i64: (log.weight * log.reps as f64 * 100.0) as i64

    // let best_day = logs.iter(). ... ;
    // println!("Highest volume day: {}", best_day.unwrap().date);
}
```

---

All of Rust's iterator magic rests on one method: `next()`. Adaptors are structs wrapping structs, consumers are loops calling `next()`, and the compiler flattens the whole tower into a single pass. The trainer with the clipboard never makes a photocopy. They just read the next line, apply all your instructions, and hand you the result -- one exercise at a time, zero wasted effort.
