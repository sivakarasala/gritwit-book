# The Magic Podium (A Heap for Your Leaderboard)

## The Problem: Sorting an Entire Stadium

Your GrindIt leaderboard works. When someone posts a new score, you sort all scores and display the top 10:

```rust
fn top_10(scores: &mut Vec<(String, f64)>) -> Vec<(String, f64)> {
    // Sort by score ascending (lower time = better for ForTime)
    scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    scores.iter().take(10).cloned().collect()
}
```

For your garage gym with 12 athletes, this is instant. But picture the CrossFit Games Open: 300,000 athletes, scores streaming in all week. Every new submission triggers a full sort — O(n log n) where n = 300,000. That's roughly 5.5 million comparisons *per new score*. To show ten names on a screen.

It's like making 300,000 people line up tallest-to-shortest just to hand a trophy to the top 10. Most of them have zero chance of placing. Why are they even in line?

## The Insight: A Podium That Maintains Itself

What if you had a podium that holds exactly 10 spots? When a new score arrives, you compare it to the *worst* score on the podium. Better? Kick the worst one off and let the new score find its place. Worse? Toss it — don't even look at it.

That podium is a **heap** — specifically a min-heap used as a "top K" tracker. The min-heap always gives you instant access to its smallest element (the worst score on the podium). Comparing a new score against it is O(1). Inserting into the heap is O(log k), where k is 10. Not log 300,000. Log 10.

## The Build: A Min-Heap from Scratch

A binary heap is stored as a flat array. The trick is the parent-child relationship lives in the *indices*:

```
        Array: [2, 5, 3, 8, 7, 6, 4]

        As a tree:
                2           <- index 0
              /   \
            5       3       <- indices 1, 2
           / \     / \
          8   7   6   4     <- indices 3, 4, 5, 6

        Parent of i:        (i - 1) / 2
        Left child of i:    2 * i + 1
        Right child of i:   2 * i + 2
```

The **heap property**: every parent is less than or equal to its children (for a min-heap). The root is always the minimum.

```rust
#[derive(Debug)]
struct MinHeap<T: PartialOrd> {
    data: Vec<T>,
}

impl<T: PartialOrd + std::fmt::Debug> MinHeap<T> {
    fn new() -> Self {
        MinHeap { data: Vec::new() }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Look at the minimum element without removing it. O(1).
    fn peek(&self) -> Option<&T> {
        self.data.first()
    }

    /// Add an element and restore the heap property. O(log n).
    fn push(&mut self, value: T) {
        self.data.push(value);
        self.bubble_up(self.data.len() - 1);
    }

    /// Remove and return the minimum element. O(log n).
    fn pop(&mut self) -> Option<T> {
        if self.data.is_empty() {
            return None;
        }
        let last = self.data.len() - 1;
        self.data.swap(0, last);
        let min = self.data.pop();
        if !self.data.is_empty() {
            self.bubble_down(0);
        }
        min
    }

    /// After inserting at the end, move the element UP until the
    /// heap property is restored. Like a light weight floating to the top.
    fn bubble_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if self.data[idx] < self.data[parent] {
                self.data.swap(idx, parent);
                idx = parent;
            } else {
                break;
            }
        }
    }

    /// After swapping root with last element, move the new root DOWN
    /// until the heap property is restored. Like a heavy weight sinking.
    fn bubble_down(&mut self, mut idx: usize) {
        let len = self.data.len();
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut smallest = idx;

            if left < len && self.data[left] < self.data[smallest] {
                smallest = left;
            }
            if right < len && self.data[right] < self.data[smallest] {
                smallest = right;
            }

            if smallest != idx {
                self.data.swap(idx, smallest);
                idx = smallest;
            } else {
                break;
            }
        }
    }
}
```

That's a fully functional min-heap. Now let's build the podium.

## The TopK Tracker

Here's the clever part: we use a *min*-heap of size K to track the *top* K scores. The smallest element in the heap is the worst score that made the cut — the gatekeeper. Any new score must beat the gatekeeper to get on the podium.

For "ForTime" scoring where lower is better, "top" means lowest times, so we want a *max*-heap of size K (or equivalently, we negate the values). Let's make this generic with a wrapper:

```rust
#[derive(Debug)]
struct TopKTracker {
    heap: MinHeap<f64>,
    k: usize,
    lower_is_better: bool,
}

impl TopKTracker {
    fn new(k: usize, lower_is_better: bool) -> Self {
        TopKTracker {
            heap: MinHeap::new(),
            k,
            lower_is_better,
        }
    }

    fn submit(&mut self, score: f64) {
        // Internally, we normalize so that "better" always means "higher"
        // (for the min-heap, the gatekeeper is the min = worst of the top K)
        let normalized = if self.lower_is_better { -score } else { score };

        if self.heap.len() < self.k {
            self.heap.push(normalized);
        } else if let Some(&gatekeeper) = self.heap.peek() {
            if normalized > gatekeeper {
                self.heap.pop();
                self.heap.push(normalized);
            }
        }
    }

    fn get_top_k(&self) -> Vec<f64> {
        let mut results: Vec<f64> = self.heap.data.iter().copied().collect();
        results.sort_by(|a, b| b.partial_cmp(a).unwrap()); // best first
        if self.lower_is_better {
            results.iter().map(|v| -v).collect()
        } else {
            results
        }
    }
}
```

## The Payoff: CrossFit Games Scale

```rust
fn main() {
    // Simulate 300,000 athletes posting ForTime scores (in seconds)
    let mut tracker = TopKTracker::new(10, true); // lower time = better

    // In production these come from a database/stream. We'll simulate.
    let fake_scores: Vec<f64> = (0..300_000)
        .map(|i| 180.0 + (i as f64 * 7.3 % 600.0)) // scores between 180s and 780s
        .collect();

    for score in &fake_scores {
        tracker.submit(*score);
    }

    println!("=== Top 10 (ForTime — lower is better) ===");
    for (i, score) in tracker.get_top_k().iter().enumerate() {
        let mins = (*score as u64) / 60;
        let secs = (*score as u64) % 60;
        println!("  #{:>2}  {}:{:02}", i + 1, mins, secs);
    }

    // Now AMRAP — higher is better
    let mut amrap_tracker = TopKTracker::new(5, false);
    let amrap_scores = vec![
        15.23, 12.05, 18.30, 14.12, 16.45,
        11.00, 19.15, 13.08, 17.22, 20.01,
    ];
    for s in &amrap_scores {
        amrap_tracker.submit(*s);
    }
    println!("\n=== Top 5 AMRAP ===");
    for (i, score) in amrap_tracker.get_top_k().iter().enumerate() {
        let rounds = *score as u32;
        let reps = ((*score - rounds as f64) * 100.0) as u32;
        println!("  #{} {} rounds + {} reps", i + 1, rounds, reps);
    }
}
```

300,000 scores processed to produce a top 10, and each insertion did at most log(10) = ~3.3 comparisons. The total work: ~300,000 x 3.3 = ~1 million comparisons. The full-sort approach would have done ~5.5 million. And the gap widens as N grows — log(K) stays constant while log(N) keeps climbing.

## Complexity Comparison

| Operation | Full Sort | Heap Top-K |
|---|---|---|
| Build initial top K | O(n log n) | O(n log k) |
| Insert new score | O(n log n) re-sort | O(log k) single insert |
| Get top K | O(k) slice | O(k log k) extract + sort |
| Space | O(n) all scores | O(k) just the podium |
| For n=300K, k=10 | ~5.5M comparisons | ~1M comparisons |

The heap doesn't just win on theory — it wins on *memory* too. You only ever hold K scores in the tracker, not all N. When scores stream from a WebSocket, you never even need them all in memory.

## Try It Yourself

1. **Add athlete names.** Change the heap to store `(f64, String)` tuples and implement `PartialOrd` for them (compare on the score only). Display the leaderboard with names.

2. **Build a `MaxHeap`** by wrapping `MinHeap` with a newtype that reverses the ordering. Use `std::cmp::Reverse` or your own wrapper struct.

3. **Streaming leaderboard.** Write a function that takes an iterator of scores (simulating a live feed) and prints the updated top 5 after every 1,000 scores. Show how the podium evolves as more athletes submit.
