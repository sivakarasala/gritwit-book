// Problem 8: Progression Path — BFS / Dijkstra
// Find easiest progression from beginner to advanced exercises.
// Run with: cargo run --bin progression_path

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Reverse;

// --- Brute Force: BFS with relaxation ---

fn shortest_path_bfs(
    edges: &[(&str, &str, u32)],
    start: &str,
    end: &str,
) -> Option<(u32, Vec<String>)> {
    let mut adj: HashMap<&str, Vec<(&str, u32)>> = HashMap::new();
    for &(from, to, cost) in edges {
        adj.entry(from).or_default().push((to, cost));
    }

    let mut best_cost: HashMap<&str, u32> = HashMap::new();
    let mut best_path: HashMap<&str, Vec<String>> = HashMap::new();
    let mut queue = VecDeque::new();

    best_cost.insert(start, 0);
    best_path.insert(start, vec![start.to_string()]);
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        let current_cost = best_cost[current];
        if let Some(neighbors) = adj.get(current) {
            for &(next, edge_cost) in neighbors {
                let new_cost = current_cost + edge_cost;
                if !best_cost.contains_key(next) || new_cost < best_cost[next] {
                    best_cost.insert(next, new_cost);
                    let mut path = best_path[current].clone();
                    path.push(next.to_string());
                    best_path.insert(next, path);
                    queue.push_back(next);
                }
            }
        }
    }

    best_cost
        .get(end)
        .map(|&cost| (cost, best_path[end].clone()))
}

// --- Optimized: Dijkstra with min-heap ---

fn shortest_path(
    edges: &[(&str, &str, u32)],
    start: &str,
    end: &str,
) -> Option<(u32, Vec<String>)> {
    let mut adj: HashMap<&str, Vec<(&str, u32)>> = HashMap::new();
    for &(from, to, cost) in edges {
        adj.entry(from).or_default().push((to, cost));
    }

    let mut heap = BinaryHeap::new();
    let mut dist: HashMap<&str, u32> = HashMap::new();
    let mut prev: HashMap<&str, &str> = HashMap::new();

    heap.push(Reverse((0u32, start)));
    dist.insert(start, 0);

    while let Some(Reverse((cost, node))) = heap.pop() {
        if let Some(&best) = dist.get(node) {
            if cost > best {
                continue;
            }
        }

        if node == end {
            let mut path = vec![end.to_string()];
            let mut current = end;
            while let Some(&p) = prev.get(current) {
                path.push(p.to_string());
                current = p;
            }
            path.reverse();
            return Some((cost, path));
        }

        if let Some(neighbors) = adj.get(node) {
            for &(next, edge_cost) in neighbors {
                let new_cost = cost + edge_cost;
                if !dist.contains_key(next) || new_cost < dist[next] {
                    dist.insert(next, new_cost);
                    prev.insert(next, node);
                    heap.push(Reverse((new_cost, next)));
                }
            }
        }
    }

    None
}

fn main() {
    let edges = vec![
        ("Air Squat", "Goblet Squat", 2),
        ("Air Squat", "Jump Squat", 3),
        ("Goblet Squat", "Front Squat", 3),
        ("Goblet Squat", "Bulgarian Split Squat", 4),
        ("Jump Squat", "Box Jump", 2),
        ("Front Squat", "Overhead Squat", 4),
        ("Bulgarian Split Squat", "Pistol Squat", 5),
        ("Front Squat", "Pistol Squat", 7),
        ("Overhead Squat", "Pistol Squat", 3),
    ];

    println!("=== BFS (brute force) ===");
    match shortest_path_bfs(&edges, "Air Squat", "Pistol Squat") {
        Some((cost, path)) => {
            println!("  Cost: {}", cost);
            println!("  Path: {}", path.join(" -> "));
        }
        None => println!("  No path found."),
    }

    println!("\n=== Dijkstra (optimized) ===");
    match shortest_path(&edges, "Air Squat", "Pistol Squat") {
        Some((cost, path)) => {
            println!("  Cost: {}", cost);
            println!("  Path: {}", path.join(" -> "));
        }
        None => println!("  No path found."),
    }
}
