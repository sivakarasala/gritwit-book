// Chapter 0.2: Break Things on Purpose
// This file shows the FIXED versions of intentional compiler errors.
// Run with: cargo run --bin compiler_error

fn main() {
    // Fix 1: Missing closing quote — add the quote
    println!("This string is properly closed");

    // Fix 2: Missing semicolon — add it
    println!("This line has a semicolon");

    // Fix 3: Misspelled macro — use println! not printl!
    println!("Spelled correctly!");
}
