# Your First Program — What Is Code?

In the last chapter, you set up your workshop: a terminal, a code editor, and a workspace folder. Now it is time to use them.

By the end of this chapter, you will have installed the Rust programming language, created your first project, and run a program that you wrote. You will also understand — at a real level, not a hand-wavy one — what happened when you ran it.

---

## What Is a Programming Language?

You speak English (or another human language) to communicate with people. But computers do not understand English. Deep down, a computer only understands **binary** — long strings of ones and zeros, like `01001000 01101001`. That is how a computer says "Hi."

Nobody wants to write in ones and zeros. So people invented **programming languages** — languages that humans can read and write, which get translated into binary for the computer. A programming language is the meeting point between your brain and the machine.

Here is how that translation works:

```
You write code          The compiler translates it          The computer runs it
(human-readable)   -->  (machine-readable binary)     -->  (things happen!)
```

There are hundreds of programming languages, each with different strengths. Python is popular for data science. JavaScript runs in web browsers. Swift is used for iPhone apps.

We are going to learn **Rust**.

---

## Why Rust?

You might wonder: if there are hundreds of programming languages, why learn Rust?

Three reasons:

1. **It is fast.** Rust programs run about as fast as programs written in C or C++, which are the languages used to build operating systems, game engines, and web browsers. When we build GrindIt, it will be snappy.

2. **It catches your mistakes early.** Rust has a **compiler** (more on this soon) that checks your code before it runs. If you made a mistake, Rust tells you *before* your program crashes — not after. Many languages let bugs sneak through and only fail when a real user is affected. Rust does not.

3. **It teaches you to think clearly.** Rust requires you to be precise about what your code does. This can feel strict at first, but it builds habits that make you a better programmer in *any* language.

You do not need to understand all of that right now. What matters is this: Rust is a language that respects your time. It works hard to help you write correct programs from the start.

---

## Installing Rust

Let's install Rust on your computer. Open your terminal and run this command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

That is a long command, so let's break it down:

- **`curl`** is a tool that downloads things from the internet.
- The long URL (`https://sh.rustup.rs`) is a script written by the Rust team that installs Rust.
- The `|` symbol (called a **pipe**) takes the downloaded script and feeds it to `sh`, which runs it.

In short: *"Download the Rust installer and run it."*

When the installer starts, it will ask you a question:

```
1) Proceed with standard installation (default - just press enter)
2) Customize installation
3) Cancel installation
```

Press **Enter** to choose the default. The installer will download and set up everything you need. This may take a minute or two.

When it finishes, you will see:

```
Rust is installed now. Great!
```

Now, close your terminal and open a new one. (This is necessary so your terminal picks up the new Rust tools.) Then verify the installation:

```bash
rustc --version
```

You should see something like:

```
rustc 1.83.0 (90b35a623 2024-11-26)
```

The exact numbers will differ — that is fine. What matters is that you see a version number, not an error.

Also try:

```bash
cargo --version
```

```
cargo 1.83.0 (5ffbef321 2024-10-29)
```

**`rustc`** is the Rust **compiler** — the program that translates your code into something the computer can run. **`cargo`** is Rust's **build tool and package manager** — we will talk about it more at the end of this chapter. For now, just know that if both commands showed version numbers, Rust is installed and ready.

---

## Your First Program

Here is the moment. Let's create and run a Rust program.

In your terminal, navigate to your workspace:

```bash
cd ~/rusty
```

Now use Cargo to create a new project:

```bash
cargo new hello_grindit
```

You should see:

```
    Creating binary (application) `hello_grindit` package
```

Cargo just created a folder called `hello_grindit` with everything you need to get started. Let's go inside:

```bash
cd hello_grindit
```

Now, run the program:

```bash
cargo run
```

You will see some output as Rust compiles your code, and then:

```
   Compiling hello_grindit v0.1.0 (/Users/yourname/rusty/hello_grindit)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.73s
     Running `target/debug/hello_grindit`
Hello, world!
```

There it is. **Hello, world!** Your first program just ran.

You did not write the code yet — Cargo generated a starter program for you. But it is *your* project now, and in a moment you are going to change it. First, let's understand what Cargo created.

---

## Anatomy of a Rust Program

Open your project in VS Code:

```bash
code ~/rusty/hello_grindit
```

In the left sidebar, you will see this file structure:

```
hello_grindit/
├── Cargo.toml
└── src/
    └── main.rs
```

There are two important files. Let's start with the one that matters most.

### `src/main.rs`

Click on `src/main.rs` in VS Code. You will see:

```rust
fn main() {
    println!("Hello, world!");
}
```

Three lines. That is the entire program. Let's go through it piece by piece.

---

**`fn main() {`**

- **`fn`** is short for **function**. A function is a named block of instructions. Think of it like a chapter in a recipe: "Chapter: Make the Sauce" groups together all the sauce-making steps.
- **`main`** is the name of this function. The name `main` is special in Rust — it is the **entry point** of your program. When you run a Rust program, the computer starts by looking for a function called `main` and runs whatever is inside it.
- **`()`** after the name is where you would list any inputs the function needs. `main` does not need any inputs, so the parentheses are empty.
- **`{`** is an opening curly brace. It marks the **beginning** of the function's body — the instructions inside it.

---

**`println!("Hello, world!");`**

- **`println!`** is a **macro** that prints text to the terminal. (A macro is like a special function — the `!` at the end is how you know it is a macro, not a regular function. Don't worry about the difference yet.)
- **`"Hello, world!"`** is a **string** — a piece of text. The double quotes tell Rust *"this is text, not code."*
- **`;`** is a **semicolon**. In Rust, most lines of code end with a semicolon. It is like the period at the end of a sentence — it tells Rust *"this instruction is complete."*

So this line means: *"Print the text 'Hello, world!' to the terminal, then move to a new line."* (The `ln` in `println` stands for "line" — it adds a new line after printing.)

---

**`}`**

This closing curly brace marks the **end** of the `main` function. Everything between `{` and `}` is the body of the function.

---

Putting it all together in plain English:

> *"Here is a function called `main`. When the program starts, print 'Hello, world!' to the terminal."*

That is it. That is your whole program.

---

## The Compiler

When you ran `cargo run`, something important happened before your program printed "Hello, world!" — Rust **compiled** your code.

**Compilation** is the process of translating your human-readable code into machine-readable binary (those ones and zeros we talked about earlier). The program that does this translation is called a **compiler**.

Here is what happened step by step:

1. You typed `cargo run`.
2. Cargo looked at your `main.rs` file and handed it to the Rust compiler (`rustc`).
3. The compiler read your code, checked it for mistakes, and translated it into a binary file (an **executable**) that your computer can run directly.
4. Cargo then ran that executable, and "Hello, world!" appeared on your screen.

The compiler is your **strictest teacher**. Before it translates your code, it examines every line and checks for problems:

- Did you forget a semicolon?
- Did you misspell something?
- Are you using a variable that does not exist?
- Could your code crash under certain conditions?

If it finds *any* problem, it **refuses to compile** your code and instead gives you an error message explaining what went wrong and where. This might sound annoying, but it is actually a superpower. It means that if your code compiles, a whole category of bugs has already been eliminated.

Some languages (like Python and JavaScript) are **interpreted** — they run your code line by line without a separate compilation step. This is convenient, but it means mistakes only show up when the program is running. Rust's approach means you catch problems before your code ever runs. It is like having a coach who reviews your workout form *before* you add heavy weight — it prevents injuries.

---

## Cargo

We have been using a tool called **Cargo** without fully explaining it. Let's fix that.

**Cargo** is Rust's official build tool and package manager. It does several jobs:

| Command | What it does |
|---------|-------------|
| `cargo new project_name` | Creates a new Rust project with the right folder structure |
| `cargo run` | Compiles your code *and* runs the result |
| `cargo build` | Compiles your code without running it (useful when you just want to check for errors) |
| `cargo check` | Checks your code for errors even faster than `cargo build` (does not produce a binary) |

You will use `cargo run` the most. Think of Cargo as your project manager — it organizes your project, fetches any libraries you need, and makes sure everything builds correctly.

### `Cargo.toml`

Open `Cargo.toml` in VS Code. You will see something like:

```toml
[package]
name = "hello_grindit"
version = "0.1.0"
edition = "2021"

[dependencies]
```

This is your project's **configuration file**. Let's break it down:

- **`[package]`** — This section describes your project.
  - **`name`** — The name of your project. Cargo set this to `hello_grindit` because that is what you typed in `cargo new`.
  - **`version`** — Your project's version number. `0.1.0` means "this is the very first version, and it is not finished yet."
  - **`edition`** — Which edition of the Rust language to use. Don't worry about this for now.
- **`[dependencies]`** — This is where you list other people's code that your project uses (called **libraries** or **crates** in Rust). It is empty because our tiny program does not need anything beyond what Rust provides built-in.

You rarely edit `Cargo.toml` by hand this early on. But it is good to know what it is — the ID card for your project.

---

## Exercises

### Exercise 1: Install Rust

**Goal:** Install Rust and verify that both `rustc` and `cargo` are available.

**Instructions:**
1. Open your terminal.
2. Run the Rust installer:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3. Press Enter when prompted to choose the default installation.
4. Close your terminal and open a new one.
5. Verify the installation:
   ```bash
   rustc --version
   cargo --version
   ```

<details>
<summary>Hints</summary>

- If you see `command not found` after installing, make sure you opened a **new** terminal window. The old window does not know about the new tools yet.
- On some systems, you may need to run `source ~/.cargo/env` to make the tools available without restarting the terminal.
- If you are on macOS and see a popup about "developer tools," click **Install**. This installs some tools that Rust needs.

</details>

<details>
<summary>Solution</summary>

You should see output like this (your version numbers will likely be different):

```
$ rustc --version
rustc 1.83.0 (90b35a623 2024-11-26)

$ cargo --version
cargo 1.83.0 (5ffbef321 2024-10-29)
```

If you see version numbers for both commands, Rust is installed correctly. The exact numbers do not matter — any recent version will work for this book.

</details>

---

### Exercise 2: Your First GrindIt Message

**Goal:** Create a new Rust project and change it to print a fitness-themed message.

**Instructions:**
1. Navigate to your workspace: `cd ~/rusty`
2. Create a new project: `cargo new hello_grindit`
   (If you already created it earlier in this chapter, you can skip this step.)
3. Open `src/main.rs` in VS Code (or use `code ~/rusty/hello_grindit`).
4. Change the `println!` line so it prints: `Welcome to GrindIt! Let's track some workouts.`
5. Save the file.
6. In your terminal, navigate to the project: `cd ~/rusty/hello_grindit`
7. Run it: `cargo run`

<details>
<summary>Hints</summary>

- The text you want to print goes inside the double quotes: `println!("your text here");`
- Make sure you keep the `!` after `println` — it is part of the syntax.
- Make sure the line ends with a semicolon `;`.
- If you see an error, read it carefully. The compiler will tell you exactly which line has the problem.

</details>

<details>
<summary>Solution</summary>

Your `src/main.rs` should look like this:

```rust
fn main() {
    println!("Welcome to GrindIt! Let's track some workouts.");
}
```

When you run `cargo run`, you should see:

```
   Compiling hello_grindit v0.1.0 (/Users/yourname/rusty/hello_grindit)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s
     Running `target/debug/hello_grindit`
Welcome to GrindIt! Let's track some workouts.
```

</details>

---

### Exercise 3: Add a Second Line

**Goal:** Make your program print two lines of text.

**Instructions:**
1. Open `src/main.rs` in VS Code.
2. Add a second `println!` line after the first one. Make it print: `Today's workout: 5 sets of 10 push-ups`
3. Save the file.
4. Run it: `cargo run`

<details>
<summary>Hints</summary>

- Each `println!` call prints one line of text. To print two lines, you need two `println!` calls.
- Both lines go inside the curly braces `{ }` of the `main` function.
- Both lines need their own semicolon at the end.
- Rust runs lines of code in order, from top to bottom — just like you read a book.

</details>

<details>
<summary>Solution</summary>

Your `src/main.rs` should look like this:

```rust
fn main() {
    println!("Welcome to GrindIt! Let's track some workouts.");
    println!("Today's workout: 5 sets of 10 push-ups");
}
```

When you run `cargo run`, you should see:

```
Welcome to GrindIt! Let's track some workouts.
Today's workout: 5 sets of 10 push-ups
```

Notice that each `println!` produced one line of output, and they appeared in order. That is how programs work — instructions execute from top to bottom, one at a time.

</details>

---

### Exercise 4: Break Things on Purpose

**Goal:** Learn to read compiler error messages by intentionally creating mistakes.

This is the most important exercise in this chapter. Errors are not failures — they are the compiler **helping you**. Every programmer encounters errors constantly. The skill is not avoiding errors; it is reading them calmly and understanding what they mean.

**Instructions:**

**Part A — Remove a semicolon:**
1. Open `src/main.rs`.
2. Delete the semicolon from the end of the first `println!` line.
3. Save the file.
4. Run `cargo run` and read the error message.
5. Fix the error by adding the semicolon back. Run `cargo run` again to confirm it works.

**Part B — Misspell `println`:**
1. Change `println!` to `printl!` (remove the last `n`).
2. Save and run `cargo run`.
3. Read the error message.
4. Fix the spelling. Run `cargo run` again.

**Part C — Remove a curly brace:**
1. Delete the closing `}` at the end of the file.
2. Save and run `cargo run`.
3. Read the error message.
4. Fix it by adding `}` back. Run `cargo run` again.

<details>
<summary>Hints</summary>

- When you see an error, look for the **line number**. The compiler tells you exactly where the problem is. For example, `--> src/main.rs:2:55` means "the problem is in `main.rs`, line 2, column 55."
- The compiler also often suggests a fix. Look for lines that say "help:" — these are the compiler's suggestions.
- Don't panic when you see an error. Read it slowly. The most important parts are: (1) what went wrong, and (2) where it happened.

</details>

<details>
<summary>Solution</summary>

**Part A** — Removing a semicolon gives an error like:

```
error: expected `;`
 --> src/main.rs:2:56
  |
2 |     println!("Welcome to GrindIt! Let's track some workouts.")
  |                                                                ^
  |                                                                help: add `;` here
```

The compiler tells you *exactly* what is missing and *exactly* where to put it. It even says "help: add `;` here." How nice is that?

**Part B** — Misspelling `println` gives:

```
error: cannot find macro `printl` in this scope
 --> src/main.rs:2:5
  |
2 |     printl!("Welcome to GrindIt! Let's track some workouts.");
  |     ^^^^^^
```

The compiler cannot find anything called `printl!` because it does not exist. It is telling you: *"I looked everywhere and couldn't find a macro with this name."* This is your cue to check for typos.

**Part C** — Removing the closing `}` gives:

```
error: this file contains an unclosed delimiter
 --> src/main.rs:4:1
  |
1 | fn main() {
  |           - unclosed delimiter
```

The compiler knows that `{` was opened on line 1 but never closed. It is like starting a parenthesis in English and never finishing it (which drives people crazy.

Add the `}` back, and all is well.

**The lesson:** The Rust compiler is not yelling at you. It is pointing at the problem, explaining what went wrong, and often telling you how to fix it. Treat error messages as helpful notes from a very thorough colleague.

</details>

---

## Summary

Here is what you accomplished in this chapter:

| What you did | Why it matters |
|-------------|---------------|
| Installed Rust with `rustup` | You now have the Rust compiler and Cargo on your machine |
| Created a project with `cargo new` | Cargo sets up the right structure so you can focus on code |
| Ran a program with `cargo run` | You went from source code to a running program |
| Read and understood `main.rs` | You know what `fn`, `main`, `println!`, `;`, and `{}` do |
| Learned what a compiler does | You understand that Rust checks your code before running it |
| Read compiler error messages | You know that errors are helpful, not scary |

You wrote your first Rust program. It was small — just a couple of `println!` calls — but every program in the world started with something this simple. The GrindIt app we are building will have hundreds of lines of code across many files, but it will still begin with `fn main()`.

In the next chapter, we will learn how to **think like a programmer** — how to take a big, fuzzy idea like "build a fitness app" and break it down into small, clear steps that a computer can follow.
