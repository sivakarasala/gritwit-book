// Problem 4: Movement Prerequisites — Topological Sort
// Find valid learning order for exercises with prerequisites.
// Run with: cargo run --bin movement_prerequisites

use std::collections::{HashMap, HashSet, VecDeque};

// --- Brute Force ---

fn topo_sort_brute<'a>(
    exercises: &[&'a str],
    prereqs: &[(&'a str, &'a str)],
) -> Option<Vec<String>> {
    let mut remaining: Vec<&str> = exercises.to_vec();
    let mut result = Vec::new();
    let mut learned: HashSet<&str> = HashSet::new();

    while !remaining.is_empty() {
        let mut progress = false;
        remaining.retain(|&ex| {
            let all_met = prereqs
                .iter()
                .filter(|(_, dep)| *dep == ex)
                .all(|(pre, _)| learned.contains(pre));
            if all_met {
                result.push(ex.to_string());
                learned.insert(ex);
                progress = true;
                false
            } else {
                true
            }
        });
        if !progress {
            return None;
        }
    }
    Some(result)
}

// --- Optimized: Kahn's Algorithm ---

fn topo_sort<'a>(
    exercises: &[&'a str],
    prereqs: &[(&'a str, &'a str)],
) -> Option<Vec<String>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();

    for &ex in exercises {
        adj.entry(ex).or_default();
        in_degree.entry(ex).or_insert(0);
    }

    for &(pre, dep) in prereqs {
        adj.entry(pre).or_default().push(dep);
        *in_degree.entry(dep).or_insert(0) += 1;
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&ex, _)| ex)
        .collect();

    let mut result = Vec::new();

    while let Some(current) = queue.pop_front() {
        result.push(current.to_string());
        if let Some(dependents) = adj.get(current) {
            for &dep in dependents {
                let deg = in_degree.get_mut(dep).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(dep);
                }
            }
        }
    }

    if result.len() == exercises.len() {
        Some(result)
    } else {
        None
    }
}

fn main() {
    let exercises = vec![
        "Strict Pull-up", "Kipping Pull-up", "Butterfly Pull-up",
        "Muscle-up", "Air Squat", "Front Squat", "Overhead Squat",
        "Strict Press", "Push Press", "Push Jerk", "Split Jerk",
    ];

    let prereqs = vec![
        ("Strict Pull-up", "Kipping Pull-up"),
        ("Kipping Pull-up", "Butterfly Pull-up"),
        ("Kipping Pull-up", "Muscle-up"),
        ("Air Squat", "Front Squat"),
        ("Front Squat", "Overhead Squat"),
        ("Strict Press", "Push Press"),
        ("Push Press", "Push Jerk"),
        ("Push Jerk", "Split Jerk"),
    ];

    println!("=== Brute Force ===");
    match topo_sort_brute(&exercises, &prereqs) {
        Some(order) => {
            for (i, ex) in order.iter().enumerate() {
                println!("  {}. {}", i + 1, ex);
            }
        }
        None => println!("  Cycle detected!"),
    }

    println!("\n=== Kahn's Algorithm ===");
    match topo_sort(&exercises, &prereqs) {
        Some(order) => {
            for (i, ex) in order.iter().enumerate() {
                println!("  {}. {}", i + 1, ex);
            }
        }
        None => println!("  Cycle detected!"),
    }
}
