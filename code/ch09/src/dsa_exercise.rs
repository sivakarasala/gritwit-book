// Chapter 9 DSA Exercise: Strategy Pattern
//
// Different WOD section types use different scoring strategies.
// Match-based dispatch (closed set) vs trait-based dispatch (open set).

use std::fmt;

// ----------------------------------------------------------------
// The scoring domain: each section type scores differently
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
enum SectionType {
    ForTime,
    Amrap,
    Emom,
    Strength,
    Static,
}

impl fmt::Display for SectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SectionType::ForTime => write!(f, "For Time"),
            SectionType::Amrap => write!(f, "AMRAP"),
            SectionType::Emom => write!(f, "EMOM"),
            SectionType::Strength => write!(f, "Strength"),
            SectionType::Static => write!(f, "Static"),
        }
    }
}

/// Raw score input from the athlete
#[derive(Debug, Clone)]
struct ScoreInput {
    finish_time_seconds: Option<u32>,
    rounds_completed: Option<u32>,
    extra_reps: Option<u32>,
    weight_kg: Option<f64>,
    is_rx: bool,
}

/// Computed score for leaderboard ranking
#[derive(Debug)]
struct ComputedScore {
    display: String,
    sort_value: f64, // lower is better for time, higher is better for rounds/weight
    is_rx: bool,
}

// ----------------------------------------------------------------
// Approach 1: Match-based strategy (what GrindIt uses)
// Good for a closed, fixed set of types.
// ----------------------------------------------------------------

fn compute_score_match(section_type: &SectionType, input: &ScoreInput) -> ComputedScore {
    match section_type {
        SectionType::ForTime => {
            let seconds = input.finish_time_seconds.unwrap_or(0);
            let mins = seconds / 60;
            let secs = seconds % 60;
            ComputedScore {
                display: format!("{}:{:02}", mins, secs),
                sort_value: -(seconds as f64), // negative because lower time = better
                is_rx: input.is_rx,
            }
        }
        SectionType::Amrap => {
            let rounds = input.rounds_completed.unwrap_or(0);
            let extra = input.extra_reps.unwrap_or(0);
            ComputedScore {
                display: format!("{} rounds + {} reps", rounds, extra),
                sort_value: (rounds * 1000 + extra) as f64, // higher = better
                is_rx: input.is_rx,
            }
        }
        SectionType::Emom => {
            let rounds = input.rounds_completed.unwrap_or(0);
            ComputedScore {
                display: format!("{} rounds completed", rounds),
                sort_value: rounds as f64,
                is_rx: input.is_rx,
            }
        }
        SectionType::Strength => {
            let weight = input.weight_kg.unwrap_or(0.0);
            ComputedScore {
                display: format!("{:.1} kg", weight),
                sort_value: weight, // higher = better
                is_rx: input.is_rx,
            }
        }
        SectionType::Static => ComputedScore {
            display: "Completed".to_string(),
            sort_value: 0.0,
            is_rx: true,
        },
    }
}

// ----------------------------------------------------------------
// Approach 2: Trait-based strategy (for extensible systems)
// Good when third parties can add new scoring algorithms.
// ----------------------------------------------------------------

trait ScoringStrategy {
    fn compute(&self, input: &ScoreInput) -> ComputedScore;
    fn name(&self) -> &str;
}

struct ForTimeStrategy;
struct AmrapStrategy;
struct StrengthStrategy;

impl ScoringStrategy for ForTimeStrategy {
    fn compute(&self, input: &ScoreInput) -> ComputedScore {
        let seconds = input.finish_time_seconds.unwrap_or(0);
        let mins = seconds / 60;
        let secs = seconds % 60;
        ComputedScore {
            display: format!("{}:{:02}", mins, secs),
            sort_value: -(seconds as f64),
            is_rx: input.is_rx,
        }
    }
    fn name(&self) -> &str {
        "For Time"
    }
}

impl ScoringStrategy for AmrapStrategy {
    fn compute(&self, input: &ScoreInput) -> ComputedScore {
        let rounds = input.rounds_completed.unwrap_or(0);
        let extra = input.extra_reps.unwrap_or(0);
        ComputedScore {
            display: format!("{} rounds + {} reps", rounds, extra),
            sort_value: (rounds * 1000 + extra) as f64,
            is_rx: input.is_rx,
        }
    }
    fn name(&self) -> &str {
        "AMRAP"
    }
}

impl ScoringStrategy for StrengthStrategy {
    fn compute(&self, input: &ScoreInput) -> ComputedScore {
        let weight = input.weight_kg.unwrap_or(0.0);
        ComputedScore {
            display: format!("{:.1} kg", weight),
            sort_value: weight,
            is_rx: input.is_rx,
        }
    }
    fn name(&self) -> &str {
        "Strength"
    }
}

/// Factory: select strategy at runtime (trait object approach)
fn get_strategy(section_type: &SectionType) -> Box<dyn ScoringStrategy> {
    match section_type {
        SectionType::ForTime => Box::new(ForTimeStrategy),
        SectionType::Amrap => Box::new(AmrapStrategy),
        SectionType::Strength => Box::new(StrengthStrategy),
        _ => Box::new(ForTimeStrategy), // fallback
    }
}

// ----------------------------------------------------------------
// Leaderboard: sort athletes using the scoring strategy
// ----------------------------------------------------------------

#[derive(Debug)]
struct LeaderboardEntry {
    athlete: String,
    score: ComputedScore,
}

fn sort_leaderboard(entries: &mut [LeaderboardEntry], higher_is_better: bool) {
    entries.sort_by(|a, b| {
        // RX always ranks above Scaled
        let rx_cmp = b.score.is_rx.cmp(&a.score.is_rx);
        if rx_cmp != std::cmp::Ordering::Equal {
            return rx_cmp;
        }
        // Then by score value
        if higher_is_better {
            b.score
                .sort_value
                .partial_cmp(&a.score.sort_value)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.score
                .sort_value
                .partial_cmp(&b.score.sort_value)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });
}

fn main() {
    println!("=== Strategy Pattern: WOD Scoring ===\n");

    // Sample scores for a For Time WOD
    let for_time_scores = vec![
        ("Alice", ScoreInput { finish_time_seconds: Some(480), rounds_completed: None, extra_reps: None, weight_kg: None, is_rx: true }),
        ("Bob",   ScoreInput { finish_time_seconds: Some(540), rounds_completed: None, extra_reps: None, weight_kg: None, is_rx: true }),
        ("Carol", ScoreInput { finish_time_seconds: Some(420), rounds_completed: None, extra_reps: None, weight_kg: None, is_rx: false }),
        ("Dave",  ScoreInput { finish_time_seconds: Some(510), rounds_completed: None, extra_reps: None, weight_kg: None, is_rx: true }),
    ];

    // Approach 1: Match-based
    println!("--- Match-based Strategy (For Time) ---");
    let section_type = SectionType::ForTime;
    let mut entries: Vec<LeaderboardEntry> = for_time_scores
        .iter()
        .map(|(name, input)| LeaderboardEntry {
            athlete: name.to_string(),
            score: compute_score_match(&section_type, input),
        })
        .collect();

    sort_leaderboard(&mut entries, false); // lower time = better
    for (rank, entry) in entries.iter().enumerate() {
        println!(
            "  #{}: {} - {} {}",
            rank + 1,
            entry.athlete,
            entry.score.display,
            if entry.score.is_rx { "Rx" } else { "Scaled" }
        );
    }

    // Approach 2: Trait-based
    println!("\n--- Trait-based Strategy (For Time) ---");
    let strategy = get_strategy(&SectionType::ForTime);
    println!("Using strategy: {}", strategy.name());
    for (name, input) in &for_time_scores {
        let score = strategy.compute(input);
        println!("  {} => {}", name, score.display);
    }

    // AMRAP scoring
    println!("\n--- AMRAP Scoring ---");
    let amrap_scores = vec![
        ("Alice", ScoreInput { finish_time_seconds: None, rounds_completed: Some(8), extra_reps: Some(12), weight_kg: None, is_rx: true }),
        ("Bob",   ScoreInput { finish_time_seconds: None, rounds_completed: Some(7), extra_reps: Some(20), weight_kg: None, is_rx: true }),
        ("Carol", ScoreInput { finish_time_seconds: None, rounds_completed: Some(9), extra_reps: Some(5), weight_kg: None, is_rx: false }),
        ("Dave",  ScoreInput { finish_time_seconds: None, rounds_completed: Some(8), extra_reps: Some(3), weight_kg: None, is_rx: true }),
    ];

    let mut entries: Vec<LeaderboardEntry> = amrap_scores
        .iter()
        .map(|(name, input)| LeaderboardEntry {
            athlete: name.to_string(),
            score: compute_score_match(&SectionType::Amrap, input),
        })
        .collect();

    sort_leaderboard(&mut entries, true); // higher rounds = better
    for (rank, entry) in entries.iter().enumerate() {
        println!(
            "  #{}: {} - {} {}",
            rank + 1,
            entry.athlete,
            entry.score.display,
            if entry.score.is_rx { "Rx" } else { "Scaled" }
        );
    }

    // Strength scoring
    println!("\n--- Strength Scoring ---");
    let strength_scores = vec![
        ("Alice", ScoreInput { finish_time_seconds: None, rounds_completed: None, extra_reps: None, weight_kg: Some(100.0), is_rx: true }),
        ("Bob",   ScoreInput { finish_time_seconds: None, rounds_completed: None, extra_reps: None, weight_kg: Some(140.0), is_rx: true }),
        ("Carol", ScoreInput { finish_time_seconds: None, rounds_completed: None, extra_reps: None, weight_kg: Some(80.0), is_rx: true }),
    ];

    let mut entries: Vec<LeaderboardEntry> = strength_scores
        .iter()
        .map(|(name, input)| LeaderboardEntry {
            athlete: name.to_string(),
            score: compute_score_match(&SectionType::Strength, input),
        })
        .collect();

    sort_leaderboard(&mut entries, true); // higher weight = better
    for (rank, entry) in entries.iter().enumerate() {
        println!(
            "  #{}: {} - {}",
            rank + 1,
            entry.athlete,
            entry.score.display
        );
    }

    // Comparison table
    println!("\n=== Match vs Trait Strategy ===");
    println!("{:<18} {:<30} {:<30}", "Criterion", "Match (enum)", "Trait (dyn)");
    println!("{}", "-".repeat(78));
    println!("{:<18} {:<30} {:<30}", "Dispatch", "Static (compile-time)", "Dynamic (vtable)");
    println!("{:<18} {:<30} {:<30}", "Extension", "Add variant + update matches", "Implement trait");
    println!("{:<18} {:<30} {:<30}", "Exhaustiveness", "Compiler checks all arms", "No compile check");
    println!("{:<18} {:<30} {:<30}", "Overhead", "Zero (branch instruction)", "1 pointer indirection");
    println!("{:<18} {:<30} {:<30}", "Best for", "Fixed types (GrindIt)", "Pluggable types (plugins)");
    println!();
    println!("GrindIt uses match because section types are a fixed PostgreSQL enum.");
    println!("Adding a new type requires DB migration + scoring logic + UI — tightly coupled.");
    println!("The match expression is exhaustive — the compiler catches missing types.");
}
