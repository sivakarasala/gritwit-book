// Chapter 10 DSA Exercise: Greedy + Custom Comparators
//
// Streak as longest consecutive sequence (LeetCode 128 variant).
// Multi-criteria sort with Ordering chains for leaderboard ranking.

use std::collections::HashSet;
use std::cmp::Ordering;

// ----------------------------------------------------------------
// Part 1: Streak calculation — greedy consecutive day algorithm
// GrindIt's streak_days_db walks sorted dates from most recent,
// counting consecutive days anchored at today.
// ----------------------------------------------------------------

/// Simulates GrindIt's streak calculation.
/// Dates are represented as day-of-year integers (sorted descending).
/// Returns the current streak ending at `today` or `today - 1`.
fn streak_from_today(dates: &[i32], today: i32) -> i32 {
    if dates.is_empty() {
        return 0;
    }

    // dates should be sorted descending (most recent first)
    let mut sorted = dates.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    sorted.dedup();

    let mut streak = 0;
    let mut expected = today;

    for &date in &sorted {
        // Allow starting from yesterday if today has no workout
        if streak == 0 && date == today - 1 {
            expected = today - 1;
        }
        if date == expected {
            streak += 1;
            expected -= 1;
        } else if date < expected {
            break; // gap found — streak ends
        }
    }
    streak
}

// ----------------------------------------------------------------
// Part 2: LeetCode 128 — Longest Consecutive Sequence
// Given an unsorted array, find the length of the longest consecutive
// sequence. O(n) using HashSet.
// ----------------------------------------------------------------

fn longest_consecutive(nums: Vec<i32>) -> i32 {
    let set: HashSet<i32> = nums.into_iter().collect();
    let mut best = 0;

    for &n in &set {
        // Only start counting from sequence beginnings
        if !set.contains(&(n - 1)) {
            let mut current = n;
            let mut length = 1;
            while set.contains(&(current + 1)) {
                current += 1;
                length += 1;
            }
            best = best.max(length);
        }
    }
    best
}

/// Variant: find the longest training streak in a list of dates (day numbers)
fn longest_streak(mut dates: Vec<i32>) -> i32 {
    if dates.is_empty() {
        return 0;
    }
    dates.sort_unstable();
    dates.dedup();

    let mut best = 1;
    let mut current = 1;

    for window in dates.windows(2) {
        if window[1] == window[0] + 1 {
            current += 1;
            best = best.max(current);
        } else {
            current = 1;
        }
    }
    best
}

// ----------------------------------------------------------------
// Part 3: Multi-criteria sort for leaderboard
// Sort athletes: Rx first, then by score descending, then by name.
// Uses Ordering::then_with() chains — lazy evaluation.
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
struct LeaderboardEntry {
    name: String,
    workouts_this_week: i32,
    total_score: f64,
    is_rx: bool,
}

/// Sort leaderboard: Rx first, then by workouts desc, then by score desc, then by name asc
fn sort_leaderboard(entries: &mut Vec<LeaderboardEntry>) {
    entries.sort_by(|a, b| {
        // Rx athletes rank above scaled (true > false → reverse for Rx-first)
        b.is_rx
            .cmp(&a.is_rx)
            .then_with(|| {
                // More workouts = higher rank
                b.workouts_this_week.cmp(&a.workouts_this_week)
            })
            .then_with(|| {
                // Higher score = higher rank
                b.total_score
                    .partial_cmp(&a.total_score)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                // Alphabetical tiebreaker
                a.name.cmp(&b.name)
            })
    });
}

// ----------------------------------------------------------------
// Part 4: Greedy interval scheduling — another greedy algorithm
// Given workout time slots, find the maximum non-overlapping sessions.
// ----------------------------------------------------------------

#[derive(Debug, Clone)]
struct TimeSlot {
    name: String,
    start: u32, // minutes from midnight
    end: u32,
}

impl TimeSlot {
    fn display_time(minutes: u32) -> String {
        let h = minutes / 60;
        let m = minutes % 60;
        let period = if h >= 12 { "PM" } else { "AM" };
        let h12 = if h > 12 { h - 12 } else if h == 0 { 12 } else { h };
        format!("{}:{:02}{}", h12, m, period)
    }
}

/// Greedy: select maximum non-overlapping time slots.
/// Sort by end time, greedily pick the earliest-ending non-overlapping slot.
fn max_non_overlapping(slots: &mut Vec<TimeSlot>) -> Vec<TimeSlot> {
    slots.sort_by_key(|s| s.end);
    let mut selected = Vec::new();
    let mut last_end = 0;

    for slot in slots.iter() {
        if slot.start >= last_end {
            selected.push(slot.clone());
            last_end = slot.end;
        }
    }
    selected
}

fn main() {
    println!("=== Greedy Algorithms & Custom Comparators ===\n");

    // Part 1: Streak calculation
    println!("--- Part 1: Current Training Streak ---");
    let today = 80; // day 80 of the year

    let test_cases = vec![
        ("5-day streak ending today", vec![76, 77, 78, 79, 80], today),
        ("3-day streak ending yesterday", vec![75, 77, 78, 79], today),
        ("Broken streak (gap at day 78)", vec![76, 77, 79, 80], today),
        ("No workouts", vec![], today),
        ("Only today", vec![80], today),
    ];

    for (label, dates, today) in &test_cases {
        let streak = streak_from_today(dates, *today);
        println!("  {}: streak = {} days", label, streak);
    }

    // Part 2: Longest consecutive sequence (LeetCode 128)
    println!("\n--- Part 2: Longest Consecutive Sequence ---");

    let workout_days = vec![1, 2, 3, 10, 11, 12, 13, 14, 20, 30, 31, 32];
    println!("Workout days: {:?}", workout_days);
    println!(
        "Longest consecutive streak: {} days (days 10-14)",
        longest_consecutive(workout_days.clone())
    );

    let unsorted_days = vec![100, 4, 200, 1, 3, 2, 50, 51, 52, 53, 54, 55];
    println!("\nUnsorted days: {:?}", unsorted_days);
    println!(
        "Longest consecutive: {} (days 50-55)",
        longest_consecutive(unsorted_days.clone())
    );

    // Using the sorting approach
    println!(
        "Longest streak (sort approach): {}",
        longest_streak(workout_days)
    );

    println!("\nTime complexity comparison:");
    println!("  HashSet approach: O(n) — each element visited at most twice");
    println!("  Sort approach: O(n log n) — dominated by the sort step");
    println!("  GrindIt approach: O(n) — dates come pre-sorted from database");

    // Part 3: Multi-criteria leaderboard sort
    println!("\n--- Part 3: Leaderboard with Custom Comparators ---");
    let mut entries = vec![
        LeaderboardEntry { name: "Alice".to_string(), workouts_this_week: 5, total_score: 1250.0, is_rx: true },
        LeaderboardEntry { name: "Bob".to_string(), workouts_this_week: 5, total_score: 1100.0, is_rx: true },
        LeaderboardEntry { name: "Carol".to_string(), workouts_this_week: 6, total_score: 1400.0, is_rx: false },
        LeaderboardEntry { name: "Dave".to_string(), workouts_this_week: 5, total_score: 1250.0, is_rx: true },
        LeaderboardEntry { name: "Eve".to_string(), workouts_this_week: 4, total_score: 900.0, is_rx: true },
        LeaderboardEntry { name: "Frank".to_string(), workouts_this_week: 6, total_score: 1500.0, is_rx: true },
    ];

    sort_leaderboard(&mut entries);

    println!("  {:<5} {:<10} {:>8} {:>10} {:>5}", "Rank", "Name", "WODs", "Score", "Rx");
    println!("  {}", "-".repeat(42));
    for (i, entry) in entries.iter().enumerate() {
        println!(
            "  {:<5} {:<10} {:>8} {:>10.0} {:>5}",
            format!("#{}", i + 1),
            entry.name,
            entry.workouts_this_week,
            entry.total_score,
            if entry.is_rx { "Rx" } else { "Scaled" }
        );
    }

    println!("\n  Sort criteria (in order):");
    println!("  1. Rx > Scaled");
    println!("  2. More workouts > fewer workouts");
    println!("  3. Higher score > lower score");
    println!("  4. Alphabetical name (tiebreaker)");
    println!("  Note: .then_with() is lazy — later criteria only evaluated on ties.");

    // Part 4: Greedy interval scheduling
    println!("\n--- Part 4: Greedy Interval Scheduling ---");
    println!("Maximize non-overlapping workout sessions:\n");

    let mut slots = vec![
        TimeSlot { name: "Yoga".to_string(), start: 6 * 60, end: 7 * 60 },
        TimeSlot { name: "CrossFit".to_string(), start: 6 * 60 + 30, end: 7 * 60 + 30 },
        TimeSlot { name: "Spin".to_string(), start: 7 * 60, end: 7 * 60 + 45 },
        TimeSlot { name: "Weightlifting".to_string(), start: 8 * 60, end: 9 * 60 + 30 },
        TimeSlot { name: "HIIT".to_string(), start: 9 * 60, end: 9 * 60 + 30 },
        TimeSlot { name: "Boxing".to_string(), start: 10 * 60, end: 11 * 60 },
        TimeSlot { name: "Swimming".to_string(), start: 9 * 60 + 30, end: 10 * 60 + 15 },
    ];

    println!("  Available slots:");
    for slot in &slots {
        println!(
            "    {} - {} : {}",
            TimeSlot::display_time(slot.start),
            TimeSlot::display_time(slot.end),
            slot.name
        );
    }

    let selected = max_non_overlapping(&mut slots);
    println!("\n  Maximum non-overlapping ({} sessions):", selected.len());
    for slot in &selected {
        println!(
            "    {} - {} : {}",
            TimeSlot::display_time(slot.start),
            TimeSlot::display_time(slot.end),
            slot.name
        );
    }
    println!("\n  Greedy strategy: always pick the session that ends earliest.");
    println!("  This guarantees the maximum number of non-overlapping sessions.");
}
