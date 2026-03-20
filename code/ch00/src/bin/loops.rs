// Chapter 0.4: Loops — for loop through sets with running total
// Run with: cargo run --bin loops

fn main() {
    let reps = 5;
    let weight = 100.0;
    let mut total_volume = 0.0;

    println!("Back Squat — 5 sets x 5 reps @ 100 kg\n");

    for set_number in 1..=5 {
        let set_volume = reps as f64 * weight;
        total_volume += set_volume;
        println!(
            "  Set {}: {} reps @ {} kg  |  Running total: {} kg",
            set_number, reps, weight, total_volume
        );
    }

    println!("\nTotal volume: {} kg", total_volume);

    // While loop example: countdown
    println!("\n--- Rep countdown ---");
    let mut remaining = 5;
    while remaining > 0 {
        println!("{} reps to go...", remaining);
        remaining -= 1;
    }
    println!("Done! Good set.");
}
