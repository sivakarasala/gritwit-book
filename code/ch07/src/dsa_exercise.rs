// Chapter 7 DSA Exercise: Hashing
//
// HashMap for O(1) lookup + password hashing concepts.
// Compares data structure hashing (speed) with cryptographic hashing (security).

use std::collections::HashMap;

// ----------------------------------------------------------------
// Part 1: HashMap — fast user lookup by email
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
struct User {
    id: String,
    email: String,
    display_name: String,
    role: String,
    password_hash: String,
}

/// Build a HashMap index for O(1) user lookup by email.
/// This is what the database does with a UNIQUE index on email.
fn build_email_index(users: &[User]) -> HashMap<String, usize> {
    let mut index = HashMap::new();
    for (i, user) in users.iter().enumerate() {
        index.insert(user.email.clone(), i);
    }
    index
}

/// Linear search: O(n) — scan every user
fn find_by_email_linear<'a>(users: &'a [User], email: &str) -> Option<&'a User> {
    users.iter().find(|u| u.email == email)
}

/// HashMap lookup: O(1) average
fn find_by_email_hash<'a>(
    users: &'a [User],
    index: &HashMap<String, usize>,
    email: &str,
) -> Option<&'a User> {
    index.get(email).map(|&i| &users[i])
}

// ----------------------------------------------------------------
// Part 2: Simulated password hashing
// Uses a simple hash to demonstrate the concept without external crates.
// Real apps use Argon2, bcrypt, or scrypt.
// ----------------------------------------------------------------

/// A very simple hash function for demonstration.
/// NOT cryptographically secure — use Argon2 in production.
fn simple_hash(input: &str, salt: &str) -> u64 {
    let salted = format!("{}:{}", salt, input);
    let mut hash: u64 = 5381;
    for byte in salted.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// Simulate password registration: hash the password with a random salt
fn register_password(password: &str) -> (String, u64) {
    // In production, salt is generated with a CSPRNG (OsRng)
    let salt = format!("salt_{}", password.len() * 7 + 42);
    let hash = simple_hash(password, &salt);
    (salt, hash)
}

/// Simulate password verification: hash the attempt with the stored salt
fn verify_password(attempt: &str, stored_salt: &str, stored_hash: u64) -> bool {
    let attempt_hash = simple_hash(attempt, stored_salt);
    // Constant-time comparison in production to prevent timing attacks
    attempt_hash == stored_hash
}

// ----------------------------------------------------------------
// Part 3: Hash collision demonstration
// ----------------------------------------------------------------

/// Count collisions when hashing n strings into m buckets
fn collision_analysis(items: &[String], num_buckets: usize) -> (usize, usize) {
    let mut buckets: Vec<Vec<String>> = vec![Vec::new(); num_buckets];
    for item in items {
        let mut hash: u64 = 5381;
        for byte in item.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
        }
        let bucket = (hash as usize) % num_buckets;
        buckets[bucket].push(item.clone());
    }

    let max_bucket = buckets.iter().map(|b| b.len()).max().unwrap_or(0);
    let collisions = buckets.iter().filter(|b| b.len() > 1).count();
    (collisions, max_bucket)
}

// ----------------------------------------------------------------
// Part 4: Interview problem — Two Sum using HashMap
// ----------------------------------------------------------------

/// Classic Two Sum: find two indices whose values sum to target.
/// HashMap approach: O(n) time, O(n) space.
fn two_sum(nums: &[i32], target: i32) -> Option<(usize, usize)> {
    let mut seen: HashMap<i32, usize> = HashMap::new();
    for (i, &num) in nums.iter().enumerate() {
        let complement = target - num;
        if let Some(&j) = seen.get(&complement) {
            return Some((j, i));
        }
        seen.insert(num, i);
    }
    None
}

fn main() {
    println!("=== Hashing: HashMap + Password Hashing ===\n");

    // Create test users
    let users = vec![
        User {
            id: "u1".to_string(),
            email: "mike@grindit.app".to_string(),
            display_name: "Coach Mike".to_string(),
            role: "admin".to_string(),
            password_hash: "stored_hash_1".to_string(),
        },
        User {
            id: "u2".to_string(),
            email: "jane@grindit.app".to_string(),
            display_name: "Jane".to_string(),
            role: "athlete".to_string(),
            password_hash: "stored_hash_2".to_string(),
        },
        User {
            id: "u3".to_string(),
            email: "bob@grindit.app".to_string(),
            display_name: "Bob".to_string(),
            role: "coach".to_string(),
            password_hash: "stored_hash_3".to_string(),
        },
    ];

    // Part 1: HashMap vs linear search
    println!("--- Part 1: HashMap vs Linear Search ---");
    let email_index = build_email_index(&users);

    let search_email = "jane@grindit.app";
    let linear_result = find_by_email_linear(&users, search_email);
    let hash_result = find_by_email_hash(&users, &email_index, search_email);

    println!("Linear search for '{}': {:?}", search_email, linear_result.map(|u| &u.display_name));
    println!("HashMap search for '{}': {:?}", search_email, hash_result.map(|u| &u.display_name));
    println!("Linear: O(n), HashMap: O(1) average");
    println!();

    // Part 2: Password hashing
    println!("--- Part 2: Password Hashing ---");
    let password = "hunter2";
    let (salt, hash) = register_password(password);
    println!("Password: '{}'", password);
    println!("Salt:     '{}'", salt);
    println!("Hash:     {}", hash);
    println!();

    // Same password with different salt = different hash
    let password2 = "hunter2";
    let (salt2, hash2) = register_password(password2);
    println!("Same password, different registration:");
    println!("Salt:     '{}'", salt2);
    println!("Hash:     {}", hash2);
    println!("Hashes equal: {} (same salt means same hash in our simple version)", hash == hash2);
    println!();

    // Verification
    println!("Verify correct password: {}", verify_password("hunter2", &salt, hash));
    println!("Verify wrong password:   {}", verify_password("wrong", &salt, hash));
    println!();

    // Comparison table
    println!("--- HashMap vs Password Hash Comparison ---");
    println!("{:<25} {:<25} {:<25}", "Property", "HashMap Hash", "Password Hash (Argon2)");
    println!("{}", "-".repeat(75));
    println!("{:<25} {:<25} {:<25}", "Speed", "Fast (nanoseconds)", "Slow (100ms+)");
    println!("{:<25} {:<25} {:<25}", "Deterministic", "Yes", "Yes (with same salt)");
    println!("{:<25} {:<25} {:<25}", "Collision resistance", "Low priority", "Critical");
    println!("{:<25} {:<25} {:<25}", "Reversible", "No", "No");
    println!("{:<25} {:<25} {:<25}", "Salt", "Not used", "Required");
    println!();

    // Part 3: Collision analysis
    println!("--- Part 3: Hash Collision Analysis ---");
    let exercise_names: Vec<String> = vec![
        "Back Squat", "Front Squat", "Overhead Squat", "Deadlift",
        "Bench Press", "Pull-Up", "Muscle-Up", "Thruster",
        "Clean", "Snatch", "Box Jump", "Burpee",
        "Wall Ball", "Rowing", "Running", "Kettlebell Swing",
    ].into_iter().map(String::from).collect();

    for num_buckets in [4, 8, 16, 32] {
        let (collisions, max) = collision_analysis(&exercise_names, num_buckets);
        println!(
            "  {} items -> {} buckets: {} collision buckets, max chain length {}",
            exercise_names.len(), num_buckets, collisions, max
        );
    }
    println!("  More buckets = fewer collisions = closer to O(1)");
    println!("  HashMap resizes when load factor exceeds ~0.75");
    println!();

    // Part 4: Two Sum interview problem
    println!("--- Part 4: Two Sum (Classic HashMap Interview Problem) ---");

    // Fitness context: find two exercises whose weights sum to a target
    let weights = vec![135, 95, 225, 155, 185, 65, 45, 275];
    let target = 320;
    println!("Weights: {:?}", weights);
    println!("Target sum: {}", target);
    match two_sum(&weights, target) {
        Some((i, j)) => println!(
            "Found: weights[{}]={} + weights[{}]={} = {}",
            i, weights[i], j, weights[j], target
        ),
        None => println!("No pair found"),
    }

    let target2 = 180;
    println!("\nTarget sum: {}", target2);
    match two_sum(&weights, target2) {
        Some((i, j)) => println!(
            "Found: weights[{}]={} + weights[{}]={} = {}",
            i, weights[i], j, weights[j], target2
        ),
        None => println!("No pair found"),
    }

    println!("\nTwo Sum: O(n) time with HashMap vs O(n^2) brute force");
    println!("Pattern: store complements as you scan, check each new element against stored values");
}
