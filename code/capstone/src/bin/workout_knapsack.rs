// Problem 1: Workout Knapsack — Dynamic Programming
// Maximize training benefit within a 60-minute time cap.
// Run with: cargo run --bin workout_knapsack

fn max_benefit_brute(exercises: &[(&str, u32, u32)], time_cap: u32) -> u32 {
    let n = exercises.len();
    let mut best = 0;

    for mask in 0..(1u64 << n) {
        let mut total_time = 0;
        let mut total_benefit = 0;
        for i in 0..n {
            if mask & (1 << i) != 0 {
                total_time += exercises[i].1;
                total_benefit += exercises[i].2;
            }
        }
        if total_time <= time_cap {
            best = best.max(total_benefit);
        }
    }
    best
}

fn max_benefit(exercises: &[(&str, u32, u32)], time_cap: u32) -> u32 {
    let cap = time_cap as usize;
    let mut dp = vec![0u32; cap + 1];

    for &(_name, duration, benefit) in exercises {
        let dur = duration as usize;
        for t in (dur..=cap).rev() {
            dp[t] = dp[t].max(dp[t - dur] + benefit);
        }
    }
    dp[cap]
}

fn main() {
    let exercises = vec![
        ("Back Squat", 12, 8),
        ("Deadlift", 15, 10),
        ("Box Jumps", 8, 5),
        ("Pull-ups", 10, 6),
        ("Rowing", 20, 12),
        ("Wall Balls", 7, 4),
        ("Burpees", 5, 3),
    ];

    let brute = max_benefit_brute(&exercises, 60);
    let optimized = max_benefit(&exercises, 60);

    println!("Brute force: maximum benefit = {}", brute);
    println!("DP optimized: maximum benefit = {}", optimized);
    assert_eq!(brute, optimized);
    println!("Both approaches agree!");
}
