// Chapter 10: History & Leaderboard
// Spotlight: Collections & Sorting Deep Dive
//
// HashMap grouping, streak calculation, custom comparators.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct WorkoutEntry {
    pub user_name: String,
    pub wod_date: String,
    pub score: String,
    pub section_type: String,
    pub rx: bool,
}

/// Group workout logs by date using HashMap
pub fn group_by_date(entries: &[WorkoutEntry]) -> HashMap<String, Vec<&WorkoutEntry>> {
    let mut groups: HashMap<String, Vec<&WorkoutEntry>> = HashMap::new();
    for entry in entries {
        groups.entry(entry.wod_date.clone()).or_default().push(entry);
    }
    groups
}

/// Calculate streak: count of consecutive days with workouts
/// Uses the greedy algorithm — check each day backwards from today
pub fn calculate_streak(workout_dates: &[chrono::NaiveDate]) -> i32 {
    if workout_dates.is_empty() {
        return 0;
    }

    let mut sorted = workout_dates.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut streak = 1;
    // Walk backwards from the most recent date
    for i in (0..sorted.len() - 1).rev() {
        let diff = sorted[i + 1].signed_duration_since(sorted[i]).num_days();
        if diff == 1 {
            streak += 1;
        } else {
            break;
        }
    }
    streak
}

/// Sort leaderboard entries with custom comparator:
/// 1. Rx before Scaled
/// 2. Better score (lower time for ForTime, higher for AMRAP/Strength)
/// 3. Alphabetical name as tiebreaker
pub fn sort_leaderboard(entries: &mut [WorkoutEntry], section_type: &str) {
    entries.sort_by(|a, b| {
        // Rx first
        let rx_cmp = b.rx.cmp(&a.rx);
        if rx_cmp != std::cmp::Ordering::Equal {
            return rx_cmp;
        }

        // Score comparison depends on section type
        let a_val: i32 = a.score.parse().unwrap_or(0);
        let b_val: i32 = b.score.parse().unwrap_or(0);

        let score_cmp = match section_type {
            "ForTime" => a_val.cmp(&b_val),     // Lower is better
            _ => b_val.cmp(&a_val),              // Higher is better
        };

        if score_cmp != std::cmp::Ordering::Equal {
            return score_cmp;
        }

        // Tiebreaker: alphabetical
        a.user_name.cmp(&b.user_name)
    });
}
