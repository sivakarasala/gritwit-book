// Chapter 0.4: Functions — calculate_volume and classify_workout
// Run with: cargo run --bin functions

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

fn print_workout_header(exercise: &str) {
    println!("===========================");
    println!("  Exercise: {}", exercise);
    println!("===========================");
}

fn main() {
    print_workout_header("Back Squat");

    let volume = calculate_volume(5, 5, 100.0);
    println!("Total volume: {} kg", volume);

    let classification = classify_workout(volume);
    println!("Intensity: {}", classification);

    println!();
    print_workout_header("Deadlift");

    let volume2 = calculate_volume(3, 3, 180.0);
    println!("Total volume: {} kg", volume2);
    println!("Intensity: {}", classify_workout(volume2));
}
