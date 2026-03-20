// Chapter 0.4: Putting It All Together — Workout Volume Calculator
// Combines variables, types, functions, if/else, and for loops.
// Run with: cargo run --bin building_blocks

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
        println!(
            "  Set {}: {} reps @ {} kg  |  Running total: {} kg",
            set_number, reps, weight, total_volume
        );
    }

    // Output
    println!("===========================");
    let expected_volume = calculate_volume(sets, reps, weight);
    let classification = classify_workout(expected_volume);
    println!("  Total volume: {} kg", expected_volume);
    println!("  Intensity: {}", classification);
    println!("===========================");
}
