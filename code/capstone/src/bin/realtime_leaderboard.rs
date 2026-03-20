// Problem 3: Real-time Leaderboard — Heap / Priority Queue
// Top-K scores with lazy deletion for updates.
// Run with: cargo run --bin realtime_leaderboard

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

// --- Brute Force ---

struct LeaderboardBrute {
    scores: HashMap<String, u32>,
}

impl LeaderboardBrute {
    fn new() -> Self {
        Self { scores: HashMap::new() }
    }

    fn update(&mut self, athlete: &str, score: u32) {
        let entry = self.scores.entry(athlete.to_string()).or_insert(0);
        *entry = score.max(*entry);
    }

    fn top_k(&self, k: usize) -> Vec<(String, u32)> {
        let mut sorted: Vec<_> = self.scores.iter()
            .map(|(name, &score)| (name.clone(), score))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(k);
        sorted
    }
}

// --- Optimized: Heap with lazy deletion ---

#[derive(Eq, PartialEq, Clone)]
struct Entry {
    score: u32,
    athlete: String,
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct Leaderboard {
    heap: BinaryHeap<Entry>,
    current_scores: HashMap<String, u32>,
}

impl Leaderboard {
    fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            current_scores: HashMap::new(),
        }
    }

    fn update(&mut self, athlete: &str, score: u32) {
        let entry = self.current_scores.entry(athlete.to_string()).or_insert(0);
        *entry = score.max(*entry);
        let current = *entry;
        self.heap.push(Entry {
            score: current,
            athlete: athlete.to_string(),
        });
    }

    fn top_k(&self, k: usize) -> Vec<(String, u32)> {
        let mut heap = self.heap.clone();
        let mut results = Vec::with_capacity(k);
        let mut seen = HashSet::new();

        while results.len() < k {
            match heap.pop() {
                None => break,
                Some(entry) => {
                    if seen.contains(&entry.athlete) {
                        continue;
                    }
                    if let Some(&current) = self.current_scores.get(&entry.athlete) {
                        if entry.score == current {
                            seen.insert(entry.athlete.clone());
                            results.push((entry.athlete, entry.score));
                        }
                    }
                }
            }
        }
        results
    }
}

fn main() {
    println!("=== Brute Force ===");
    let mut brute = LeaderboardBrute::new();
    brute.update("Alice", 185);
    brute.update("Bob", 210);
    brute.update("Carol", 195);
    brute.update("Dave", 225);
    brute.update("Alice", 230);

    for (name, score) in brute.top_k(3) {
        println!("  {}: {} lbs", name, score);
    }

    println!("\n=== Heap (optimized) ===");
    let mut lb = Leaderboard::new();
    lb.update("Alice", 185);
    lb.update("Bob", 210);
    lb.update("Carol", 195);
    lb.update("Dave", 225);
    lb.update("Alice", 230);

    for (name, score) in lb.top_k(3) {
        println!("  {}: {} lbs", name, score);
    }
}
