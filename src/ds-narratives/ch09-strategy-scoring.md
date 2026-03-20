# Every Scoring Type Knows How to Score Itself

## The Problem: The Match Statement That Ate Your Codebase

GrindIt supports three workout scoring types. ForTime means lowest time wins. AMRAP means most rounds+reps wins. Strength means heaviest weight wins. Your first implementation is honest and direct:

```rust
#[derive(Debug, Clone)]
enum ScoringType {
    ForTime,
    Amrap,
    Strength,
}

#[derive(Debug, Clone)]
struct Score {
    athlete: String,
    value: f64,       // seconds for ForTime, rounds.reps for AMRAP, lbs for Strength
    scoring_type: ScoringType,
}

fn compare_scores(a: &Score, b: &Score) -> std::cmp::Ordering {
    match a.scoring_type {
        ScoringType::ForTime => {
            // Lower time is better
            a.value.partial_cmp(&b.value).unwrap()
        }
        ScoringType::Amrap => {
            // Higher rounds+reps is better
            b.value.partial_cmp(&a.value).unwrap()
        }
        ScoringType::Strength => {
            // Higher weight is better
            b.value.partial_cmp(&a.value).unwrap()
        }
    }
}

fn format_score(score: &Score) -> String {
    match score.scoring_type {
        ScoringType::ForTime => {
            let mins = (score.value as u64) / 60;
            let secs = (score.value as u64) % 60;
            format!("{}:{:02}", mins, secs)
        }
        ScoringType::Amrap => {
            let rounds = score.value as u32;
            let reps = ((score.value - rounds as f64) * 100.0) as u32;
            format!("{} rounds + {} reps", rounds, reps)
        }
        ScoringType::Strength => format!("{} lbs", score.value),
    }
}

fn rank_label(score: &Score) -> String {
    match score.scoring_type {
        ScoringType::ForTime => "Fastest".to_string(),
        ScoringType::Amrap => "Most Rounds".to_string(),
        ScoringType::Strength => "Heaviest".to_string(),
    }
}
```

Three `match` statements, each with three arms. Nine total branches. Now your coach says: "Can we add EMOM scoring? And Tabata?" Each new type adds an arm to *every* match. Miss one and the compiler yells at you (thanks, Rust), but the real problem is subtler — every new scoring type forces you to modify `compare_scores`, `format_score`, `rank_label`, and any other function that touches scoring. You're one distracted Friday afternoon away from a bug.

## The Insight: What If Each Scoring Type Knew How to Score Itself?

Instead of one function that knows about *all* scoring types, what if each scoring type carried its own behavior? The ForTime scorer knows that lower is better. The AMRAP scorer knows that higher is better. They each know how to format themselves. You just ask them: "hey, score this."

This is the Strategy pattern — define a family of algorithms, encapsulate each one, and make them interchangeable.

## The Build: Traits as Strategies

In Rust, a trait is a natural strategy. Let's define the contract:

```rust
use std::cmp::Ordering;

trait ScoringStrategy {
    /// Compare two raw scores. Return Ordering from the perspective of ranking
    /// (Less means `a` ranks higher / is better than `b`).
    fn compare(&self, a: f64, b: f64) -> Ordering;

    /// Format a raw score value into a human-readable string.
    fn format(&self, value: f64) -> String;

    /// Label for the leaderboard ("Fastest", "Heaviest", etc.)
    fn rank_label(&self) -> &str;

    /// Name of this scoring type.
    fn name(&self) -> &str;
}
```

Now each scoring type implements the trait:

```rust
struct ForTimeScorer;

impl ScoringStrategy for ForTimeScorer {
    fn compare(&self, a: f64, b: f64) -> Ordering {
        // Lower time is better
        a.partial_cmp(&b).unwrap_or(Ordering::Equal)
    }

    fn format(&self, value: f64) -> String {
        let total_secs = value as u64;
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}:{:02}", mins, secs)
    }

    fn rank_label(&self) -> &str { "Fastest" }
    fn name(&self) -> &str { "For Time" }
}

struct AmrapScorer;

impl ScoringStrategy for AmrapScorer {
    fn compare(&self, a: f64, b: f64) -> Ordering {
        // Higher is better — reverse the comparison
        b.partial_cmp(&a).unwrap_or(Ordering::Equal)
    }

    fn format(&self, value: f64) -> String {
        let rounds = value as u32;
        let reps = ((value - rounds as f64) * 100.0) as u32;
        format!("{} rounds + {} reps", rounds, reps)
    }

    fn rank_label(&self) -> &str { "Most Rounds" }
    fn name(&self) -> &str { "AMRAP" }
}

struct StrengthScorer;

impl ScoringStrategy for StrengthScorer {
    fn compare(&self, a: f64, b: f64) -> Ordering {
        b.partial_cmp(&a).unwrap_or(Ordering::Equal)
    }

    fn format(&self, value: f64) -> String {
        format!("{} lbs", value)
    }

    fn rank_label(&self) -> &str { "Heaviest" }
    fn name(&self) -> &str { "Strength" }
}
```

Now the magic moment. Your coach asks for EMOM scoring. You don't touch *any* existing code:

```rust
struct EmomScorer;

impl ScoringStrategy for EmomScorer {
    fn compare(&self, a: f64, b: f64) -> Ordering {
        // Higher total reps completed is better
        b.partial_cmp(&a).unwrap_or(Ordering::Equal)
    }

    fn format(&self, value: f64) -> String {
        format!("{} total reps", value as u32)
    }

    fn rank_label(&self) -> &str { "Most Reps" }
    fn name(&self) -> &str { "EMOM" }
}
```

That's the Open/Closed Principle in action — open for extension, closed for modification.

## Dynamic Dispatch: `Box<dyn ScoringStrategy>`

When you don't know the scoring type until runtime (like reading it from a database), you need dynamic dispatch:

```rust
struct Workout {
    name: String,
    scorer: Box<dyn ScoringStrategy>,
}

struct AthleteScore {
    athlete: String,
    value: f64,
}

impl Workout {
    fn leaderboard(&self, scores: &mut [AthleteScore]) -> Vec<String> {
        scores.sort_by(|a, b| self.scorer.compare(a.value, b.value));
        scores
            .iter()
            .enumerate()
            .map(|(i, s)| {
                format!(
                    "#{} {} — {}",
                    i + 1,
                    s.athlete,
                    self.scorer.format(s.value)
                )
            })
            .collect()
    }
}

fn scorer_from_type(name: &str) -> Box<dyn ScoringStrategy> {
    match name {
        "for_time" => Box::new(ForTimeScorer),
        "amrap" => Box::new(AmrapScorer),
        "strength" => Box::new(StrengthScorer),
        "emom" => Box::new(EmomScorer),
        _ => panic!("Unknown scoring type: {}", name),
    }
}
```

The `match` still exists in `scorer_from_type` — you can't entirely eliminate it when bridging from stringly-typed data. But it's a *single* match in *one* place, and all it does is pick the strategy. The behavior lives with the strategy.

## Static Dispatch: When You Know at Compile Time

If the scoring type is known at compile time, generics give you zero-cost abstraction:

```rust
fn print_leaderboard<S: ScoringStrategy>(
    strategy: &S,
    scores: &mut Vec<AthleteScore>,
) {
    scores.sort_by(|a, b| strategy.compare(a.value, b.value));
    println!("--- {} Leaderboard ({}) ---", strategy.name(), strategy.rank_label());
    for (i, s) in scores.iter().enumerate() {
        println!("  #{} {} — {}", i + 1, s.athlete, strategy.format(s.value));
    }
}
```

No `Box`, no vtable, no heap allocation. The compiler monomorphizes this into separate functions for each strategy. Use this when performance matters and the type is statically known.

## The Payoff

```rust
fn main() {
    let mut scores = vec![
        AthleteScore { athlete: "Alex".into(), value: 485.0 },
        AthleteScore { athlete: "Jordan".into(), value: 423.0 },
        AthleteScore { athlete: "Sam".into(), value: 512.0 },
    ];

    // Static dispatch — type known at compile time
    print_leaderboard(&ForTimeScorer, &mut scores);

    // Dynamic dispatch — type from "database"
    let workout = Workout {
        name: "Fran".into(),
        scorer: scorer_from_type("for_time"),
    };
    let board = workout.leaderboard(&mut scores);
    for line in &board {
        println!("{}", line);
    }
}
```

Adding Tabata scoring? Write a struct, implement the trait, done. Nobody touches `ForTimeScorer`. Nobody touches `Workout`. Nobody touches `leaderboard`. Each scoring type is an island.

## Complexity Comparison

| Concern | Enum + Match | Trait Strategy |
|---|---|---|
| Add new scoring type | Touch every function with a `match` | Add one new struct + impl |
| Runtime score comparison | O(1) match arm | O(1) vtable lookup (dynamic) or inlined (static) |
| Compile-time safety | Exhaustive match catches missing arms | Compiler enforces trait impl completeness |
| Code locality | Behavior scattered across functions | Behavior co-located with its type |
| Testing | Test the mega-function with every variant | Test each strategy in isolation |

Neither approach is wrong. Enums work great when the set of variants is small and stable. Traits shine when the set is open-ended and growing. GrindIt's scoring types will keep growing as coaches invent new formats — traits are the right call here.

## Try It Yourself

1. **Add a `TabataScorer`** where the score is the *lowest* round count (Tabata scoring penalizes your worst round). Implement `ScoringStrategy` for it without modifying any existing code.

2. **Add a `validate` method** to the `ScoringStrategy` trait that checks whether a raw score value is valid (e.g., ForTime must be positive, AMRAP can't be negative). Give it a default implementation that accepts any non-negative value, then override it for types with stricter rules.

3. **Build a `ScoringRegistry`** — a `HashMap<String, Box<dyn ScoringStrategy>>` that lets you register and look up scorers by name at runtime. Replace `scorer_from_type`'s match with a registry lookup.
