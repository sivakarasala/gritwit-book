# The Building Blocks — Variables, Functions & Loops

This is the chapter where you go from printing hardcoded text to writing programs that *compute things*. By the end, you will have a working command-line workout calculator that loops through sets, calculates total training volume, and classifies your workout intensity.

That is a real program. You will write it yourself.

We are covering four concepts:

1. **Variables** — storing data
2. **Types** — what kinds of data Rust understands
3. **Functions** — reusable blocks of logic
4. **Control flow** — making decisions and repeating actions

Each builds on the previous one. Take your time. Type every example — do not copy and paste.

---

## Variables: Storing Data

In Chapter 0.3, you hardcoded everything directly into `println!` statements:

```rust
println!("Exercise: Bench Press");
println!("Reps: 10");
```

This works, but it has a problem. What if "Bench Press" appears in five different `println!` lines and you want to change it to "Back Squat"? You would have to change it in five places. Miss one and your program is inconsistent.

**Variables** solve this. A variable is a named container that holds a value. You set it once, then use the name everywhere.

### Creating a variable with `let`

```rust
fn main() {
    let exercise = "Back Squat";
    println!("Exercise: {}", exercise);
}
```

Let's break down `let exercise = "Back Squat";`:

| Part | Meaning |
|------|---------|
| `let` | "I am creating a new variable" |
| `exercise` | The name you chose for the variable |
| `=` | "and its value is" |
| `"Back Squat"` | The value being stored |
| `;` | End of the statement |

The `{}` inside the `println!` string is a **placeholder**. When the program runs, Rust replaces `{}` with the value of `exercise`. So the output is:

```text
Exercise: Back Squat
```

You can use multiple placeholders in one `println!`:

```rust
fn main() {
    let exercise = "Back Squat";
    let reps = 5;
    let weight = 100.0;
    println!("{}: {} reps at {} kg", exercise, reps, weight);
}
```

Output:

```text
Back Squat: 5 reps at 100.0 kg
```

The placeholders `{}` are filled in order — the first `{}` gets `exercise`, the second gets `reps`, the third gets `weight`.

### Immutability: Rust's safety default

Try this program:

```rust
fn main() {
    let reps = 5;
    println!("Planned reps: {}", reps);
    reps = 8;
    println!("Actual reps: {}", reps);
}
```

Run it with `cargo run`. You will get an error:

```text
error[E0384]: cannot assign twice to immutable variable `reps`
 --> src/main.rs:4:5
  |
2 |     let reps = 5;
  |         ---- first assignment to `reps`
3 |     println!("Planned reps: {}", reps);
4 |     reps = 8;
  |     ^^^^^^^^ cannot assign twice to immutable variable
  |
help: consider making this binding mutable: `let mut reps`
```

In Rust, variables are **immutable** by default. That means once you set a value, you cannot change it.

*Why?* Because changing data is one of the most common sources of bugs in programming. If a value is immutable, you can look at the line where it was created and know *with certainty* what it contains — no need to trace through the whole program to see if something changed it later.

Think of it like writing a workout plan in permanent marker. You know the plan will not change mid-workout. That is a feature, not a limitation.

### Mutable variables with `let mut`

When you genuinely need a value to change — like tracking a running total — use `let mut`:

```rust
fn main() {
    let mut reps = 5;
    println!("Planned reps: {}", reps);
    reps = 8;
    println!("Actual reps: {}", reps);
}
```

Output:

```text
Planned reps: 5
Actual reps: 8
```

The `mut` keyword (short for "mutable") tells Rust: "I know this value will change, and that is intentional."

### The rule of thumb

- Use `let` (immutable) by default
- Only add `mut` when you have a reason to change the value later
- If you are not sure, start with `let` — the compiler will tell you if you need `mut`

---

## Types: What Kinds of Data Exist

Every value in Rust has a **type** — a label that tells the compiler what kind of data it is and what you can do with it. You would not try to add "Back Squat" + "Deadlift" and expect a number. Types prevent that kind of mistake.

Here are the types you will use most often:

### Integers: `i32`

Whole numbers, positive or negative. "i32" means "a 32-bit integer."

```rust
let sets: i32 = 5;
let reps: i32 = 10;
let negative_example: i32 = -3;
```

Use `i32` for anything you count in whole numbers: reps, sets, rounds, calories.

### Floating-point numbers: `f64`

Numbers with decimal points. "f64" means "a 64-bit floating-point number."

```rust
let weight_kg: f64 = 102.5;
let body_weight: f64 = 80.0;
let time_seconds: f64 = 245.7;
```

Use `f64` for weights, times, percentages — anything that can have a fractional part. Note that even `80.0` needs the decimal point to be an `f64`.

### Booleans: `bool`

True or false. Only two possible values.

```rust
let is_completed: bool = true;
let is_personal_record: bool = false;
```

Use `bool` for yes/no questions: Is the set done? Is this a PR? Is the timer running?

### Text: `String` and `&str`

Rust has two kinds of text, which can be confusing at first. For now, here is the simple version:

- `&str` (pronounced "string slice") — text that is written directly in your code, like `"Back Squat"`. You cannot change it.
- `String` — text that your program creates or modifies at runtime. You can change it.

```rust
let exercise: &str = "Back Squat";              // fixed text — a string slice
let note: String = String::from("Felt strong"); // dynamic text — a String
```

For Part 0, we will mostly use `&str` because we are working with fixed text. You will learn the full story of `String` vs `&str` when we build the GrindIt app in Chapter 1.

### Type annotations vs. type inference

In all the examples above, I wrote the type explicitly: `let reps: i32 = 5;`. But Rust can usually *figure out* the type on its own:

```rust
let reps = 5;           // Rust infers i32
let weight = 100.0;     // Rust infers f64
let exercise = "Squat"; // Rust infers &str
let done = true;        // Rust infers bool
```

Both forms are valid. When you are learning, writing the type explicitly can help you remember what each variable is. As you get comfortable, you can let Rust infer types and only annotate when it is ambiguous or when you want to be extra clear.

### Why types matter

Types catch bugs at compile time, before your program ever runs. If you accidentally write:

```rust
let total = "five" * 3;
```

Rust will refuse to compile and tell you that you cannot multiply text by a number. In many other languages, this kind of mistake would silently produce a weird result at runtime. Rust catches it immediately.

---

## Functions: Reusable Blocks of Logic

You already know one function: `fn main()`. It is the entry point of every Rust program. But you can create your own functions to organize your code and avoid repeating yourself.

### Why functions?

Imagine you need to calculate workout volume (sets x reps x weight) in five different places in your program. Without functions, you would write the same math five times. If you later discover a bug in that math, you would have to fix it in five places.

A function lets you write the logic *once* and *call* it from anywhere.

### Defining a function

```rust
fn calculate_volume(sets: i32, reps: i32, weight: f64) -> f64 {
    let volume = sets as f64 * reps as f64 * weight;
    volume
}
```

Let's break this down piece by piece:

| Part | Meaning |
|------|---------|
| `fn` | "I am defining a function" |
| `calculate_volume` | The name of the function |
| `(sets: i32, reps: i32, weight: f64)` | **Parameters** — the inputs the function needs, each with a name and type |
| `-> f64` | **Return type** — this function produces an `f64` value |
| `{ ... }` | The **body** — the code that runs when the function is called |
| `sets as f64` | Converts the integer `sets` to a floating-point number so we can multiply it with `weight` |
| `volume` | The last expression without a semicolon is the **return value** |

That last point is important and unique to Rust: **the last expression in a function is what gets returned.** No `return` keyword needed (though Rust does have one for early returns).

### Calling a function

```rust
fn calculate_volume(sets: i32, reps: i32, weight: f64) -> f64 {
    sets as f64 * reps as f64 * weight
}

fn main() {
    let volume = calculate_volume(5, 5, 100.0);
    println!("Total volume: {} kg", volume);
}
```

Output:

```text
Total volume: 2500 kg
```

When you write `calculate_volume(5, 5, 100.0)`:

1. Rust passes `5` into `sets`, `5` into `reps`, and `100.0` into `weight`
2. The function calculates `5.0 * 5.0 * 100.0 = 2500.0`
3. The result `2500.0` is returned and stored in the `volume` variable in `main`

### Functions with no return value

Not every function needs to return something. A function that just prints output does not need a return type:

```rust
fn print_workout_header(exercise: &str) {
    println!("===========================");
    println!("  Exercise: {}", exercise);
    println!("===========================");
}

fn main() {
    print_workout_header("Back Squat");
}
```

Output:

```text
===========================
  Exercise: Back Squat
===========================
```

When there is no `-> Type` in the function signature, the function returns nothing (technically it returns `()`, called the "unit type," but you do not need to think about that now).

### Function placement

In Rust, it does not matter whether you define a function before or after `main`. This works fine:

```rust
fn main() {
    print_greeting();
}

fn print_greeting() {
    println!("Welcome to GrindIt!");
}
```

Rust reads the entire file before compiling, so it knows about all functions regardless of their position.

---

## Control Flow: Making Decisions and Repeating Actions

So far, our programs run every line once, top to bottom. Real programs need to **make decisions** ("if the volume is high, print a warning") and **repeat actions** ("do this for each of the 5 sets"). That is what control flow gives us.

### `if` / `else` — Making decisions

```rust
fn main() {
    let volume = 2500.0;

    if volume > 5000.0 {
        println!("Heavy session!");
    } else if volume > 1000.0 {
        println!("Moderate session.");
    } else {
        println!("Light session.");
    }
}
```

Output:

```text
Moderate session.
```

How it works:

1. Rust checks `volume > 5000.0`. Is 2500 greater than 5000? No. Skip that block.
2. Rust checks `volume > 1000.0`. Is 2500 greater than 1000? Yes. Run that block.
3. The `else` block is skipped because a condition already matched.

The conditions are checked **in order, from top to bottom.** The first one that is true wins. All others are skipped.

### Comparison operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `>` | greater than | `volume > 5000.0` |
| `<` | less than | `reps < 5` |
| `>=` | greater than or equal to | `sets >= 3` |
| `<=` | less than or equal to | `weight <= 60.0` |
| `==` | equal to | `exercise == "Squat"` |
| `!=` | not equal to | `exercise != "Rest"` |

Note the double equals `==` for comparison. A single `=` means "assign a value." A double `==` means "check if equal." Mixing them up is a common mistake.

### `for` loops — Repeating with a range

A `for` loop runs a block of code once for each value in a sequence:

```rust
fn main() {
    for set_number in 1..=5 {
        println!("Set {}: 5 reps @ 100 kg", set_number);
    }
}
```

Output:

```text
Set 1: 5 reps @ 100 kg
Set 2: 5 reps @ 100 kg
Set 3: 5 reps @ 100 kg
Set 4: 5 reps @ 100 kg
Set 5: 5 reps @ 100 kg
```

The `1..=5` is a **range**. It means "from 1 to 5, inclusive." Each time through the loop, `set_number` takes the next value in the range: 1, then 2, then 3, then 4, then 5.

Two kinds of ranges:

| Syntax | Meaning | Values |
|--------|---------|--------|
| `1..5` | 1 up to but *not including* 5 | 1, 2, 3, 4 |
| `1..=5` | 1 up to and *including* 5 | 1, 2, 3, 4, 5 |

For sets and reps, `1..=5` (inclusive) is almost always what you want, because "5 sets" means sets 1 through 5.

### Loops with mutable variables: running totals

Loops become powerful when combined with a mutable variable that accumulates a result:

```rust
fn main() {
    let reps = 5;
    let weight = 100.0;
    let mut total_volume = 0.0;

    for set_number in 1..=5 {
        let set_volume = reps as f64 * weight;
        total_volume += set_volume;
        println!("Set {}: {} reps @ {} kg (running total: {} kg)",
            set_number, reps, weight, total_volume);
    }

    println!("Total volume: {} kg", total_volume);
}
```

Output:

```text
Set 1: 5 reps @ 100 kg (running total: 500 kg)
Set 2: 5 reps @ 100 kg (running total: 1000 kg)
Set 3: 5 reps @ 100 kg (running total: 1500 kg)
Set 4: 5 reps @ 100 kg (running total: 2000 kg)
Set 5: 5 reps @ 100 kg (running total: 2500 kg)
Total volume: 2500 kg
```

The line `total_volume += set_volume;` means "add `set_volume` to `total_volume`." It is shorthand for `total_volume = total_volume + set_volume;`.

Notice that `total_volume` must be `let mut` because its value changes each iteration.

### `while` loops — Repeating with a condition

A `while` loop keeps running as long as a condition is true:

```rust
fn main() {
    let mut remaining_reps = 10;

    while remaining_reps > 0 {
        println!("{} reps to go...", remaining_reps);
        remaining_reps -= 1;
    }

    println!("Done! Good set.");
}
```

Output:

```text
10 reps to go...
9 reps to go...
8 reps to go...
7 reps to go...
6 reps to go...
5 reps to go...
4 reps to go...
3 reps to go...
2 reps to go...
1 reps to go...
Done! Good set.
```

We will use `for` loops much more often than `while` loops, but it is good to know both exist. Use `for` when you know how many times to repeat. Use `while` when you want to keep going until a condition changes.

---

## Putting It All Together

Let's combine everything — variables, types, functions, if/else, and for loops — into a single program: a workout volume calculator.

Here is the plan (pseudocode first, as we learned in Chapter 0.3):

```text
PROGRAM: Workout Volume Calculator

1. Set exercise to "Back Squat", reps to 5, weight to 100.0 kg, sets to 5
2. Print the workout header
3. Loop through each set (1 to 5):
   a. Calculate this set's volume (reps × weight)
   b. Add it to the running total
   c. Print the set number and running total
4. Print the total volume
5. Classify the workout: Light / Moderate / Heavy
6. Print the classification
```

And here is the Rust implementation:

```rust
fn calculate_volume(sets: i32, reps: i32, weight: f64) -> f64 {
    sets as f64 * reps as f64 * weight
}

fn classify_workout(volume: f64) -> &'static str {
    if volume > 5000.0 {
        "Heavy"
    } else if volume >= 1000.0 {
        "Moderate"
    } else {
        "Light"
    }
}

fn main() {
    // Input
    let exercise = "Back Squat";
    let sets = 5;
    let reps = 5;
    let weight = 100.0;

    // Header
    println!("===========================");
    println!("  {}", exercise);
    println!("  {} sets x {} reps @ {} kg", sets, reps, weight);
    println!("===========================");

    // Processing: loop through sets and track volume
    let mut total_volume = 0.0;

    for set_number in 1..=sets {
        let set_volume = reps as f64 * weight;
        total_volume += set_volume;
        println!("  Set {}: {} reps @ {} kg  |  Running total: {} kg",
            set_number, reps, weight, total_volume);
    }

    // Output
    println!("===========================");
    let expected_volume = calculate_volume(sets, reps, weight);
    let classification = classify_workout(expected_volume);
    println!("  Total volume: {} kg", expected_volume);
    println!("  Intensity: {}", classification);
    println!("===========================");
}
```

Output:

```text
===========================
  Back Squat
  5 sets x 5 reps @ 100 kg
===========================
  Set 1: 5 reps @ 100 kg  |  Running total: 500 kg
  Set 2: 5 reps @ 100 kg  |  Running total: 1000 kg
  Set 3: 5 reps @ 100 kg  |  Running total: 1500 kg
  Set 4: 5 reps @ 100 kg  |  Running total: 2000 kg
  Set 5: 5 reps @ 100 kg  |  Running total: 2500 kg
===========================
  Total volume: 2500 kg
  Intensity: Moderate
===========================
```

Take a moment to trace through this program line by line. You understand every piece now:

- `let` and `let mut` for variables
- `i32`, `f64`, and `&str` for types
- `fn calculate_volume(...)` and `fn classify_workout(...)` for functions
- `for set_number in 1..=sets` for looping
- `if/else if/else` for classification

A note on `&'static str`: You may have noticed `-> &'static str` in the `classify_workout` function. The `'static` part is a *lifetime annotation* — it tells Rust that the returned text lives for the entire duration of the program. Since `"Heavy"`, `"Moderate"`, and `"Light"` are written directly in the source code, they are always available. Do not worry about understanding lifetimes yet. For now, just know that when a function returns a hardcoded string, you write `-> &'static str`. You will learn lifetimes properly when we build the GrindIt app.

---

## Exercises

### Exercise 1: Variables

**Goal:** Declare variables for a workout and print them. Then experience the immutability error and fix it.

**Instructions:**

1. Create a new project: `cargo new workout_variables`
2. In `src/main.rs`, declare these variables:
   - `exercise` — a `String` with value `"Overhead Press"` (use `String::from("Overhead Press")`)
   - `reps` — an `i32` with value `8`
   - `weight_kg` — an `f64` with value `40.0`
   - `is_completed` — a `bool` with value `false`
3. Print all four values on separate lines, using `{}` placeholders
4. Now *after* the print statements, try to change `is_completed` to `true`. Compile and read the error.
5. Fix the error by making the variable mutable.
6. Print the updated value of `is_completed`.

Expected final output:

```text
Exercise: Overhead Press
Reps: 8
Weight: 40 kg
Completed: false
Updated — Completed: true
```

<details>
<summary>Hint 1</summary>

To create a `String`, use `let exercise: String = String::from("Overhead Press");`. To print it, use `println!("Exercise: {}", exercise);`.

</details>

<details>
<summary>Hint 2</summary>

When you try to reassign `is_completed = true;` and the compiler complains, read the `help:` line. It will tell you to add `mut` to the declaration: `let mut is_completed: bool = false;`.

</details>

<details>
<summary>Hint 3</summary>

You need `println!("Updated — Completed: {}", is_completed);` after the reassignment.

</details>

<details>
<summary>Solution</summary>

```rust
fn main() {
    let exercise: String = String::from("Overhead Press");
    let reps: i32 = 8;
    let weight_kg: f64 = 40.0;
    let mut is_completed: bool = false;

    println!("Exercise: {}", exercise);
    println!("Reps: {}", reps);
    println!("Weight: {} kg", weight_kg);
    println!("Completed: {}", is_completed);

    is_completed = true;
    println!("Updated — Completed: {}", is_completed);
}
```

When you first write `let is_completed: bool = false;` (without `mut`) and try `is_completed = true;`, the compiler error says:

```text
error[E0384]: cannot assign twice to immutable variable `is_completed`
```

Adding `mut` fixes it. This is Rust's immutability default in action — it made you explicitly say "yes, I want this value to change."

</details>

---

### Exercise 2: Functions

**Goal:** Write a function that calculates workout volume, and call it from `main`.

**Instructions:**

1. Create a new project: `cargo new volume_calculator`
2. Write a function `fn calculate_volume(sets: i32, reps: i32, weight: f64) -> f64` that returns `sets * reps * weight` (remember to convert `sets` and `reps` to `f64` before multiplying)
3. In `main`, call the function for a Back Squat workout: 5 sets, 5 reps, 100 kg
4. Store the result in a variable called `volume`
5. Print: `"Back Squat — Total volume: XXXX kg"` where XXXX is the calculated value

Expected output:

```text
Back Squat — Total volume: 2500 kg
```

<details>
<summary>Hint 1</summary>

To convert an `i32` to `f64`, use `as f64`. For example: `sets as f64`.

</details>

<details>
<summary>Hint 2</summary>

The function body should be: `sets as f64 * reps as f64 * weight`. Remember, the last expression (without a semicolon) is the return value.

</details>

<details>
<summary>Hint 3</summary>

Call the function like this: `let volume = calculate_volume(5, 5, 100.0);`. Note that the weight must be `100.0` (with a decimal), not `100`, because the parameter type is `f64`.

</details>

<details>
<summary>Solution</summary>

```rust
fn calculate_volume(sets: i32, reps: i32, weight: f64) -> f64 {
    sets as f64 * reps as f64 * weight
}

fn main() {
    let volume = calculate_volume(5, 5, 100.0);
    println!("Back Squat — Total volume: {} kg", volume);
}
```

A few things to notice:

- The function is defined *outside* of `main`. It is a separate block of code.
- The body has no semicolon after the multiplication — this makes it the return value.
- The function takes `i32` values for sets and reps (because they are whole numbers) and `f64` for weight (because it can have decimals). Inside the function, we convert the integers to `f64` before multiplying.

Try changing the values and running again. What is the volume for 3 sets of 10 reps at 60 kg?

</details>

---

### Exercise 3: Control Flow

**Goal:** Write a function that classifies a workout based on its volume.

**Instructions:**

1. Continue in the same project (or create `cargo new workout_classifier`)
2. Write a function `fn classify_workout(volume: f64) -> &'static str` that returns:
   - `"Light"` if volume is less than 1000
   - `"Moderate"` if volume is between 1000 and 5000 (inclusive of 1000)
   - `"Heavy"` if volume is greater than 5000
3. In `main`, test it with three different volumes and print the results:
   - 500.0 (should print "Light")
   - 2500.0 (should print "Moderate")
   - 7500.0 (should print "Heavy")

Expected output:

```text
Volume: 500 kg — Light
Volume: 2500 kg — Moderate
Volume: 7500 kg — Heavy
```

<details>
<summary>Hint 1</summary>

The function uses `if/else if/else`:

```rust
fn classify_workout(volume: f64) -> &'static str {
    if volume > 5000.0 {
        // return "Heavy"
    } else if ... {
        // ...
    } else {
        // ...
    }
}
```

The return value is the string in each branch — no semicolon, no `return` keyword needed.

</details>

<details>
<summary>Hint 2</summary>

The order of conditions matters. Check `> 5000.0` first, then `>= 1000.0`, then the `else` catches everything below 1000.

</details>

<details>
<summary>Hint 3</summary>

Print each result like this:

```rust
println!("Volume: {} kg — {}", 500.0, classify_workout(500.0));
```

You can call a function directly inside `println!`.

</details>

<details>
<summary>Solution</summary>

```rust
fn classify_workout(volume: f64) -> &'static str {
    if volume > 5000.0 {
        "Heavy"
    } else if volume >= 1000.0 {
        "Moderate"
    } else {
        "Light"
    }
}

fn main() {
    println!("Volume: {} kg — {}", 500.0, classify_workout(500.0));
    println!("Volume: {} kg — {}", 2500.0, classify_workout(2500.0));
    println!("Volume: {} kg — {}", 7500.0, classify_workout(7500.0));
}
```

Notice that the strings `"Heavy"`, `"Moderate"`, and `"Light"` have no semicolons — they are the return values of the function. Each `if/else` branch is an expression that evaluates to a value. Rust uses the value of whichever branch is true as the function's return value.

Try adding a fourth classification: "Extreme" for volume above 10,000. Where would you add it?

</details>

---

### Exercise 4: Loops + Everything Together

**Goal:** Write a complete program that loops through 5 sets, prints each set, calculates a running total, and classifies the workout at the end.

This is the final exercise of Part 0 — it combines variables, types, functions, if/else, and for loops into one program.

**Instructions:**

1. Create a new project: `cargo new workout_tracker`
2. Write two functions:
   - `fn calculate_volume(sets: i32, reps: i32, weight: f64) -> f64`
   - `fn classify_workout(volume: f64) -> &'static str`
3. In `main`:
   - Set variables: exercise = "Back Squat", sets = 5, reps = 5, weight = 100.0
   - Print a header with the exercise name
   - Create a mutable variable `total_volume` starting at 0.0
   - Use a `for` loop over `1..=sets` to:
     - Calculate volume for this set: `reps as f64 * weight`
     - Add it to `total_volume`
     - Print: `"  Set X: Y reps @ Z kg"` where X is the set number
   - After the loop, print the total volume
   - Call `classify_workout` and print the classification

Expected output:

```text
=== Back Squat ===
  Set 1: 5 reps @ 100 kg
  Set 2: 5 reps @ 100 kg
  Set 3: 5 reps @ 100 kg
  Set 4: 5 reps @ 100 kg
  Set 5: 5 reps @ 100 kg
==================
Total volume: 2500 kg
Intensity: Moderate
```

<details>
<summary>Hint 1</summary>

Start with the skeleton:

```rust
fn calculate_volume(...) -> f64 {
    // ...
}

fn classify_workout(...) -> &'static str {
    // ...
}

fn main() {
    let exercise = "Back Squat";
    let sets = 5;
    let reps = 5;
    let weight = 100.0;

    // Header
    println!("=== {} ===", exercise);

    // Loop
    let mut total_volume = 0.0;
    for set_number in 1..=sets {
        // ... fill this in
    }

    // Summary
    // ... fill this in
}
```

</details>

<details>
<summary>Hint 2</summary>

Inside the loop, add the set's volume to the running total:

```rust
let set_volume = reps as f64 * weight;
total_volume += set_volume;
```

Then print the set information:

```rust
println!("  Set {}: {} reps @ {} kg", set_number, reps, weight);
```

</details>

<details>
<summary>Hint 3</summary>

After the loop, use `calculate_volume` to double-check the total, and `classify_workout` for the classification:

```rust
let final_volume = calculate_volume(sets, reps, weight);
let classification = classify_workout(final_volume);
println!("Total volume: {} kg", final_volume);
println!("Intensity: {}", classification);
```

</details>

<details>
<summary>Solution</summary>

```rust
fn calculate_volume(sets: i32, reps: i32, weight: f64) -> f64 {
    sets as f64 * reps as f64 * weight
}

fn classify_workout(volume: f64) -> &'static str {
    if volume > 5000.0 {
        "Heavy"
    } else if volume >= 1000.0 {
        "Moderate"
    } else {
        "Light"
    }
}

fn main() {
    // Input
    let exercise = "Back Squat";
    let sets = 5;
    let reps = 5;
    let weight = 100.0;

    // Header
    println!("=== {} ===", exercise);

    // Loop through each set
    let mut total_volume = 0.0;
    for set_number in 1..=sets {
        let set_volume = reps as f64 * weight;
        total_volume += set_volume;
        println!("  Set {}: {} reps @ {} kg", set_number, reps, weight);
    }

    // Summary
    println!("==================");
    let final_volume = calculate_volume(sets, reps, weight);
    let classification = classify_workout(final_volume);
    println!("Total volume: {} kg", final_volume);
    println!("Intensity: {}", classification);
}
```

Run it with `cargo run`:

```text
=== Back Squat ===
  Set 1: 5 reps @ 100 kg
  Set 2: 5 reps @ 100 kg
  Set 3: 5 reps @ 100 kg
  Set 4: 5 reps @ 100 kg
  Set 5: 5 reps @ 100 kg
==================
Total volume: 2500 kg
Intensity: Moderate
```

**Bonus challenge:** Try changing the exercise, reps, and weight to see different results. What values push the volume above 5000? Try "Deadlift", 5 sets, 3 reps, 180 kg — is it Heavy?

You now have a real program that does real computation. Every line of it uses something you learned in this chapter.

</details>

---

## What You've Learned

This was the most important chapter of Part 0. Here is everything you now know:

- **Variables** store data. Use `let` for immutable values (the default) and `let mut` when a value needs to change.
- **Types** tell Rust what kind of data a variable holds: `i32` for whole numbers, `f64` for decimals, `bool` for true/false, `&str` and `String` for text.
- **Functions** let you write logic once and use it anywhere. They take parameters (inputs) and return values (outputs). The last expression without a semicolon is the return value.
- **`if/else`** lets your program make decisions based on conditions.
- **`for` loops** repeat a block of code for each value in a range. Combined with mutable variables, they can accumulate results (like running totals).
- **`while` loops** repeat as long as a condition is true.

You can now write programs that store data, calculate results, make decisions, and repeat actions. Those four capabilities are the foundation of every program ever written.

---

## What's Next

Part 0 is complete. You have gone from "what is a terminal?" to writing a working Rust program with functions, loops, and conditional logic. That is a serious accomplishment.

In [Chapter 1: Hello, GrindIt!](../beginner/ch01-hello-grindit.md), we leave the command line behind and start building the actual GrindIt web application. You will set up a Leptos project, see your first web page rendered in a browser, and begin learning how modern web apps work — all in Rust.

The concepts from Part 0 — variables, functions, types, loops, if/else — will appear on every single page of the rest of this book. You now have the vocabulary to read and write Rust. It is time to build something real.
