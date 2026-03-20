# Introduction

Welcome to **GrindIt** — a hands-on guide that teaches you Rust, full-stack web development, data structures & algorithms, and system design by building a real production fitness tracker from scratch.

By the end of this book, you will have built a complete application with:

- **Leptos 0.8** — A reactive Rust web framework (SSR + WASM hydration)
- **Axum 0.8** — A fast, ergonomic HTTP server
- **SQLx + PostgreSQL** — Type-safe, compile-time verified database queries
- **Authentication** — Google OAuth, email/password (Argon2), phone OTP
- **PWA** — Installable, offline-capable progressive web app
- **REST API** — OpenAPI-documented endpoints with Swagger UI
- **Docker** — Multi-stage production builds
- **CI/CD** — GitHub Actions for testing, linting, and deployment

...and you'll have practiced **26 DSA patterns** and **6 system design topics** — all in the context of real code you wrote.

---

## The Triple Goal

This book serves three purposes simultaneously:

1. **Build production Rust** — Not toy examples. You'll build a real app that handles auth, file uploads, complex data models, and deployment.

2. **Learn DSA organically** — Every data structure and algorithm you encounter arises naturally from a feature you're building. HashMap for exercise grouping. Greedy algorithms for streak calculation. N-ary trees for WOD sections. Then capstone chapters tackle the harder patterns (DP, tries, heaps) using your app's domain.

3. **Prepare for interviews** — System Design Corners frame every architecture decision as an interview talking point. The capstone includes full mock interviews. You'll walk into interviews with concrete examples from code you built.

---

## Choose Your Track

This book has two tracks. Both build the same app, both cover the same features. The difference is in how concepts are explained.

### Beginner Track (Start at Part 0)

**For you if:** You've never written code before, or you've only done very basic scripting.

- Starts with **Part 0: Programming Fundamentals** — what is a terminal, what is code, how to think like a programmer
- Every Rust concept is explained from scratch
- **"Programming Concept"** boxes explain fundamentals (what is a variable? what is a function?)
- More screenshots, more expected-output blocks, simpler Rust Gym drills
- No assumptions about prior knowledge

### Experienced Track (Start at Chapter 1)

**For you if:** You program in JavaScript, Python, Go, Java, or another language and want to learn Rust.

- Skips Part 0 entirely — you already know what a variable is
- Concise explanations focused on what's *different* about Rust
- **"Coming from JS/Python/Go?"** comparison boxes with side-by-side code
- Interview-adjacent Rust Gym drills
- Assumes you can read code and debug independently

**Both tracks converge** at the Design Reflection chapter (18.5) and Capstone chapters (19-21), which are shared.

---

## How This Book Works

### Feature-Driven Learning

Each chapter builds **one complete feature** of the app — from database to UI. You don't learn Rust concepts in isolation and then try to apply them. You build something real, and learn the concepts because you *need* them.

### Spotlight + Reps

Each chapter has:

- **One Spotlight concept** — the Rust concept taught in depth (e.g., "Error Handling" in Ch 4)
- **Supporting concepts** — used but not deeply explained yet, marked with 💡 "Spotlighted in Ch X"
- **Rust Gym drills** — 2-3 isolated exercises after the feature, drilling only the spotlight concept

Every concept is seen at least 3 times: first exposure → spotlight deep dive → reinforcement in later chapters.

### Recurring Sections

Throughout the chapters, you'll encounter these callout boxes:

> **Coming from JS/Python/Go?** (Experienced track)
> Side-by-side code comparisons showing how the concept maps to languages you know.

> **Programming Concept** (Beginner track)
> Foundational explanations of programming concepts for absolute beginners.

> **📊 DSA in Context**
> Connects the code you just wrote to an interview pattern. Includes a bonus challenge.

> **🏗️ System Design Corner**
> Frames the architecture decision as an interview talking point with key discussion bullets.

> **📐 Design Insight** (Ousterhout)
> Connects the code to a principle from *A Philosophy of Software Design* — deep modules, information hiding, defining errors out of existence, etc.

### Exercises

Each exercise follows this format:

1. **Goal** — what you're building
2. **Instructions** — step-by-step directions
3. **Hints** — progressively more specific (collapsed, click to reveal)
4. **Solution** — complete working code

Try each exercise yourself before looking at hints or solutions. Type the code — don't copy-paste. Typing doubles retention.

---

## Prerequisites

### Beginner Track
- A computer (macOS, Linux, or Windows with WSL)
- Curiosity and patience
- That's it. Part 0 teaches everything else.

### Experienced Track
- Comfortable in at least one programming language
- Basic terminal/command line usage
- Familiarity with concepts like variables, functions, loops, and HTTP

### Both Tracks (installed during Chapter 1)
- [Rust](https://rustup.rs/) (installed via rustup)
- [Docker](https://www.docker.com/get-started/) (for PostgreSQL)
- A code editor (VS Code with [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) recommended)

---

## The Reference Implementation

This book builds the GrindIt app from scratch. A complete, working version exists at:

**[github.com/sivakarasala/gritwit](https://github.com/sivakarasala/gritwit)** (tag: `book-v1`)

You can reference this codebase at any point to see the "finished product." Each chapter notes which files in the reference implementation correspond to what you're building.

---

## Let's Build

If you're a **beginner**, start with [Part 0: Your Workshop](part-0-foundations/ch00-1-your-workshop.md).

If you're an **experienced programmer**, jump to [Chapter 1: Hello, GrindIt!](experienced/ch01-hello-grindit.md).

Let's go.
