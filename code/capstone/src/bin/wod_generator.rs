// Problem 6: WOD Generator — Backtracking
// Generate valid 3-exercise WODs with muscle group and time constraints.
// Run with: cargo run --bin wod_generator

use std::collections::HashSet;

fn generate_wods_brute<'a>(exercises: &'a [(&'a str, &'a str, u32)]) -> Vec<Vec<&'a str>> {
    let n = exercises.len();
    let mut results = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let groups: HashSet<&str> =
                    [exercises[i].1, exercises[j].1, exercises[k].1]
                        .into_iter()
                        .collect();

                if groups.len() != 3 {
                    continue;
                }

                let total_time = exercises[i].2 + exercises[j].2 + exercises[k].2;
                if total_time >= 15 && total_time <= 25 {
                    results.push(vec![exercises[i].0, exercises[j].0, exercises[k].0]);
                }
            }
        }
    }
    results
}

fn generate_wods(exercises: &[(&str, &str, u32)]) -> Vec<Vec<String>> {
    let mut results = Vec::new();
    let mut current = Vec::new();
    let mut used_groups = HashSet::new();

    backtrack(exercises, 0, &mut current, &mut used_groups, 0, &mut results);
    results
}

fn backtrack(
    exercises: &[(&str, &str, u32)],
    start: usize,
    current: &mut Vec<(String, u32)>,
    used_groups: &mut HashSet<String>,
    current_time: u32,
    results: &mut Vec<Vec<String>>,
) {
    if current_time > 25 {
        return;
    }

    if current.len() == 3 {
        if current_time >= 15 {
            results.push(current.iter().map(|(name, _)| name.clone()).collect());
        }
        return;
    }

    let remaining_slots = 3 - current.len();
    if exercises.len() - start < remaining_slots {
        return;
    }

    for i in start..exercises.len() {
        let (name, group, time) = exercises[i];

        if used_groups.contains(group) {
            continue;
        }

        current.push((name.to_string(), time));
        used_groups.insert(group.to_string());

        backtrack(exercises, i + 1, current, used_groups, current_time + time, results);

        current.pop();
        used_groups.remove(group);
    }
}

fn main() {
    let exercises = vec![
        ("Deadlift", "posterior", 8),
        ("Back Squat", "legs", 7),
        ("Push Press", "shoulders", 6),
        ("Bench Press", "chest", 8),
        ("Pull-up", "back", 5),
        ("Box Jump", "legs", 4),
        ("Thruster", "full body", 9),
    ];

    println!("=== Brute Force ===");
    let brute = generate_wods_brute(&exercises);
    println!("Found {} valid WODs:", brute.len());
    for wod in &brute {
        println!("  {:?}", wod);
    }

    println!("\n=== Backtracking ===");
    let optimized = generate_wods(&exercises);
    println!("Found {} valid WODs:", optimized.len());
    for wod in &optimized {
        println!("  {:?}", wod);
    }

    assert_eq!(brute.len(), optimized.len());
    println!("\nBoth approaches found the same number of WODs!");
}
