# Trait Objects & Vtables -- "The Workout Card and Its Instruction Sheet"

In Chapter 9 you wrote `Box<dyn ScoringStrategy>` and it felt magical -- you stored different scoring types in one Vec and called `.score()` on each, and the right implementation ran. But how does Rust know WHICH `.score()` to call at runtime? In C++ they call it a vtable. In Rust, it's a fat pointer -- two pointers for the price of one. One points to your data, the other to a table of function pointers. Let's crack open the vtable and see the machinery behind polymorphism.

Think of it this way. A **trait object** is a generic workout card. The card doesn't say what KIND of workout it is, but it has a standard format: "score this", "display this", "validate this". Any workout type -- ForTime, AMRAP, Strength -- can fill in the card. The **vtable** is the instruction sheet taped to the back of the card. When you flip the card over, it tells you: "for scoring, go to function at address X. For display, go to address Y." Different workout types have different instruction sheets, but they all follow the same format. And a **fat pointer** is you holding the card with both hands -- one hand holds the card itself (pointer to data), the other holds the instruction sheet (pointer to vtable).

---

## 1. Static Dispatch -- The Default (and Why It's Fast)

Let's start with the approach Rust uses by default. When you write a generic function, the compiler generates a separate copy for each concrete type:

```rust
trait ScoringStrategy {
    fn score(&self) -> f64;
    fn description(&self) -> String;
}

struct ForTimeScorer { time_seconds: u32 }
struct AmrapScorer { rounds: u32, extra_reps: u32 }
struct StrengthScorer { weight_lbs: f64 }

impl ScoringStrategy for ForTimeScorer {
    fn score(&self) -> f64 { self.time_seconds as f64 }
    fn description(&self) -> String {
        format!("For Time: {}s", self.time_seconds)
    }
}

impl ScoringStrategy for AmrapScorer {
    fn score(&self) -> f64 {
        self.rounds as f64 + (self.extra_reps as f64 / 100.0)
    }
    fn description(&self) -> String {
        format!("AMRAP: {} + {}", self.rounds, self.extra_reps)
    }
}

impl ScoringStrategy for StrengthScorer {
    fn score(&self) -> f64 { self.weight_lbs }
    fn description(&self) -> String {
        format!("Strength: {} lbs", self.weight_lbs)
    }
}

// Static dispatch -- one copy per type
fn display_score<T: ScoringStrategy>(strategy: &T) {
    println!("{}: {}", strategy.description(), strategy.score());
}

fn main() {
    let ft = ForTimeScorer { time_seconds: 185 };
    let amrap = AmrapScorer { rounds: 12, extra_reps: 5 };
    let strength = StrengthScorer { weight_lbs: 225.0 };

    display_score(&ft);      // calls display_score::<ForTimeScorer>
    display_score(&amrap);   // calls display_score::<AmrapScorer>
    display_score(&strength); // calls display_score::<StrengthScorer>
}
```

The compiler performs **monomorphization** -- it stamps out three concrete versions of `display_score`, one for each type. At every call site, the compiler knows EXACTLY which `.score()` to invoke. No lookup, no indirection, and the function can even be inlined.

The gym analogy: the gym has separate instruction manuals for ForTime, AMRAP, and Strength workouts. No looking anything up at runtime -- you grab the right manual directly off the shelf.

The trade-off? Your binary grows. Three types means three copies of the function. For a small trait with three implementors, that is trivial. For a trait with 50 methods and 20 implementors, it adds up.

---

## 2. Dynamic Dispatch -- When You Don't Know the Type at Compile Time

Now change one character. Replace the generic `<T: ScoringStrategy>` with `dyn ScoringStrategy`:

```rust
# trait ScoringStrategy {
#     fn score(&self) -> f64;
#     fn description(&self) -> String;
# }
# struct ForTimeScorer { time_seconds: u32 }
# struct AmrapScorer { rounds: u32, extra_reps: u32 }
# impl ScoringStrategy for ForTimeScorer {
#     fn score(&self) -> f64 { self.time_seconds as f64 }
#     fn description(&self) -> String { format!("For Time: {}s", self.time_seconds) }
# }
# impl ScoringStrategy for AmrapScorer {
#     fn score(&self) -> f64 { self.rounds as f64 + (self.extra_reps as f64 / 100.0) }
#     fn description(&self) -> String { format!("AMRAP: {} + {}", self.rounds, self.extra_reps) }
# }
// Dynamic dispatch -- ONE copy, works with any implementor
fn display_score_dyn(strategy: &dyn ScoringStrategy) {
    println!("{}: {}", strategy.description(), strategy.score());
}

fn main() {
    let ft = ForTimeScorer { time_seconds: 185 };
    let amrap = AmrapScorer { rounds: 12, extra_reps: 5 };

    display_score_dyn(&ft);    // same function, different vtable
    display_score_dyn(&amrap); // same function, different vtable
}
```

ONE copy of the function exists in the binary. At runtime, Rust follows a pointer to find which `.score()` to call. The cost is a single pointer dereference -- roughly one nanosecond.

When do you NEED dynamic dispatch? Three cases come up constantly:

- **Heterogeneous collections**: `Vec<Box<dyn ScoringStrategy>>` -- different types in one container.
- **Plugin-style extensibility**: a library defines a trait, users provide implementations.
- **Returning different types**: a function that returns `Box<dyn ScoringStrategy>` based on runtime config.

---

## 3. What a Trait Object Actually Is -- The Fat Pointer

Here is the key insight. A regular reference `&ForTimeScorer` is one pointer -- 8 bytes on a 64-bit machine. It points to the data, and the compiler already knows the type (and therefore which methods to call).

A trait object `&dyn ScoringStrategy` is TWO pointers -- 16 bytes. It must carry extra information because the compiler has erased the concrete type.

```
Regular reference (&ForTimeScorer):
  ┌──────────────┐
  │ data pointer  │  ← 8 bytes, one pointer
  └──────────────┘

Trait object (&dyn ScoringStrategy):
  ┌──────────────┬──────────────┐
  │ data pointer  │ vtable ptr   │  ← 16 bytes, two pointers
  └──────┬───────┴──────┬───────┘
         │              │
         ▼              ▼
  ┌───────────────┐  ┌──────────────────────┐
  │ ForTimeScorer  │  │ vtable:              │
  │ { time: 185 } │  │   score()   → 0x7f00 │
  │               │  │   description → 0x8a10│
  └───────────────┘  │   drop()   → 0x9c20 │
                     │   size:    4         │
                     │   align:   4         │
                     └──────────────────────┘
```

The left pointer says "here is the data." The right pointer says "here is the instruction sheet that tells you what to do with it." This is why they call it a fat pointer -- it is twice the width of a normal pointer.

Prove it:

```rust
use std::mem::size_of;

fn main() {
    // Regular references: one pointer
    assert_eq!(size_of::<&u32>(), 8);
    assert_eq!(size_of::<&String>(), 8);

    // Trait objects: two pointers (fat pointer)
    assert_eq!(size_of::<&dyn std::fmt::Display>(), 16);
    assert_eq!(size_of::<&dyn std::fmt::Debug>(), 16);

    // Box<dyn Trait> is also a fat pointer (heap-allocated data + vtable)
    assert_eq!(size_of::<Box<dyn std::fmt::Display>>(), 16);

    println!("Regular ref: {} bytes", size_of::<&u32>());
    println!("Trait object: {} bytes", size_of::<&dyn std::fmt::Display>());
    println!("Fat pointer confirmed!");
}
```

Back to the gym analogy: a regular reference is picking up a specific workout card -- you can see it says "ForTime" right on the front. A trait object is picking up a card in a plain sleeve. You cannot see the type, but you CAN flip it over and read the instruction sheet (vtable) to know what to do.

---

## 4. Build a Vtable by Hand

To truly understand what the compiler generates, let's build our own vtable. This is the machinery that `dyn ScoringStrategy` hides from you:

```rust
use std::alloc::{self, Layout};

// The vtable the compiler would generate
struct ScoringVtable {
    score: fn(*const ()) -> f64,
    description: fn(*const ()) -> String,
    drop: fn(*mut ()),
    size: usize,
    align: usize,
}

// Our concrete type
struct ForTimeScorer {
    time_seconds: u32,
}

// Vtable function implementations -- note the raw pointer casts
fn fortime_score(ptr: *const ()) -> f64 {
    let scorer = unsafe { &*(ptr as *const ForTimeScorer) };
    scorer.time_seconds as f64
}

fn fortime_description(ptr: *const ()) -> String {
    let scorer = unsafe { &*(ptr as *const ForTimeScorer) };
    format!("For Time: {}s", scorer.time_seconds)
}

fn fortime_drop(ptr: *mut ()) {
    unsafe {
        let _ = Box::from_raw(ptr as *mut ForTimeScorer);
    }
}

// The vtable for ForTimeScorer -- one static instance, shared by all ForTimeScorer trait objects
static FORTIME_VTABLE: ScoringVtable = ScoringVtable {
    score: fortime_score,
    description: fortime_description,
    drop: fortime_drop,
    size: std::mem::size_of::<ForTimeScorer>(),
    align: std::mem::align_of::<ForTimeScorer>(),
};

// A "manual trait object" -- exactly what &dyn ScoringStrategy is under the hood
struct ManualTraitObject {
    data: *const (),
    vtable: &'static ScoringVtable,
}

impl ManualTraitObject {
    fn score(&self) -> f64 {
        (self.vtable.score)(self.data)
    }

    fn description(&self) -> String {
        (self.vtable.description)(self.data)
    }
}

fn main() {
    let scorer = ForTimeScorer { time_seconds: 185 };

    // Create our manual "trait object"
    let trait_obj = ManualTraitObject {
        data: &scorer as *const ForTimeScorer as *const (),
        vtable: &FORTIME_VTABLE,
    };

    // Call through the vtable -- exactly what dyn dispatch does
    println!("{}: {}", trait_obj.description(), trait_obj.score());
    // Prints: "For Time: 185s: 185"

    println!("Vtable size field: {} bytes", trait_obj.vtable.size);
    println!("Vtable align field: {} bytes", trait_obj.vtable.align);
}
```

This is educational code -- you would never write this in production. But now you know: when Rust sees `&dyn ScoringStrategy`, it generates exactly this structure. One static vtable per concrete type, shared by every trait object of that type. The `dyn` keyword is syntactic sugar over a pair of pointers and a lookup table.

---

## 5. Object Safety -- Why Some Traits Can't Be Trait Objects

Not every trait can become a trait object. The compiler must be able to build a vtable, and some trait designs make that impossible. A trait is "object safe" only if:

1. **No methods return `Self`** -- the vtable doesn't know the concrete type's size.
2. **No generic methods** -- you can't put infinite function pointers in a finite table.
3. **No `Self: Sized` bound** -- trait objects are unsized by definition.

Each rule violated, with a GrindIt example:

```rust
// NOT object safe -- the compiler will refuse to create dyn NotObjectSafe
trait NotObjectSafe {
    // Rule 1: returns Self -- what size is Self behind a trait object?
    fn clone_score(&self) -> Self;

    // Rule 2: generic method -- there would be infinite versions of convert<T>
    fn convert<T>(&self, val: T) -> String;
}

// Object-safe version -- every method has a known signature
trait ObjectSafe {
    // Fix rule 1: return a Box<dyn ...> instead of Self
    fn clone_score(&self) -> Box<dyn ObjectSafe>;

    // Fix rule 2: use a concrete type instead of a generic
    fn convert_string(&self, val: String) -> String;
}

// This compiles -- ObjectSafe can be used as dyn ObjectSafe
fn use_it(scorer: &dyn ObjectSafe) {
    let cloned = scorer.clone_score();
    let result = cloned.convert_string("test".to_string());
    println!("{}", result);
}

struct DummyScorer;
impl ObjectSafe for DummyScorer {
    fn clone_score(&self) -> Box<dyn ObjectSafe> {
        Box::new(DummyScorer)
    }
    fn convert_string(&self, val: String) -> String {
        format!("Converted: {}", val)
    }
}

fn main() {
    let scorer = DummyScorer;
    use_it(&scorer);
}
```

The intuition: the vtable is a fixed-size table of function pointers. If a method returns `Self`, the vtable would need to know the concrete type's size -- but the whole point of a trait object is that the concrete type has been erased. If a method is generic over `T`, you would need a separate function pointer for every possible `T` -- an infinite table. Object safety rules exist to guarantee the vtable can be built.

---

## 6. Static vs Dynamic -- GrindIt Decision Guide

Here is the same scoring system using both approaches, so you can see the trade-off directly:

```rust
trait ScoringStrategy {
    fn score(&self) -> f64;
    fn description(&self) -> String;
}

struct ForTimeScorer { time_seconds: u32 }
struct AmrapScorer { rounds: u32, extra_reps: u32 }
struct StrengthScorer { weight_lbs: f64 }

impl ScoringStrategy for ForTimeScorer {
    fn score(&self) -> f64 { self.time_seconds as f64 }
    fn description(&self) -> String { format!("For Time: {}s", self.time_seconds) }
}
impl ScoringStrategy for AmrapScorer {
    fn score(&self) -> f64 { self.rounds as f64 + (self.extra_reps as f64 / 100.0) }
    fn description(&self) -> String { format!("AMRAP: {} + {}", self.rounds, self.extra_reps) }
}
impl ScoringStrategy for StrengthScorer {
    fn score(&self) -> f64 { self.weight_lbs }
    fn description(&self) -> String { format!("Strength: {} lbs", self.weight_lbs) }
}

// STATIC dispatch -- compiler generates one copy per type
fn process_score_static<S: ScoringStrategy>(strategy: &S) -> f64 {
    strategy.score()
}

// DYNAMIC dispatch -- one copy, vtable lookup at runtime
fn process_score_dynamic(strategy: &dyn ScoringStrategy) -> f64 {
    strategy.score()
}

fn main() {
    // Static: each call is monomorphized, can be inlined
    let ft = ForTimeScorer { time_seconds: 185 };
    let amrap = AmrapScorer { rounds: 12, extra_reps: 5 };
    println!("Static: {}", process_score_static(&ft));
    println!("Static: {}", process_score_static(&amrap));

    // Dynamic: heterogeneous collection -- MUST use trait objects
    let strategies: Vec<Box<dyn ScoringStrategy>> = vec![
        Box::new(ForTimeScorer { time_seconds: 185 }),
        Box::new(AmrapScorer { rounds: 12, extra_reps: 5 }),
        Box::new(StrengthScorer { weight_lbs: 225.0 }),
    ];

    for strategy in &strategies {
        println!("Dynamic: {} = {}", strategy.description(), strategy.score());
    }
}
```

The decision guide:

| Factor | Static (`impl Trait` / generics) | Dynamic (`dyn Trait`) |
|--------|--------------------------------|----------------------|
| Speed | Zero overhead -- inlined | One pointer indirection |
| Binary size | Larger (one copy per type) | Smaller (one copy) |
| Heterogeneous collections | Cannot mix types in Vec | `Vec<Box<dyn T>>` works |
| Compile time | Slower (monomorphization) | Faster |
| Use in GrindIt | Component props, utility functions | Scoring strategies, storage backends |

Default to static dispatch. Reach for dynamic dispatch when you need to store different types together or when you are building a plugin-style API.

---

## 7. enum vs Trait Object -- Rust's Third Way

Other languages give you two choices: static dispatch or dynamic dispatch. Rust gives you a third option that is often the best: enums as closed polymorphism.

```rust
trait ScoringStrategy {
    fn score(&self) -> f64;
    fn description(&self) -> String;
}

struct ForTimeScorer { time_seconds: u32 }
struct AmrapScorer { rounds: u32, extra_reps: u32 }
struct StrengthScorer { weight_lbs: f64 }

impl ScoringStrategy for ForTimeScorer {
    fn score(&self) -> f64 { self.time_seconds as f64 }
    fn description(&self) -> String { format!("For Time: {}s", self.time_seconds) }
}
impl ScoringStrategy for AmrapScorer {
    fn score(&self) -> f64 { self.rounds as f64 + (self.extra_reps as f64 / 100.0) }
    fn description(&self) -> String { format!("AMRAP: {} + {}", self.rounds, self.extra_reps) }
}
impl ScoringStrategy for StrengthScorer {
    fn score(&self) -> f64 { self.weight_lbs }
    fn description(&self) -> String { format!("Strength: {} lbs", self.weight_lbs) }
}

// Enum -- closed set of variants, stack-allocated, pattern-matchable
enum ScoringType {
    ForTime(ForTimeScorer),
    Amrap(AmrapScorer),
    Strength(StrengthScorer),
}

impl ScoringType {
    fn score(&self) -> f64 {
        match self {
            Self::ForTime(s) => s.score(),
            Self::Amrap(s) => s.score(),
            Self::Strength(s) => s.score(),
        }
    }

    fn description(&self) -> String {
        match self {
            Self::ForTime(s) => s.description(),
            Self::Amrap(s) => s.description(),
            Self::Strength(s) => s.description(),
        }
    }
}

fn main() {
    // All three variants in one Vec -- no Box, no heap allocation
    let scores = vec![
        ScoringType::ForTime(ForTimeScorer { time_seconds: 185 }),
        ScoringType::Amrap(AmrapScorer { rounds: 12, extra_reps: 5 }),
        ScoringType::Strength(StrengthScorer { weight_lbs: 225.0 }),
    ];

    for s in &scores {
        println!("{}: {}", s.description(), s.score());
    }

    // Pattern matching -- exhaustive, the compiler checks you covered every variant
    let first = &scores[0];
    match first {
        ScoringType::ForTime(ft) => println!("Time was {} seconds", ft.time_seconds),
        ScoringType::Amrap(a) => println!("Got {} rounds", a.rounds),
        ScoringType::Strength(st) => println!("Lifted {} lbs", st.weight_lbs),
    }
}
```

When to choose each:

| | Enum | Trait Object |
|--|------|-------------|
| Known set of variants | Perfect | Overkill |
| Open for extension | Must modify enum | Just impl the trait |
| Stack-allocated | Yes (sized) | No, needs Box (heap) |
| Pattern matching | Exhaustive | Not possible |
| GrindIt usage | `ScoringType`, `WodSection` | Plugin systems, middleware |

In GrindIt, the scoring types are a known, closed set -- ForTime, AMRAP, Strength, maybe MaxReps. An enum is the right choice. If you were building a scoring SDK where third-party gyms could define their own scoring types, trait objects would be the way to go.

---

## 8. Performance -- How Much Does Dynamic Dispatch Cost?

The honest answer: almost nothing, unless you are in a tight loop over millions of items.

- **One pointer dereference** to look up the function in the vtable. On modern hardware, roughly 1 nanosecond.
- **Prevents inlining.** The compiler cannot see through the vtable to optimize the called function. This is the bigger cost -- inlining enables constant folding, dead code elimination, and loop unrolling.
- **For scoring 30 athletes in a class:** unmeasurable. The dominant cost is the database query, not the vtable lookup.
- **For processing 10 million data points in a hot loop:** measurable. Profile first, then decide.

The rule of thumb: default to static dispatch (generics). Use dynamic dispatch when you need heterogeneous collections or plugin-style extensibility. If you are unsure whether the performance difference matters, it doesn't.

---

## 9. Try It Yourself

**Exercise 1: Heterogeneous Display**

Create a `Vec<Box<dyn std::fmt::Display>>` containing three different types: a `u32`, an `f64`, and a `String`. Print all three in a loop using `{}` formatting. Then try adding a type that does NOT implement `Display` (like a custom struct without a `Display` impl) and observe the compiler error.

**Exercise 2: Fat Pointer Proof**

Write a `ScoringStrategy` trait with `score()` and `description()` methods. Implement it for three types. Create both a generic function (static dispatch) and a `&dyn ScoringStrategy` function (dynamic dispatch) that accepts a scorer. Use `std::mem::size_of_val` on a `&dyn ScoringStrategy` reference to confirm the fat pointer is 16 bytes. Compare it against the size of a regular reference.

**Exercise 3: Object Safety**

Define a trait `Cloneable` with a method `fn duplicate(&self) -> Self`. Try to use it as `&dyn Cloneable`. Read the compiler error carefully -- it will tell you exactly which object safety rule was violated. Fix the trait by changing the return type to `Box<dyn Cloneable>`, and verify the trait object compiles.

---

## Summary

| Concept | What It Is | Gym Analogy |
|---------|-----------|-------------|
| Static dispatch | Compiler generates one function per type | Separate manuals for each workout type |
| Dynamic dispatch | One function, vtable lookup at runtime | One generic manual, flip card for instructions |
| Fat pointer | Two pointers: data + vtable | Holding the card with both hands |
| Vtable | Table of function pointers per type | Instruction sheet on the back of the card |
| Object safety | Rules for traits that can become trait objects | The instruction sheet format must be fixed-size |
| Enum dispatch | Match on a closed set of variants | A filing cabinet with labeled drawers |

When you write `Box<dyn ScoringStrategy>`, Rust is not doing magic. It is storing two pointers, looking up a function address in a table, and calling through it. Now you know exactly what those two pointers are, what the table contains, and why some traits cannot make the cut. The workout card has no secrets left.
