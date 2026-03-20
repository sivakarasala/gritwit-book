// Chapter 0.3: Debug Exercise — Three Bugs Fixed
// Each section shows the corrected version of an intentional bug.
// Run with: cargo run --bin debug_exercise

fn main() {
    // Bug 1 (fixed): was missing semicolon
    println!("Starting workout...");

    // Bug 2 (fixed): was "prnintln" — misspelled
    println!("Exercise: Deadlift");

    // Bug 3 (fixed): needed to escape the inner quote
    println!("Coach says: \"Keep your back straight!\"");

    println!("All bugs fixed!");
}
