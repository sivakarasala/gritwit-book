// Chapter 2 DSA Exercise: Data Modeling
//
// Struct as record, memory layout, and how Rust's type system
// maps to database design. Uses GrindIt's Exercise model.

use std::collections::HashMap;

/// An exercise in the GrindIt library — maps 1:1 to a database row.
#[derive(Debug, Clone)]
struct Exercise {
    id: String,
    name: String,
    category: String,
    scoring_type: String,
    muscle_groups: Vec<String>,
}

impl Exercise {
    fn new(id: &str, name: &str, category: &str, scoring_type: &str, muscles: &[&str]) -> Self {
        Exercise {
            id: id.to_string(),
            name: name.to_string(),
            category: category.to_string(),
            scoring_type: scoring_type.to_string(),
            muscle_groups: muscles.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Interview problem: Given a list of records, group them by a key field
/// and compute aggregate statistics per group.
///
/// This mirrors real database GROUP BY operations and tests understanding
/// of HashMap, iterators, and struct access patterns.
fn group_by_category(exercises: &[Exercise]) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for ex in exercises {
        groups
            .entry(ex.category.clone())
            .or_default()
            .push(ex.name.clone());
    }
    groups
}

/// Count exercises per scoring type — similar to SELECT scoring_type, COUNT(*)
/// FROM exercises GROUP BY scoring_type
fn count_by_scoring_type(exercises: &[Exercise]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for ex in exercises {
        *counts.entry(ex.scoring_type.clone()).or_insert(0) += 1;
    }
    counts
}

/// Interview question: Find all exercises that target a specific muscle group.
/// This is a linear scan with nested iteration — O(n * m) where n = exercises
/// and m = average muscle groups per exercise.
fn exercises_targeting_muscle(exercises: &[Exercise], muscle: &str) -> Vec<String> {
    exercises
        .iter()
        .filter(|ex| ex.muscle_groups.iter().any(|m| m == muscle))
        .map(|ex| ex.name.clone())
        .collect()
}

/// Memory layout demonstration: Rust structs have a known size at compile time.
/// String is 24 bytes (ptr + len + capacity) on 64-bit. Vec<String> is also 24 bytes.
/// The struct itself is stack-allocated; the String/Vec contents are heap-allocated.
fn show_memory_layout() {
    println!("=== Memory Layout ===");
    println!("Size of Exercise struct: {} bytes", std::mem::size_of::<Exercise>());
    println!("Size of String:          {} bytes", std::mem::size_of::<String>());
    println!("Size of Vec<String>:     {} bytes", std::mem::size_of::<Vec<String>>());
    println!("Size of &str:            {} bytes", std::mem::size_of::<&str>());
    println!();
    println!("The struct stores metadata (pointers, lengths) on the stack.");
    println!("The actual string content lives on the heap.");
    println!("This is why Clone copies heap data — it's not just copying pointers.");
}

fn main() {
    let exercises = vec![
        Exercise::new("1", "Back Squat", "Weightlifting", "weight_and_reps", &["Quadriceps", "Glutes", "Hamstrings"]),
        Exercise::new("2", "Deadlift", "Weightlifting", "weight_and_reps", &["Hamstrings", "Glutes", "Back"]),
        Exercise::new("3", "Pull-Up", "Gymnastics", "reps", &["Lats", "Biceps", "Back"]),
        Exercise::new("4", "Muscle-Up", "Gymnastics", "reps", &["Lats", "Chest", "Triceps"]),
        Exercise::new("5", "Box Jump", "Conditioning", "reps", &["Quadriceps", "Glutes"]),
        Exercise::new("6", "Rowing", "Conditioning", "calories", &["Back", "Legs"]),
        Exercise::new("7", "Bench Press", "Weightlifting", "weight_and_reps", &["Chest", "Triceps"]),
        Exercise::new("8", "Thruster", "Weightlifting", "weight_and_reps", &["Quadriceps", "Shoulders"]),
    ];

    // GROUP BY category
    println!("=== Exercises Grouped by Category ===");
    let groups = group_by_category(&exercises);
    for (category, names) in &groups {
        println!("  {}: {}", category, names.join(", "));
    }
    println!();

    // COUNT BY scoring_type
    println!("=== Count by Scoring Type ===");
    let counts = count_by_scoring_type(&exercises);
    for (scoring_type, count) in &counts {
        println!("  {}: {} exercises", scoring_type, count);
    }
    println!();

    // Filter by muscle group
    println!("=== Exercises targeting Glutes ===");
    let glute_exercises = exercises_targeting_muscle(&exercises, "Glutes");
    for name in &glute_exercises {
        println!("  - {}", name);
    }
    println!();

    println!("=== Exercises targeting Back ===");
    let back_exercises = exercises_targeting_muscle(&exercises, "Back");
    for name in &back_exercises {
        println!("  - {}", name);
    }
    println!();

    // Memory layout
    show_memory_layout();
}
