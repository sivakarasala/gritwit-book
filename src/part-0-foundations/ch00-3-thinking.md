# Thinking Like a Programmer

In Chapter 0.2, you wrote your first Rust program and watched the compiler turn your code into something the computer could run. You learned `fn main()`, `println!`, and the build-run cycle.

But writing code is only half of programming. The other half — the harder half — is **thinking**. Before you type a single character, you need to know *what* to tell the computer to do and *in what order*.

This chapter teaches you that thinking process. By the end, you will be able to:

- Break any problem into a sequence of small steps
- Recognize the Input-Process-Output pattern that every program follows
- Write pseudocode (a plan in plain English) before writing Rust
- Read Rust compiler error messages without panicking
- Debug methodically instead of guessing randomly

No new Rust syntax in this chapter — just three short programs and a lot of thinking practice. This is the chapter that separates people who *memorize code* from people who *understand programming*.

---

## Breaking Problems into Steps

Imagine you are coaching a friend through their first gym workout. They have never touched a barbell before. You would not say "just work out." You would give them **specific steps in a specific order**:

1. Choose an exercise (e.g., Back Squat)
2. Load the barbell with the right weight
3. Do 5 reps
4. Rest for 2 minutes
5. Repeat for 5 sets
6. Record the result

That list is a **program**. Each line is an **instruction**. The order matters — you cannot rest before you lift, and you cannot record a result before you finish.

Programming works exactly the same way. A computer is your extremely obedient but extremely literal friend. It will do precisely what you say, in the exact order you say it. It will never "figure out what you meant." If you skip a step or put steps in the wrong order, the result will be wrong.

### The workout analogy

Let's say we want to build a program that helps someone track a workout. Before writing any code, we need to answer:

- What information do we need? (exercise name, weight, reps, sets)
- What do we do with that information? (calculate total volume)
- What do we show the user? (a summary of the workout)

These three questions map directly to the most fundamental pattern in all of computing.

---

## Input, Processing, Output

Every program you will ever write follows this pattern:

```
INPUT  →  PROCESSING  →  OUTPUT
```

- **Input**: The data your program starts with. It could come from the user typing on a keyboard, from a file, from a database, or from the internet.
- **Processing**: What your program *does* with the data. Calculations, comparisons, sorting, filtering, transforming.
- **Output**: What your program produces. Text on the screen, a file saved to disk, a web page, a notification.

### Example: Workout volume calculator

Let's trace this pattern for a simple workout tracker:

| Stage | What happens | Example |
|-------|-------------|---------|
| **Input** | Exercise name, sets, reps, weight | "Back Squat", 5 sets, 5 reps, 100 kg |
| **Processing** | Calculate total volume: sets × reps × weight | 5 × 5 × 100 = 2,500 kg |
| **Output** | Display the result | "Back Squat — Total volume: 2500 kg" |

That is the entire program. Three stages. Every feature of the GrindIt app we will build later — the exercise library, the workout logger, the leaderboard — is just this pattern repeated and combined in different ways.

### Example: Rep counter

| Stage | What happens | Example |
|-------|-------------|---------|
| **Input** | Target reps, completed reps | Target: 10, Completed: 7 |
| **Processing** | Calculate remaining: target - completed | 10 - 7 = 3 |
| **Output** | Display remaining reps | "3 reps to go!" |

### Example: Rest timer

| Stage | What happens | Example |
|-------|-------------|---------|
| **Input** | Rest duration in seconds | 120 seconds |
| **Processing** | Count down from duration to zero | 120, 119, 118, ... 0 |
| **Output** | Display "REST COMPLETE!" | "REST COMPLETE!" |

Once you start seeing this pattern, you will see it everywhere — not just in code, but in spreadsheets, recipes, tax forms, and assembly instructions.

---

## Pseudocode: Planning Before Coding

**Pseudocode** is a plan written in plain English (or any human language) that describes what a program should do, step by step. It is not real code. No computer can run it. Its purpose is to help *you* think clearly before you start worrying about syntax.

Here is pseudocode for our workout volume calculator:

```text
PROGRAM: Workout Volume Calculator

1. Set the exercise name to "Back Squat"
2. Set the number of sets to 5
3. Set the number of reps to 5
4. Set the weight to 100 kg
5. Calculate total volume = sets × reps × weight
6. Print the exercise name
7. Print the total volume
8. If volume is greater than 5000, print "Heavy session!"
9. Otherwise, print "Nice work!"
```

Notice a few things about this pseudocode:

- **Each line does one thing.** No line tries to do three things at once.
- **The order matters.** We set the values before we calculate, and we calculate before we print.
- **It uses plain words.** "Set", "Calculate", "Print", "If." No curly braces, no semicolons.
- **It is specific.** Not "do some math" but "calculate total volume = sets × reps × weight."

### Why bother with pseudocode?

When you sit down to write Rust code, you have to think about two things simultaneously:

1. **What** the program should do (the logic)
2. **How** to say it in Rust (the syntax)

That is two hard things at once. Pseudocode lets you solve problem #1 first, on its own, so that when you open your editor, you only have to solve problem #2. One hard thing at a time.

Professional programmers write pseudocode, sketches, and outlines before coding. It is not a beginner crutch — it is a professional practice.

---

## Reading Error Messages

Here is a truth that surprises most beginners: **you will spend more time reading error messages than writing code.** This is normal. This is healthy. Error messages are not failures — they are the compiler *helping you*.

Rust has one of the best compilers in any programming language. It does not just say "something is wrong." It tells you:

- **What** is wrong
- **Where** it went wrong (file name, line number, column number)
- **Why** it is wrong
- Often, **how to fix it**

Let's look at three common errors and learn to read them.

### Error 1: The missing semicolon

Here is a program with a bug:

```rust
fn main() {
    println!("Workout: Back Squat")
    println!("Sets: 5")
}
```

If you try to compile this with `cargo run`, Rust prints:

```text
error: expected `;`, found `println`
 --> src/main.rs:3:5
  |
2 |     println!("Workout: Back Squat")
  |                                    ^ help: add `;` here
3 |     println!("Sets: 5")
  |     ^^^^^^^ unexpected token
```

Let's decode each part:

| Part | Meaning |
|------|---------|
| `error:` | This is an error (your code will not compile) |
| `expected ';', found 'println'` | Rust expected a semicolon but found the next `println` instead |
| `--> src/main.rs:3:5` | The problem is in file `src/main.rs`, at line 3, column 5 |
| `help: add ';' here` | Rust is literally telling you the fix |

**The fix:** Add a semicolon at the end of line 2:

```rust
fn main() {
    println!("Workout: Back Squat");
    println!("Sets: 5");
}
```

Every statement in Rust ends with a semicolon. Forget one, and the compiler catches it instantly.

### Error 2: The mismatched type

```rust
fn main() {
    let reps: i32 = "five";
    println!("Reps: {}", reps);
}
```

Compiler output:

```text
error[E0308]: mismatched types
 --> src/main.rs:2:21
  |
2 |     let reps: i32 = "five";
  |               ---   ^^^^^^ expected `i32`, found `&str`
  |               |
  |               expected due to this
```

| Part | Meaning |
|------|---------|
| `error[E0308]` | Error code E0308 — you can search "Rust E0308" for details |
| `mismatched types` | You tried to put the wrong kind of data into a variable |
| `expected 'i32', found '&str'` | The variable expects a number (`i32`) but you gave it text (`"five"`) |
| `expected due to this` | The arrow points to `: i32`, showing *why* Rust expected a number |

**The fix:** Use an actual number, not the word "five":

```rust
fn main() {
    let reps: i32 = 5;
    println!("Reps: {}", reps);
}
```

Don't worry about `i32` and `&str` yet — you will learn about types in Chapter 0.4. The point here is that the compiler told you *exactly* what was wrong and *exactly* where.

### Error 3: The undefined variable

```rust
fn main() {
    let exercise = "Deadlift";
    println!("Exercise: {}", exercize);
}
```

Compiler output:

```text
error[E0425]: cannot find value `exercize` in this scope
 --> src/main.rs:3:29
  |
3 |     println!("Exercise: {}", exercize);
  |                               ^^^^^^^^ help: a local variable with a similar name exists: `exercise`
```

| Part | Meaning |
|------|---------|
| `cannot find value 'exercize'` | You used a name that does not exist |
| `help: a local variable with a similar name exists: 'exercise'` | Rust noticed your typo and suggested the correct name |

**The fix:** Correct the spelling:

```rust
fn main() {
    let exercise = "Deadlift";
    println!("Exercise: {}", exercise);
}
```

The Rust compiler caught a *typo* and suggested the right word. Most compilers do not do this. Rust's error messages are famously helpful — learn to read them and they become your best debugging tool.

### The error-reading recipe

Every time you see a compiler error, follow these four steps:

1. **Read the first line.** It tells you *what* kind of error (missing semicolon, wrong type, unknown name, etc.)
2. **Look at the file and line number.** Open that file, go to that line.
3. **Read the arrows and highlights.** They show *exactly* which characters are wrong.
4. **Read the `help:` line.** If there is one, it often contains the exact fix.

Do not skip to step 4. Understanding *what* went wrong (steps 1-3) is how you learn. Just applying the suggested fix without understanding teaches you nothing.

---

## The Debugging Mindset

Errors are inevitable. Even programmers with 30 years of experience get compiler errors every single day. The difference between a beginner and an expert is not the number of errors — it is *how they respond to errors*.

### What beginners do (and why it doesn't work)

1. See a red error message
2. Panic
3. Change something random
4. Compile again
5. See a *different* error
6. Panic harder
7. Change something else random
8. Repeat until frustrated

This is called **random debugging**, and it almost never works. Each random change can introduce a *new* bug, so you end up with more errors than you started with.

### What programmers do

1. **Read the error.** The whole thing. Every word.
2. **Isolate the problem.** Which line? Which word on that line? What was Rust expecting versus what it found?
3. **Form a hypothesis.** "I think the problem is a missing semicolon on line 7."
4. **Test the hypothesis.** Make *one* change. Compile again. Did the error go away?
5. **If it did not work,** read the *new* error message. It might be a different problem, or your fix might have been close but not quite right. Go back to step 1.

The key principle: **change one thing at a time.** If you change three things at once and the error goes away, you do not know which change fixed it — and you will not learn anything.

### A real debugging session

Let's say you write this program:

```rust
fn main() {
    println!("Today's WOD: Fran")
    println!("21-15-9 Thrusters and Pull-ups");
    println!("Target time: sub-5 minutes")
}
```

You run `cargo run` and see an error about a missing semicolon. Following the method:

1. **Read:** "expected `;`, found `println`" on line 3
2. **Isolate:** Line 2 is missing a semicolon at the end
3. **Hypothesis:** Adding `;` after the closing `)` on line 2 will fix it
4. **Test:** Add the semicolon, run `cargo run` again

New error: missing semicolon on line 4. Same problem, same fix. Add it.

Now `cargo run` works:

```text
Today's WOD: Fran
21-15-9 Thrusters and Pull-ups
Target time: sub-5 minutes
```

Two errors. Two calm fixes. Total time: 30 seconds. No panic required.

---

## Exercises

### Exercise 1: Write Pseudocode for a Workout Tracker

**Goal:** Practice breaking a problem into steps *before* writing any code.

**Instructions:**

Open a text file (or use pen and paper). Write pseudocode — plain English instructions, numbered — for a program that does the following:

1. Stores the name of an exercise
2. Stores the number of reps completed
3. Stores the weight used (in kilograms)
4. Prints a summary line like: "You did 10 reps of Bench Press at 60 kg"

Remember the Input-Process-Output pattern:
- What are the **inputs**? (What data does the program need?)
- What is the **processing**? (What does the program do with the data?)
- What is the **output**? (What does the program display?)

You do not need to write any Rust code. Just the plan.

<details>
<summary>Hint 1</summary>

Start by listing the three inputs on separate lines:

```text
1. Set the exercise name to ...
2. Set the reps to ...
3. Set the weight to ...
```

</details>

<details>
<summary>Hint 2</summary>

The "processing" here is simple — there is no calculation. The program just *combines* the inputs into a sentence. Your pseudocode might just say:

```text
4. Print a summary with the exercise name, reps, and weight
```

</details>

<details>
<summary>Solution</summary>

```text
PROGRAM: Workout Summary

1. Set the exercise name to "Bench Press"
2. Set the number of reps to 10
3. Set the weight to 60 kg
4. Print: "You did [reps] reps of [exercise name] at [weight] kg"
```

There is no single correct answer — your pseudocode might use different words. The key things to check:

- Did you identify the three inputs?
- Did you describe the output clearly?
- Are the steps in a logical order (set values before using them)?

</details>

---

### Exercise 2: From Pseudocode to Rust

**Goal:** Turn your pseudocode into a working Rust program using only `println!` (with hardcoded values, since we have not learned variables yet).

**Instructions:**

1. Create a new project: `cargo new workout_summary`
2. Open `src/main.rs`
3. Replace the contents with `println!` statements that produce this output:

```text
=== Workout Summary ===
Exercise: Bench Press
Reps: 10
Weight: 60 kg
You did 10 reps of Bench Press at 60 kg
========================
```

4. Run it with `cargo run` and confirm the output matches

<details>
<summary>Hint 1</summary>

You need one `println!` statement for each line of output. The `===` lines are just decoration — use `println!("========================");` or similar.

</details>

<details>
<summary>Hint 2</summary>

Remember that every `println!` statement needs:
- An exclamation mark after `println`
- Parentheses around the text
- Double quotes around the string
- A semicolon at the end

</details>

<details>
<summary>Solution</summary>

```rust
fn main() {
    println!("=== Workout Summary ===");
    println!("Exercise: Bench Press");
    println!("Reps: 10");
    println!("Weight: 60 kg");
    println!("You did 10 reps of Bench Press at 60 kg");
    println!("========================");
}
```

Run it:

```bash
cd workout_summary
cargo run
```

Expected output:

```text
=== Workout Summary ===
Exercise: Bench Press
Reps: 10
Weight: 60 kg
You did 10 reps of Bench Press at 60 kg
========================
```

Yes, the values are hardcoded — we are repeating "Bench Press" and "10" in multiple places. That is annoying, and it is exactly *why* we need variables (Chapter 0.4). But for now, this works.

</details>

---

### Exercise 3: Bug Fixing Challenge

**Goal:** Practice reading compiler errors and fixing bugs methodically.

For each broken program below: **read the error message**, **explain what is wrong in your own words**, and **fix the code**. Do not just look at the solution — try to fix it yourself first.

#### Bug 1: The Easy One

```rust
fn main() {
    println!("Exercise: Push-ups")
    println!("Reps: 20");
}
```

Run this with `cargo run`. Read the error. Fix it.

<details>
<summary>Hint</summary>

Look carefully at line 2. What is missing at the end?

</details>

<details>
<summary>Solution</summary>

**The error:**
```text
error: expected `;`, found `println`
 --> src/main.rs:3:5
  |
2 |     println!("Exercise: Push-ups")
  |                                   ^ help: add `;` here
```

**What is wrong:** Line 2 is missing a semicolon at the end. In Rust, every statement must end with `;`.

**The fix:**
```rust
fn main() {
    println!("Exercise: Push-ups");
    println!("Reps: 20");
}
```

</details>

---

#### Bug 2: The Spelling One

```rust
fn main() {
    prinltn!("3 rounds for time:");
    println!("  400m Run");
    println!("  21 Kettlebell Swings");
    println!("  12 Pull-ups");
}
```

Run this with `cargo run`. Read the error. Fix it.

<details>
<summary>Hint</summary>

Read the first line very carefully, character by character. Compare it to the `println!` on the other lines.

</details>

<details>
<summary>Solution</summary>

**The error:**
```text
error: cannot find macro `prinltn` in this scope
 --> src/main.rs:2:5
  |
2 |     prinltn!("3 rounds for time:");
  |     ^^^^^^^ help: a macro with a similar name exists: `println`
```

**What is wrong:** `prinltn!` is misspelled. The letters "l" and "t" are swapped. It should be `println!`.

**The fix:**
```rust
fn main() {
    println!("3 rounds for time:");
    println!("  400m Run");
    println!("  21 Kettlebell Swings");
    println!("  12 Pull-ups");
}
```

Rust even suggested the correct name. Always read the `help:` line.

</details>

---

#### Bug 3: The Tricky One

```rust
fn main() {
    println!("=== Today's WOD ===");
    println!("AMRAP 12 minutes:");
    println!("  5 Deadlifts (100 kg)");
    println!("  10 Box Jumps (24")");
    println!("  15 Wall Balls (9 kg)");
}
```

Run this with `cargo run`. This one produces a more confusing error. Read it carefully.

<details>
<summary>Hint 1</summary>

Look at line 5. There is a `"` character inside the string that Rust thinks is the *end* of the string. The problem is the inch mark in `24"`.

</details>

<details>
<summary>Hint 2</summary>

To include a double-quote character inside a string, you need to *escape* it with a backslash: `\"`. So `24"` inside a string becomes `24\"`.

</details>

<details>
<summary>Solution</summary>

**The error:**
```text
error: unexpected closing delimiter: `)`
 --> src/main.rs:5:38
  |
5 |     println!("  10 Box Jumps (24")");
  |                                  ^ unexpected closing delimiter
```

**What is wrong:** The `"` after `24` ends the string early. Rust then sees `)");` which makes no sense. The double-quote that is meant to represent inches is being interpreted as the end of the string.

**The fix:** Escape the inner double-quote with a backslash:

```rust
fn main() {
    println!("=== Today's WOD ===");
    println!("AMRAP 12 minutes:");
    println!("  5 Deadlifts (100 kg)");
    println!("  10 Box Jumps (24\")");
    println!("  15 Wall Balls (9 kg)");
}
```

Alternatively, you could avoid the issue by writing the measurement differently:

```rust
    println!("  10 Box Jumps (24 in)");
```

This is a common real-world bug. Whenever you need to put special characters inside a string — like `"`, `\`, or `{` — you may need to escape them. You will learn more about this in later chapters.

</details>

---

## What You've Learned

- **Breaking problems into steps** is the first and most important programming skill. Before you write code, you need a plan.
- **Every program is Input, Processing, Output.** Once you identify these three parts, the structure of the program becomes clear.
- **Pseudocode** lets you think about *what* the program should do without worrying about syntax. Write the plan first, then translate to code.
- **Rust's compiler errors are your friend.** They tell you what is wrong, where it is wrong, and often how to fix it. Read the whole message.
- **Debug methodically.** Read the error, isolate the problem, form a hypothesis, test one change at a time. Never guess randomly.

---

## What's Next

You now know how to *think* about programs. In [Chapter 0.4: The Building Blocks](ch00-4-building-blocks.md), you will learn the core Rust tools that turn your thinking into real, working code: **variables** to store data, **functions** to organize logic, and **loops** to repeat actions. By the end of that chapter, you will write a complete workout volume calculator from scratch — your first real program that actually *computes* something.
