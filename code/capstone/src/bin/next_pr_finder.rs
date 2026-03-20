// Problem 5: Next PR Finder — Monotonic Stack
// For each session, find how many sessions until a heavier lift.
// Run with: cargo run --bin next_pr_finder

fn next_pr_brute(weights: &[u32]) -> Vec<i32> {
    let n = weights.len();
    let mut result = vec![-1i32; n];

    for i in 0..n {
        for j in (i + 1)..n {
            if weights[j] > weights[i] {
                result[i] = (j - i) as i32;
                break;
            }
        }
    }
    result
}

fn next_pr(weights: &[u32]) -> Vec<i32> {
    let n = weights.len();
    let mut result = vec![-1i32; n];
    let mut stack: Vec<usize> = Vec::new();

    for i in 0..n {
        while let Some(&top) = stack.last() {
            if weights[i] > weights[top] {
                result[top] = (i - top) as i32;
                stack.pop();
            } else {
                break;
            }
        }
        stack.push(i);
    }
    result
}

fn main() {
    let weights = vec![225, 245, 235, 255, 250, 275, 265, 295];

    let brute = next_pr_brute(&weights);
    let optimized = next_pr(&weights);

    println!("Sessions:  {:?}", weights);
    println!("Brute:     {:?}", brute);
    println!("Optimized: {:?}", optimized);
    assert_eq!(brute, optimized);

    println!("\nDetailed:");
    for (i, &days) in optimized.iter().enumerate() {
        let label = if days == -1 {
            "never beaten".to_string()
        } else {
            format!("{} sessions later", days)
        };
        println!("  Session {} ({} lbs): {}", i, weights[i], label);
    }
}
