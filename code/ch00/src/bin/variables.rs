// Chapter 0.4: Variables, Types & Mutability
// Run with: cargo run --bin variables

fn main() {
    // Immutable variables (default)
    let exercise: &str = "Back Squat";
    let sets: i32 = 5;
    let reps: i32 = 5;
    let weight: f64 = 100.0;
    let is_personal_record: bool = false;

    println!("{}: {} sets x {} reps @ {} kg", exercise, sets, reps, weight);
    println!("Personal record: {}", is_personal_record);

    // Mutable variable — value changes
    let mut total_reps = 0;
    println!("\nTracking reps:");

    total_reps += 5;
    println!("After set 1: {} total reps", total_reps);

    total_reps += 5;
    println!("After set 2: {} total reps", total_reps);

    total_reps += 5;
    println!("After set 3: {} total reps", total_reps);

    // Type inference — Rust figures out the types
    let exercise2 = "Deadlift"; // Rust infers &str
    let weight2 = 140.0; // Rust infers f64
    let done = true; // Rust infers bool
    println!("\n{}: {} kg, completed: {}", exercise2, weight2, done);
}
