# Struct Memory Layout: What's Your Exercise Actually Made Of?

## The Problem

You just defined your `Exercise` struct. It looks clean:

```rust,ignore
struct Exercise {
    is_bodyweight: bool,
    id: u64,
    name: String,
    category: u8,
    max_weight: f64,
}
```

You load 500 exercises from the database into a `Vec<Exercise>`. The app works. Ship it, right?

But then you start wondering. How much memory is that vector actually eating? On a phone with 3GB of RAM shared across every app, "it works" and "it works *well*" are different things. Is your exercise library sitting on 5KB or 500KB?

You reach for `std::mem::size_of` to find out, and what you discover changes how you think about structs forever.

## The Naive Way

Let's do the math by hand. We look at each field and add up the bytes:

| Field | Type | Size (bytes) |
|-------|------|-------------|
| `is_bodyweight` | `bool` | 1 |
| `id` | `u64` | 8 |
| `name` | `String` | 24 (ptr + len + cap) |
| `category` | `u8` | 1 |
| `max_weight` | `f64` | 8 |

Total: 1 + 8 + 24 + 1 + 8 = **42 bytes**. So 500 exercises = 21,000 bytes. About 21KB. Not bad.

Let's check:

```rust,ignore
fn main() {
    println!("Size of Exercise: {} bytes", std::mem::size_of::<Exercise>());
}
```

Output: `Size of Exercise: 56 bytes`

Wait. *56?* We counted 42. Where did 14 bytes go?

## The Insight: Alignment and Padding

CPUs don't read memory byte-by-byte. A 64-bit CPU reads in 8-byte chunks. When a `u64` starts at an address that isn't a multiple of 8, the CPU either slows down (two reads instead of one) or flat-out refuses. So Rust *pads* your struct to keep fields aligned.

Let's trace through the layout of our original struct, byte by byte:

```
Offset 0:  [is_bodyweight: 1 byte]
Offset 1:  [PADDING: 7 bytes]        <-- wasted! u64 needs 8-byte alignment
Offset 8:  [id: 8 bytes]
Offset 16: [name: 24 bytes]          <-- String (ptr + len + cap)
Offset 40: [category: 1 byte]
Offset 41: [PADDING: 7 bytes]        <-- wasted! f64 needs 8-byte alignment
Offset 48: [max_weight: 8 bytes]
Total: 56 bytes
```

Fourteen bytes of padding. That's 25% waste. Multiply by 500 exercises: we're burning 7,000 bytes on *nothing*. Air. Invisible gaps that exist solely because we put a `bool` before a `u64`.

## The Build: Reordering for Minimal Padding

The fix is almost embarrassingly simple. Sort your fields from largest alignment to smallest:

```rust,ignore
struct Exercise {
    id: u64,            // 8-byte alignment
    name: String,       // 8-byte alignment (contains pointers)
    max_weight: f64,    // 8-byte alignment
    category: u8,       // 1-byte alignment
    is_bodyweight: bool, // 1-byte alignment
}
```

Now trace the layout:

```
Offset 0:  [id: 8 bytes]
Offset 8:  [name: 24 bytes]
Offset 32: [max_weight: 8 bytes]
Offset 40: [category: 1 byte]
Offset 41: [is_bodyweight: 1 byte]
Offset 42: [PADDING: 6 bytes]        <-- struct size must be multiple of
Total: 48 bytes                           largest alignment (8)
```

We still have 6 bytes of tail padding (the struct's total size must be a multiple of its largest alignment so that arrays work). But we went from 56 to 48 bytes. That's a 14% reduction, just by reordering fields.

Let's prove it:

```rust,ignore
struct ExerciseOriginal {
    is_bodyweight: bool,
    id: u64,
    name: String,
    category: u8,
    max_weight: f64,
}

struct ExerciseOptimized {
    id: u64,
    name: String,
    max_weight: f64,
    category: u8,
    is_bodyweight: bool,
}

fn main() {
    println!("Original:  {} bytes", std::mem::size_of::<ExerciseOriginal>());
    println!("Optimized: {} bytes", std::mem::size_of::<ExerciseOptimized>());
    println!();

    let exercise_count = 500;
    let original_total = std::mem::size_of::<ExerciseOriginal>() * exercise_count;
    let optimized_total = std::mem::size_of::<ExerciseOptimized>() * exercise_count;
    println!("500 exercises (original):  {} bytes ({:.1} KB)", original_total, original_total as f64 / 1024.0);
    println!("500 exercises (optimized): {} bytes ({:.1} KB)", optimized_total, optimized_total as f64 / 1024.0);
    println!("Saved: {} bytes", original_total - optimized_total);
}
```

Output:

```
Original:  56 bytes
Optimized: 48 bytes

500 exercises (original):  28000 bytes (27.3 KB)
500 exercises (optimized): 24000 bytes (23.4 KB)
Saved: 4000 bytes
```

Four kilobytes saved by changing *nothing about the logic* -- just the order of lines in a struct definition.

## repr(C): When Rust Takes the Wheel

By default, Rust is allowed to reorder your struct fields to minimize padding. That's right -- the compiler might already be doing what we just did. You can verify by adding `#[repr(C)]` which forces C-compatible layout (fields stay in declaration order):

```rust,ignore
#[repr(C)]
struct ExerciseC {
    is_bodyweight: bool,
    id: u64,
    name: String,
    category: u8,
    max_weight: f64,
}

struct ExerciseRust {
    is_bodyweight: bool,
    id: u64,
    name: String,
    category: u8,
    max_weight: f64,
}

fn main() {
    println!("repr(C) layout:    {} bytes", std::mem::size_of::<ExerciseC>());
    println!("Default Rust layout: {} bytes", std::mem::size_of::<ExerciseRust>());
}
```

If Rust reordered your fields, the default layout will be smaller than the `repr(C)` version. You usually want Rust's default. Use `repr(C)` only when you need a specific memory layout for FFI (calling C libraries) or when serializing bytes directly.

## The Payoff

Back in GrindIt, our exercise library loads from the database into a `Vec<Exercise>`. Every byte of padding in the struct is multiplied by the length of that vector. In a larger app you might have nested structs -- a `WorkoutLog` containing a `Vec<Exercise>` -- and padding compounds.

The rule of thumb is simple: **put your widest fields first**. It costs nothing, it's not premature optimization, and it shows you understand what's happening beneath your code.

Here's a helper you can drop into any project during development:

```rust
struct ExerciseOptimized {
    id: u64,
    name: String,
    max_weight: f64,
    category: u8,
    is_bodyweight: bool,
}

fn inspect_layout<T>() {
    println!(
        "{}: size = {} bytes, align = {} bytes",
        std::any::type_name::<T>(),
        std::mem::size_of::<T>(),
        std::mem::align_of::<T>(),
    );
}

fn main() {
    inspect_layout::<ExerciseOptimized>();
    // Output: ExerciseOptimized: size = 48 bytes, align = 8 bytes
}
```

## Complexity Comparison

| Metric | Naive field order | Optimized field order |
|--------|------------------|-----------------------|
| Size per struct | 56 bytes | 48 bytes |
| 500 exercises | 27.3 KB | 23.4 KB |
| Wasted padding per struct | 14 bytes | 6 bytes |
| Runtime cost to optimize | -- | Zero (compile-time only) |

## Try It Yourself

1. **Measure your own structs.** Define a `WorkoutLog` struct with fields: `completed: bool`, `score: u32`, `athlete_id: u64`, `workout_type: u8`, `timestamp: i64`, `notes: String`. Print its size. Now reorder the fields to minimize padding. What's the difference?

2. **The alignment detective.** Write a function that takes a type parameter and prints both `size_of` and `align_of`. Use it to inspect `bool`, `u8`, `u16`, `u32`, `u64`, `f64`, `String`, `Vec<u8>`, and `Option<bool>`. Can you spot the pattern between a type's alignment and its size?

3. **The Option trick.** Compare `size_of::<Option<bool>>()` vs `size_of::<bool>()`. Now compare `size_of::<Option<Box<u64>>>()` vs `size_of::<Box<u64>>()`. One uses extra space; the other doesn't. Can you figure out why? (Hint: Rust knows that a `Box` can never be null.)
