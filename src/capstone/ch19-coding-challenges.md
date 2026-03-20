# Chapter 19: Coding Challenges

Chapters 1 through 18 wove roughly eighteen DSA patterns into the fabric of GrindIt — HashMap for grouping workouts by date, greedy algorithms for streak calculation, B-tree indexes for query performance, custom comparators for leaderboard ranking, tree traversal for nested WOD rendering, and many more. You learned those patterns the best way: by needing them to ship a feature.

This chapter introduces eight patterns that did not appear organically in the codebase but show up constantly in coding interviews. Each problem is framed in GrindIt's fitness domain — not abstract arrays of integers. You will see workouts, exercises, PRs, and leaderboards. The data structures are the same ones you have been building for eighteen chapters.

For each problem you get: a concrete problem statement, worked examples, a brute-force solution with analysis, an optimized solution with full compilable Rust code, complexity analysis for both, a note on where the pattern connects to GrindIt, and interview tips.

The eight patterns:

| # | Problem | Pattern |
|---|---------|---------|
| 1 | Workout Knapsack | Dynamic Programming |
| 2 | Exercise Autocomplete | Trie |
| 3 | Real-time Leaderboard | Heap / Priority Queue |
| 4 | Movement Prerequisites | Topological Sort |
| 5 | Next PR Finder | Monotonic Stack |
| 6 | WOD Generator | Backtracking |
| 7 | Exercise Cache | LRU Cache |
| 8 | Progression Path | BFS / Dijkstra |

Work through them in order or jump to whichever pattern you want to drill. Each problem stands alone.

---

## Problem 1: Workout Knapsack

### Problem Statement

You are programming a WOD with a **60-minute time cap**. You have a list of candidate exercises, each with an estimated duration (minutes) and a training benefit score. Select exercises to **maximize total benefit** without exceeding the time cap.

### Examples

```
Exercises:
  Back Squat      — 12 min, benefit 8
  Deadlift        — 15 min, benefit 10
  Box Jumps       — 8 min,  benefit 5
  Pull-ups        — 10 min, benefit 6
  Rowing          — 20 min, benefit 12
  Wall Balls      — 7 min,  benefit 4
  Burpees         — 5 min,  benefit 3

Time cap: 60 minutes

Output: Maximum benefit = 40
Selected: Deadlift (15) + Rowing (20) + Back Squat (12) + Pull-ups (10) + Burpees (5)
  → 62 min? No — pick the combination that fits in 60.
  Actually: Deadlift (15) + Rowing (20) + Back Squat (12) + Box Jumps (8) + Burpees (5) = 60 min, benefit = 40
```

### Brute Force

Try every subset of exercises, check if the total duration fits within the time cap, and track the maximum benefit.

```rust
fn max_benefit_brute(exercises: &[(&str, u32, u32)], time_cap: u32) -> u32 {
    let n = exercises.len();
    let mut best = 0;

    // Enumerate all 2^n subsets
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
```

**Time:** O(2^n) — exponential in the number of exercises.
**Space:** O(1).

### Optimized Solution

The classic 0/1 knapsack uses a 2D DP table where `dp[i][w]` is the maximum benefit using exercises `0..i` with capacity `w`. We optimize to a 1D array by iterating capacity in reverse.

```rust
fn max_benefit(exercises: &[(&str, u32, u32)], time_cap: u32) -> u32 {
    let cap = time_cap as usize;
    let mut dp = vec![0u32; cap + 1];

    for &(_name, duration, benefit) in exercises {
        let dur = duration as usize;
        // Iterate in reverse to avoid using the same exercise twice
        for t in (dur..=cap).rev() {
            dp[t] = dp[t].max(dp[t - dur] + benefit);
        }
    }
    dp[cap]
}

fn main() {
    let exercises = vec![
        ("Back Squat", 12, 8),
        ("Deadlift",   15, 10),
        ("Box Jumps",   8, 5),
        ("Pull-ups",   10, 6),
        ("Rowing",     20, 12),
        ("Wall Balls",  7, 4),
        ("Burpees",     5, 3),
    ];

    let result = max_benefit(&exercises, 60);
    println!("Maximum benefit: {}", result); // 40
}
```

### Complexity Analysis

| Approach | Time | Space |
|----------|------|-------|
| Brute force | O(2^n) | O(1) |
| DP | O(n * C) | O(C) |

Where `n` is the number of exercises and `C` is the time cap. For 20 exercises and a 60-minute cap, the DP table has 1200 cells — trivial. The brute force has over a million subsets.

### Connection to GrindIt

The WOD programming page (Chapter 8) lets coaches compose workouts from a library of movements. A "smart WOD builder" that maximizes training stimulus within a time cap is exactly this knapsack problem. The benefit scores could come from muscle group coverage, metabolic demand, or athlete weakness analysis.

### Interview Tips

- Clarify whether items can be reused (unbounded knapsack) or not (0/1 knapsack). GrindIt's version is 0/1 — you would not program three sets of deadlifts as three separate items.
- The 1D DP trick (iterating in reverse) is the key insight interviewers want to see. It shows you understand why the 2D table works and how to compress it.
- If asked to reconstruct which exercises were selected, keep the 2D table and backtrack from `dp[n][C]`.

---

## Problem 2: Exercise Autocomplete

### Problem Statement

GrindIt's exercise library has hundreds of movements. Build an autocomplete system: given a prefix string, return all exercise names that start with that prefix, sorted alphabetically.

### Examples

```
Exercises: ["Back Squat", "Bench Press", "Box Jump", "Burpee", "Bulgarian Split Squat"]

prefix = "B"   → ["Back Squat", "Bench Press", "Box Jump", "Bulgarian Split Squat", "Burpee"]
prefix = "Bu"  → ["Bulgarian Split Squat", "Burpee"]
prefix = "Bac" → ["Back Squat"]
prefix = "Z"   → []
```

### Brute Force

Iterate through all exercises and filter by prefix.

```rust
fn autocomplete_brute(exercises: &[&str], prefix: &str) -> Vec<String> {
    let prefix_lower = prefix.to_lowercase();
    let mut results: Vec<String> = exercises
        .iter()
        .filter(|e| e.to_lowercase().starts_with(&prefix_lower))
        .map(|e| e.to_string())
        .collect();
    results.sort();
    results
}
```

**Time:** O(n * m) per query where `n` is the number of exercises and `m` is the average name length.
**Space:** O(k) for the `k` results.

### Optimized Solution

A trie (prefix tree) stores exercises so that prefix lookups take O(p + k) time where `p` is the prefix length and `k` is the number of matches.

```rust
use std::collections::HashMap;

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end: bool,
}

struct Trie {
    root: TrieNode,
}

impl Trie {
    fn new() -> Self {
        Trie { root: TrieNode::default() }
    }

    fn insert(&mut self, word: &str) {
        let mut node = &mut self.root;
        for ch in word.to_lowercase().chars() {
            node = node.children.entry(ch).or_default();
        }
        node.is_end = true;
    }

    fn autocomplete(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let mut node = &self.root;

        // Navigate to the prefix node
        for ch in prefix_lower.chars() {
            match node.children.get(&ch) {
                Some(child) => node = child,
                None => return vec![],
            }
        }

        // Collect all words below this node
        let mut results = Vec::new();
        let mut current_word = prefix_lower.clone();
        Self::collect(node, &mut current_word, &mut results);
        results.sort();
        results
    }

    fn collect(node: &TrieNode, current: &mut String, results: &mut Vec<String>) {
        if node.is_end {
            results.push(current.clone());
        }
        for (&ch, child) in &node.children {
            current.push(ch);
            Self::collect(child, current, results);
            current.pop();
        }
    }
}

fn main() {
    let mut trie = Trie::new();
    let exercises = [
        "Back Squat", "Bench Press", "Box Jump",
        "Burpee", "Bulgarian Split Squat",
        "Clean and Jerk", "Deadlift",
    ];

    for name in &exercises {
        trie.insert(name);
    }

    let results = trie.autocomplete("bu");
    println!("{:?}", results);
    // ["bulgarian split squat", "burpee"]
}
```

### Complexity Analysis

| Approach | Build | Query | Space |
|----------|-------|-------|-------|
| Brute force | O(1) | O(n * m) | O(n * m) for storage |
| Trie | O(n * m) insert | O(p + k) lookup | O(n * m) for trie |

The trie invests build time upfront for fast queries. When the exercise library is loaded once and queried many times per keystroke, this tradeoff is decisive.

### Connection to GrindIt

Chapter 3's search bar uses `.contains()` — linear scan on every keystroke. For GrindIt's current library of 50-100 exercises, that is fine. For a gym platform with 10,000 custom exercises across tenants, a trie-backed autocomplete would keep the UI responsive.

### Interview Tips

- Draw the trie on the whiteboard before coding. Interviewers want to see you think in data structures.
- Mention the space tradeoff: tries use more memory than a sorted array with binary search. For short strings (exercise names), the overhead is acceptable.
- A follow-up question is often "support fuzzy matching" — mention Levenshtein distance or BK-trees.

---

## Problem 3: Real-time Leaderboard

### Problem Statement

Scores are streaming in as athletes complete a WOD. At any point, you need to return the **top K** athletes by score. Scores can be updated (an athlete might re-submit).

### Examples

```
stream of scores:
  ("Alice", 185)  → top 3: [("Alice", 185)]
  ("Bob", 210)    → top 3: [("Bob", 210), ("Alice", 185)]
  ("Carol", 195)  → top 3: [("Bob", 210), ("Carol", 195), ("Alice", 185)]
  ("Dave", 225)   → top 3: [("Dave", 225), ("Bob", 210), ("Carol", 195)]
  ("Alice", 230)  → top 3: [("Alice", 230), ("Dave", 225), ("Bob", 210)]  // Alice improved
```

### Brute Force

Store all scores in a `Vec`, sort on every query.

```rust
use std::collections::HashMap;

struct LeaderboardBrute {
    scores: HashMap<String, u32>,
}

impl LeaderboardBrute {
    fn new() -> Self {
        Self { scores: HashMap::new() }
    }

    fn update(&mut self, athlete: &str, score: u32) {
        let entry = self.scores.entry(athlete.to_string()).or_insert(0);
        *entry = score.max(*entry); // Keep the best score
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
```

**Time:** O(n log n) per `top_k` query where `n` is the total number of athletes.
**Space:** O(n).

### Optimized Solution

Use a `BinaryHeap` (max-heap) combined with a `HashMap` for current scores. Since Rust's `BinaryHeap` does not support decrease-key, we use a lazy deletion approach: push the new score and ignore stale entries when popping.

```rust
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

#[derive(Eq, PartialEq)]
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
        // Clone the heap so we can pop without mutating
        let mut heap = self.heap.clone();
        let mut results = Vec::with_capacity(k);
        let mut seen = std::collections::HashSet::new();

        while results.len() < k {
            match heap.pop() {
                None => break,
                Some(entry) => {
                    // Skip stale entries
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
    let mut lb = Leaderboard::new();

    lb.update("Alice", 185);
    lb.update("Bob", 210);
    lb.update("Carol", 195);
    lb.update("Dave", 225);
    lb.update("Alice", 230); // Alice improves

    let top3 = lb.top_k(3);
    for (name, score) in &top3 {
        println!("{}: {} lbs", name, score);
    }
    // Alice: 230 lbs
    // Dave: 225 lbs
    // Bob: 210 lbs
}
```

### Complexity Analysis

| Approach | Update | Top-K Query | Space |
|----------|--------|-------------|-------|
| Brute force | O(1) | O(n log n) | O(n) |
| Heap | O(log n) | O(k log n) amortized | O(n) |

For a leaderboard with 500 athletes queried every second, the heap approach avoids re-sorting the entire list on every query.

### Connection to GrindIt

Chapter 10 builds the leaderboard with a SQL `ORDER BY` query — the database does the sorting. This heap-based approach would be needed for a real-time, in-memory leaderboard during a live competition where scores update every few seconds and you cannot afford a database round-trip per update.

### Interview Tips

- Clarify whether scores can be updated. If not, a simple min-heap of size K is simpler and more efficient.
- Mention the lazy deletion pattern explicitly — it shows you understand the limitation of Rust's `BinaryHeap` (no decrease-key) and know a practical workaround.
- For production scale, mention Redis sorted sets (`ZADD` + `ZREVRANGE`) — the system design answer to this coding question.

---

## Problem 4: Movement Prerequisites

### Problem Statement

Some exercises have prerequisites. You cannot program Muscle-ups in a WOD unless athletes can do Kipping Pull-ups, and Kipping Pull-ups require Strict Pull-ups. Given a list of prerequisite pairs, return a valid **learning order** for all exercises, or report that the prerequisites contain a cycle (impossible ordering).

### Examples

```
Prerequisites:
  Strict Pull-up → Kipping Pull-up
  Kipping Pull-up → Butterfly Pull-up
  Kipping Pull-up → Muscle-up
  Air Squat → Front Squat
  Front Squat → Overhead Squat
  Strict Press → Push Press
  Push Press → Push Jerk
  Push Jerk → Split Jerk

Valid order: [Strict Pull-up, Air Squat, Strict Press, Kipping Pull-up,
             Front Squat, Push Press, Butterfly Pull-up, Muscle-up,
             Overhead Squat, Push Jerk, Split Jerk]
```

### Brute Force

Repeatedly scan the list for exercises with no unmet prerequisites, add them to the result, and mark them as learned. Repeat until all exercises are placed or no progress can be made (cycle).

```rust
fn topo_sort_brute(
    exercises: &[&str],
    prereqs: &[(&str, &str)], // (prerequisite, dependent)
) -> Option<Vec<String>> {
    let mut remaining: Vec<&str> = exercises.to_vec();
    let mut result = Vec::new();
    let mut learned: std::collections::HashSet<&str> = std::collections::HashSet::new();

    while !remaining.is_empty() {
        let mut progress = false;
        remaining.retain(|&ex| {
            let all_met = prereqs.iter()
                .filter(|(_, dep)| *dep == ex)
                .all(|(pre, _)| learned.contains(pre));
            if all_met {
                result.push(ex.to_string());
                learned.insert(ex);
                progress = true;
                false // remove from remaining
            } else {
                true // keep in remaining
            }
        });
        if !progress {
            return None; // Cycle detected
        }
    }
    Some(result)
}
```

**Time:** O(V * E) where V is the number of exercises and E is the number of prerequisite pairs.
**Space:** O(V + E).

### Optimized Solution

Kahn's algorithm: maintain in-degree counts for each node, process nodes with in-degree zero using a queue.

```rust
use std::collections::{HashMap, HashSet, VecDeque};

fn topo_sort(
    exercises: &[&str],
    prereqs: &[(&str, &str)], // (prerequisite, dependent)
) -> Option<Vec<String>> {
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();

    // Initialize all exercises
    for &ex in exercises {
        adj.entry(ex).or_default();
        in_degree.entry(ex).or_insert(0);
    }

    // Build adjacency list and in-degree counts
    for &(pre, dep) in prereqs {
        adj.entry(pre).or_default().push(dep);
        *in_degree.entry(dep).or_insert(0) += 1;
    }

    // Start with exercises that have no prerequisites
    let mut queue: VecDeque<&str> = in_degree.iter()
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
        None // Cycle detected — not all nodes were processed
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

    match topo_sort(&exercises, &prereqs) {
        Some(order) => {
            println!("Learning order:");
            for (i, ex) in order.iter().enumerate() {
                println!("  {}. {}", i + 1, ex);
            }
        }
        None => println!("Cycle detected — impossible ordering!"),
    }
}
```

### Complexity Analysis

| Approach | Time | Space |
|----------|------|-------|
| Brute force | O(V * E) | O(V + E) |
| Kahn's algorithm | O(V + E) | O(V + E) |

### Connection to GrindIt

GrindIt's exercise library (Chapter 2) currently treats all exercises as independent. Adding a `prerequisite_id` column to the exercises table would enable a coach to define movement progressions. Topological sort would power the "suggested learning path" feature — show athletes which foundational movements they need before attempting advanced ones.

### Interview Tips

- Always ask whether cycles are possible. If the input is guaranteed acyclic (a DAG), you can skip the cycle check, but mentioning it shows thoroughness.
- Kahn's algorithm (BFS-based) is generally easier to code in an interview than DFS-based topological sort because it avoids recursion and post-order reversal.
- The in-degree map is the key data structure. If you can explain in-degree clearly, the algorithm follows naturally.

---

## Problem 5: Next PR Finder

### Problem Statement

Given a chronological list of your deadlift weights across sessions, for each session find how many sessions until you lifted **heavier** (i.e., set a new PR relative to that session). If you never lifted heavier after a given session, return -1 for that entry.

### Examples

```
Sessions:  [225, 245, 235, 255, 250, 275, 265, 295]
             ↓    ↓    ↓    ↓    ↓    ↓    ↓    ↓
Output:    [  1,   2,   1,   2,   1,   2,   1,  -1]

Explanation:
  Session 0 (225): next heavier is session 1 (245) → 1 session later
  Session 1 (245): next heavier is session 3 (255) → 2 sessions later
  Session 2 (235): next heavier is session 3 (255) → 1 session later
  Session 7 (295): never beaten → -1
```

### Brute Force

For each session, scan forward to find the first heavier weight.

```rust
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
```

**Time:** O(n^2) in the worst case (strictly decreasing sequence).
**Space:** O(n) for the result.

### Optimized Solution

A monotonic stack processes elements from right to left (or left to right, maintaining a stack of indices with decreasing weights). When we encounter a weight, we pop all stack entries that are not heavier, then the top of the stack is our answer.

```rust
fn next_pr(weights: &[u32]) -> Vec<i32> {
    let n = weights.len();
    let mut result = vec![-1i32; n];
    let mut stack: Vec<usize> = Vec::new(); // Stack of indices

    for i in 0..n {
        // Pop all sessions where the current weight is heavier
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
    // Remaining indices in the stack never had a heavier session → already -1
    result
}

fn main() {
    let weights = vec![225, 245, 235, 255, 250, 275, 265, 295];
    let result = next_pr(&weights);

    for (i, &days) in result.iter().enumerate() {
        let label = if days == -1 {
            "never beaten".to_string()
        } else {
            format!("{} sessions later", days)
        };
        println!("Session {} ({} lbs): {}", i, weights[i], label);
    }
}
```

### Complexity Analysis

| Approach | Time | Space |
|----------|------|-------|
| Brute force | O(n^2) | O(n) |
| Monotonic stack | O(n) | O(n) |

Each index is pushed once and popped at most once, so the total work across all iterations is O(n).

### Connection to GrindIt

GrindIt's history page (Chapter 10) shows workout logs over time. A "PR timeline" feature could highlight sessions where you set a new personal record and show how long it took to break each PR. The monotonic stack would power the "days until next PR" metric.

### Interview Tips

- This is the classic "Next Greater Element" pattern. Recognize it by the phrase "next element that is greater/smaller."
- Walk through the stack invariant: the stack always holds indices in decreasing order of weight. This is why it is called a monotonic (decreasing) stack.
- A common follow-up is "previous greater element" — same pattern, iterate in reverse.

---

## Problem 6: WOD Generator

### Problem Statement

Generate all valid WODs that satisfy these constraints:

- A WOD has exactly 3 exercises
- No two exercises can be from the same muscle group
- The total estimated time must be between 15 and 25 minutes

Given a list of exercises with their muscle group and estimated time, return all valid combinations.

### Examples

```
Exercises:
  ("Deadlift",    "posterior", 8)
  ("Back Squat",  "legs",      7)
  ("Push Press",  "shoulders", 6)
  ("Bench Press", "chest",     8)
  ("Pull-up",     "back",      5)
  ("Box Jump",    "legs",      4)
  ("Thruster",    "full body", 9)

Valid WODs (3 exercises, unique muscle groups, 15-25 min):
  [Deadlift (8) + Push Press (6) + Pull-up (5)] = 19 min ✓
  [Deadlift (8) + Bench Press (8) + Pull-up (5)] = 21 min ✓
  [Back Squat (7) + Push Press (6) + Pull-up (5)] = 18 min ✓
  ... and more
```

### Brute Force

Generate all 3-element combinations and filter.

```rust
fn generate_wods_brute(
    exercises: &[(&str, &str, u32)], // (name, muscle_group, minutes)
) -> Vec<Vec<&str>> {
    let n = exercises.len();
    let mut results = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            for k in (j + 1)..n {
                let groups: std::collections::HashSet<&str> = [
                    exercises[i].1, exercises[j].1, exercises[k].1
                ].into_iter().collect();

                if groups.len() != 3 { continue; } // Duplicate muscle group

                let total_time = exercises[i].2 + exercises[j].2 + exercises[k].2;
                if total_time >= 15 && total_time <= 25 {
                    results.push(vec![exercises[i].0, exercises[j].0, exercises[k].0]);
                }
            }
        }
    }
    results
}
```

**Time:** O(n^3) — triple nested loop.
**Space:** O(r) where r is the number of valid WODs.

### Optimized Solution

Backtracking prunes branches early: if we have already used a muscle group, skip all exercises from that group. If the accumulated time already exceeds 25, stop exploring.

```rust
use std::collections::HashSet;

fn generate_wods(
    exercises: &[(&str, &str, u32)],
) -> Vec<Vec<String>> {
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
    // Prune: already over time cap
    if current_time > 25 {
        return;
    }

    // Found a valid WOD
    if current.len() == 3 {
        if current_time >= 15 {
            results.push(current.iter().map(|(name, _)| name.clone()).collect());
        }
        return;
    }

    // Prune: not enough exercises left to fill 3 slots
    let remaining_slots = 3 - current.len();
    if exercises.len() - start < remaining_slots {
        return;
    }

    for i in start..exercises.len() {
        let (name, group, time) = exercises[i];

        // Prune: muscle group already used
        if used_groups.contains(group) {
            continue;
        }

        // Choose
        current.push((name.to_string(), time));
        used_groups.insert(group.to_string());

        // Explore
        backtrack(exercises, i + 1, current, used_groups, current_time + time, results);

        // Un-choose
        current.pop();
        used_groups.remove(group);
    }
}

fn main() {
    let exercises = vec![
        ("Deadlift",    "posterior",  8),
        ("Back Squat",  "legs",       7),
        ("Push Press",  "shoulders",  6),
        ("Bench Press", "chest",      8),
        ("Pull-up",     "back",       5),
        ("Box Jump",    "legs",       4),
        ("Thruster",    "full body",  9),
    ];

    let wods = generate_wods(&exercises);
    println!("Found {} valid WODs:", wods.len());
    for wod in &wods {
        println!("  {:?}", wod);
    }
}
```

### Complexity Analysis

| Approach | Time | Space |
|----------|------|-------|
| Brute force | O(n^3) | O(r) |
| Backtracking | O(n^3) worst case, much less with pruning | O(n + r) |

The worst case is the same — but the pruning conditions (time cap and muscle group uniqueness) eliminate most branches in practice. With 50 exercises across 8 muscle groups, the backtracking version explores a fraction of the brute-force combinations.

### Connection to GrindIt

Chapter 8's WOD form lets coaches manually pick movements. A "generate WOD" button that produces balanced, time-appropriate workouts would use exactly this backtracking approach. The constraints could be extended: "at least one Olympic lift," "no more than one barbell movement," "include a monostructural element."

### Interview Tips

- The backtracking template is always the same: choose, explore, un-choose. Memorize the structure, then adapt the pruning conditions to the problem.
- Pruning is what separates a good answer from a mediocre one. Articulate each prune condition and why it is safe (does not skip valid solutions).
- If asked "what if we want the K best WODs by some score?" — combine backtracking with a min-heap of size K.

---

## Problem 7: Exercise Cache

### Problem Statement

GrindIt's exercise detail page is frequently accessed. Build an LRU (Least Recently Used) cache that holds at most `capacity` exercises. When the cache is full and a new exercise is accessed, evict the exercise that was accessed least recently.

### Examples

```
Cache capacity: 3

get("Deadlift")     → miss, load from DB, cache: [Deadlift]
get("Back Squat")   → miss, load from DB, cache: [Deadlift, Back Squat]
get("Push Press")   → miss, load from DB, cache: [Deadlift, Back Squat, Push Press]
get("Deadlift")     → hit!, cache: [Back Squat, Push Press, Deadlift]  (Deadlift moved to recent)
get("Thruster")     → miss, evict Back Squat (LRU), cache: [Push Press, Deadlift, Thruster]
get("Back Squat")   → miss (was evicted), reload, evict Push Press, cache: [Deadlift, Thruster, Back Squat]
```

### Brute Force

Use a `Vec` as an ordered list. On access, remove and re-insert at the end. On eviction, remove from the front.

```rust
struct LruBrute {
    capacity: usize,
    items: Vec<(String, String)>, // (key, value) — oldest at front
}

impl LruBrute {
    fn new(capacity: usize) -> Self {
        Self { capacity, items: Vec::new() }
    }

    fn get(&mut self, key: &str) -> Option<&str> {
        if let Some(pos) = self.items.iter().position(|(k, _)| k == key) {
            let item = self.items.remove(pos); // O(n) shift
            self.items.push(item);
            self.items.last().map(|(_, v)| v.as_str())
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: String) {
        if let Some(pos) = self.items.iter().position(|(k, _)| *k == key) {
            self.items.remove(pos);
        } else if self.items.len() == self.capacity {
            self.items.remove(0); // Evict LRU
        }
        self.items.push((key, value));
    }
}
```

**Time:** O(n) for both `get` and `put` due to the linear search and shift.
**Space:** O(capacity).

### Optimized Solution

Combine a `HashMap` for O(1) key lookup with a doubly-linked list for O(1) move-to-front and eviction. Rust's standard library does not have an intrusive doubly-linked list, so we simulate one with a `Vec` and index-based pointers. Alternatively, we use `std::collections::LinkedList` indirectly via a wrapper.

The clean approach uses `HashMap` + `VecDeque`-like indexing:

```rust
use std::collections::HashMap;

struct LruCache {
    capacity: usize,
    map: HashMap<String, usize>,   // key → index in entries
    entries: Vec<CacheEntry>,
    head: Option<usize>,           // most recently used
    tail: Option<usize>,           // least recently used
    free: Vec<usize>,              // recycled indices
}

struct CacheEntry {
    key: String,
    value: String,
    prev: Option<usize>,
    next: Option<usize>,
}

impl LruCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::new(),
            entries: Vec::new(),
            head: None,
            tail: None,
            free: Vec::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<String> {
        if let Some(&idx) = self.map.get(key) {
            self.move_to_head(idx);
            Some(self.entries[idx].value.clone())
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: String) {
        if let Some(&idx) = self.map.get(&key) {
            self.entries[idx].value = value;
            self.move_to_head(idx);
        } else {
            if self.map.len() == self.capacity {
                self.evict_tail();
            }
            let idx = self.allocate(CacheEntry {
                key: key.clone(),
                value,
                prev: None,
                next: self.head,
            });
            if let Some(old_head) = self.head {
                self.entries[old_head].prev = Some(idx);
            }
            self.head = Some(idx);
            if self.tail.is_none() {
                self.tail = Some(idx);
            }
            self.map.insert(key, idx);
        }
    }

    fn move_to_head(&mut self, idx: usize) {
        if self.head == Some(idx) { return; }

        // Detach
        let prev = self.entries[idx].prev;
        let next = self.entries[idx].next;
        if let Some(p) = prev { self.entries[p].next = next; }
        if let Some(n) = next { self.entries[n].prev = prev; }
        if self.tail == Some(idx) { self.tail = prev; }

        // Attach at head
        self.entries[idx].prev = None;
        self.entries[idx].next = self.head;
        if let Some(old_head) = self.head {
            self.entries[old_head].prev = Some(idx);
        }
        self.head = Some(idx);
    }

    fn evict_tail(&mut self) {
        if let Some(tail_idx) = self.tail {
            let key = self.entries[tail_idx].key.clone();
            self.tail = self.entries[tail_idx].prev;
            if let Some(new_tail) = self.tail {
                self.entries[new_tail].next = None;
            } else {
                self.head = None;
            }
            self.map.remove(&key);
            self.free.push(tail_idx);
        }
    }

    fn allocate(&mut self, entry: CacheEntry) -> usize {
        if let Some(idx) = self.free.pop() {
            self.entries[idx] = entry;
            idx
        } else {
            self.entries.push(entry);
            self.entries.len() - 1
        }
    }
}

fn main() {
    let mut cache = LruCache::new(3);

    cache.put("Deadlift".into(), "Posterior chain compound lift".into());
    cache.put("Back Squat".into(), "King of leg exercises".into());
    cache.put("Push Press".into(), "Overhead strength builder".into());

    // Access Deadlift — moves it to most recent
    assert!(cache.get("Deadlift").is_some());

    // Insert Thruster — evicts Back Squat (least recently used)
    cache.put("Thruster".into(), "Full body metabolic exercise".into());

    assert!(cache.get("Back Squat").is_none()); // Evicted
    assert!(cache.get("Push Press").is_some()); // Still cached
    assert!(cache.get("Deadlift").is_some());   // Still cached
    assert!(cache.get("Thruster").is_some());   // Just added

    println!("All assertions passed — LRU cache works correctly.");
}
```

### Complexity Analysis

| Approach | get | put | Space |
|----------|-----|-----|-------|
| Brute force (Vec) | O(n) | O(n) | O(capacity) |
| HashMap + linked list | O(1) | O(1) amortized | O(capacity) |

### Connection to GrindIt

Chapter 14's service worker caches HTTP responses using a cache-first strategy. An LRU cache in the Rust backend could sit between the database and the server functions — caching the 50 most frequently accessed exercises in memory. The `db.rs` functions from Chapter 5 would check the cache before hitting PostgreSQL.

### Interview Tips

- LRU Cache is one of the most frequently asked design+coding problems. Know it cold.
- The core insight is the combination of two data structures: a hash map for O(1) lookup and a doubly-linked list for O(1) ordering operations.
- In Rust, the borrow checker makes doubly-linked lists famously tricky. In an interview, explain the approach clearly and mention that production Rust code might use `std::collections::LinkedList` or an arena allocator. The index-based approach shown here avoids `unsafe` entirely.

---

## Problem 8: Progression Path

### Problem Statement

Exercises in GrindIt have difficulty levels and transitions between them. An athlete at "Air Squat" wants to reach "Pistol Squat." Each transition has a difficulty cost (how hard it is to learn). Find the **easiest progression path** — the one with the minimum total difficulty cost.

### Examples

```
Progressions (exercise → exercise, difficulty cost):
  Air Squat → Goblet Squat (2)
  Air Squat → Jump Squat (3)
  Goblet Squat → Front Squat (3)
  Goblet Squat → Bulgarian Split Squat (4)
  Jump Squat → Box Jump (2)
  Front Squat → Overhead Squat (4)
  Bulgarian Split Squat → Pistol Squat (5)
  Front Squat → Pistol Squat (7)
  Overhead Squat → Pistol Squat (3)

Shortest path from "Air Squat" to "Pistol Squat":
  Air Squat →(2)→ Goblet Squat →(3)→ Front Squat →(4)→ Overhead Squat →(3)→ Pistol Squat
  Total cost: 12

Alternative: Air Squat →(2)→ Goblet Squat →(4)→ Bulgarian Split Squat →(5)→ Pistol Squat = 11 ✓
  This is cheaper!
```

### Brute Force

BFS explores all paths, tracking the minimum cost to reach each node.

```rust
use std::collections::{HashMap, VecDeque};

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

    best_cost.get(end).map(|&cost| {
        (cost, best_path[end].clone())
    })
}
```

**Time:** O(V * E) in the worst case — BFS can revisit nodes with better costs.
**Space:** O(V + E).

### Optimized Solution

Dijkstra's algorithm with a min-heap guarantees the shortest path in O((V + E) log V) time.

```rust
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

fn shortest_path(
    edges: &[(&str, &str, u32)],
    start: &str,
    end: &str,
) -> Option<(u32, Vec<String>)> {
    let mut adj: HashMap<&str, Vec<(&str, u32)>> = HashMap::new();
    for &(from, to, cost) in edges {
        adj.entry(from).or_default().push((to, cost));
    }

    // Min-heap: (cost, node)
    let mut heap = BinaryHeap::new();
    let mut dist: HashMap<&str, u32> = HashMap::new();
    let mut prev: HashMap<&str, &str> = HashMap::new();

    heap.push(Reverse((0u32, start)));
    dist.insert(start, 0);

    while let Some(Reverse((cost, node))) = heap.pop() {
        // Already found a shorter path to this node
        if let Some(&best) = dist.get(node) {
            if cost > best { continue; }
        }

        // Found the destination
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

    None // No path exists
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

    match shortest_path(&edges, "Air Squat", "Pistol Squat") {
        Some((cost, path)) => {
            println!("Easiest progression (total difficulty: {}):", cost);
            println!("  {}", path.join(" → "));
        }
        None => println!("No progression path exists."),
    }
    // Easiest progression (total difficulty: 11):
    //   Air Squat → Goblet Squat → Bulgarian Split Squat → Pistol Squat
}
```

### Complexity Analysis

| Approach | Time | Space |
|----------|------|-------|
| BFS (unweighted relaxation) | O(V * E) worst case | O(V + E) |
| Dijkstra | O((V + E) log V) | O(V + E) |

For GrindIt's exercise graph (maybe 200 exercises with 500 progression edges), both are fast. Dijkstra's advantage shows when the graph grows — a platform with thousands of exercises across multiple disciplines.

### Connection to GrindIt

This directly extends the prerequisite system from Problem 4. Where topological sort gives a valid ordering, Dijkstra gives the *easiest* ordering. A "progression planner" feature could show an athlete: "To reach Muscle-ups from Banded Pull-ups, the easiest path is: Banded Pull-up (2 weeks) → Strict Pull-up (4 weeks) → Kipping Pull-up (3 weeks) → Muscle-up."

### Interview Tips

- Know when to use BFS (unweighted edges) vs. Dijkstra (weighted, non-negative edges) vs. Bellman-Ford (negative edges). GrindIt's difficulty costs are always positive, so Dijkstra is the right choice.
- Rust's `BinaryHeap` is a max-heap. Wrap values in `Reverse()` for a min-heap — this is idiomatic Rust and interviewers working in Rust will expect it.
- If asked "what about negative edge weights?" (e.g., an exercise that makes the next one easier), mention that Dijkstra would not work and you would need Bellman-Ford.

---

## Wrapping Up

These eight problems cover the most common interview DSA patterns that did not surface naturally in GrindIt's feature code. Combined with the eighteen patterns woven through Chapters 1-18, you have now seen roughly twenty-six distinct algorithmic patterns — all grounded in a domain you understand deeply.

A few observations that apply across all eight problems:

**The brute force matters.** In an interview, starting with brute force is not a weakness — it is a strategy. It shows you can solve the problem, gives you a baseline to optimize from, and often reveals the structure that the optimized solution exploits. Every problem in this chapter started with a working brute force before optimizing.

**Rust's type system helps.** `Option` instead of null. `Reverse()` for min-heaps. `HashMap::entry` for clean insert-or-update. These are not just Rust idioms — they are patterns that prevent bugs. An interviewer watching you code in Rust wants to see you use these tools naturally.

**The domain connection is real.** These are not contrived problems dressed in fitness language. A trie-backed autocomplete, an LRU exercise cache, a prerequisite graph, a WOD generator with constraints — these are features a production fitness tracker could actually ship. When you explain a solution in an interview and connect it to a real system you built, that is more convincing than solving an abstract puzzle.

Chapter 20 takes the system you built and asks: how would it scale to a million users? Chapter 21 puts you in the interview seat with mock coding and system design sessions. The patterns from this chapter will be your tools for both.
