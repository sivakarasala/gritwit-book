# The VIP Line: Queues and Priority Queues for Competition Day

## The Problem

It's competition day at the box. Thirty athletes are crushed into the lobby, phones in hand, hammering the "Submit Score" button the instant they finish their WOD. Your server processes submissions one at a time. In what order?

First come, first served seems fair. Athlete A finishes at 10:01:03, Athlete B at 10:01:05 — A goes first. Simple.

Then a judge walks over. "I need to correct Athlete C's score — I counted a no-rep." This correction is urgent. If it sits behind 20 pending submissions, the leaderboard shows wrong data for minutes. Athletes are refreshing, screenshots are being posted, arguments are starting.

And then the gym owner sends an admin override: "Athlete D was in the wrong heat, remove their score from this bracket."

Three priority levels. One line. A regular queue won't cut it. You need a **VIP line** — where judge corrections jump ahead of normal submissions, and admin overrides jump ahead of everything.

## The Naive Way

Your first attempt: a `Vec` where you push to the back and remove from the front.

```rust
struct NaiveQueue<T> {
    items: Vec<T>,
}

impl<T> NaiveQueue<T> {
    fn new() -> Self {
        NaiveQueue { items: Vec::new() }
    }

    fn enqueue(&mut self, item: T) {
        self.items.push(item); // O(1) amortized — fine
    }

    fn dequeue(&mut self) -> Option<T> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0)) // O(n) — every element shifts left!
        }
    }
}
```

That `remove(0)` is a killer. Every dequeue shifts the entire array left by one position. With 30 athletes submitting scores and your server dequeuing them one by one, you're doing 30 + 29 + 28 + ... = 435 element shifts. And that's a small competition.

For priority? You'd have to scan the entire queue to find the highest-priority item. O(n) every single time.

## The Insight

Picture a circular track at the gym. Runners don't start at the beginning each lap — they just keep going around. When they pass the start line, they're back where they began without anyone having to move.

That's a **ring buffer** (circular array). You keep two pointers — `head` (where you dequeue from) and `tail` (where you enqueue to). When either pointer reaches the end of the array, it wraps around to the beginning. No shifting. No wasted space. Just two pointers chasing each other in circles.

```
  Capacity = 6
  [_, _, A, B, C, _]
         ^        ^
        head     tail

  After enqueue(D):
  [_, _, A, B, C, D]
         ^           ^
        head        tail (wraps to 0 on next enqueue)

  After dequeue() -> A:
  [_, _, _, B, C, D]
            ^
           head
```

## The Build

Let's build a proper queue with a ring buffer:

```rust
pub struct Queue<T> {
    buffer: Vec<Option<T>>,
    head: usize,    // index of the front element
    tail: usize,    // index of the next free slot
    len: usize,
    capacity: usize,
}

impl<T> Queue<T> {
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(None);
        }
        Queue {
            buffer,
            head: 0,
            tail: 0,
            len: 0,
            capacity,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn enqueue(&mut self, item: T) {
        if self.is_full() {
            self.resize();
        }
        self.buffer[self.tail] = Some(item);
        self.tail = (self.tail + 1) % self.capacity;
        self.len += 1;
    }

    pub fn dequeue(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let item = self.buffer[self.head].take();
        self.head = (self.head + 1) % self.capacity;
        self.len -= 1;
        item
    }

    pub fn peek(&self) -> Option<&T> {
        if self.is_empty() {
            None
        } else {
            self.buffer[self.head].as_ref()
        }
    }

    fn resize(&mut self) {
        let new_capacity = self.capacity * 2;
        let mut new_buffer = Vec::with_capacity(new_capacity);

        // Copy elements in order, starting from head
        let mut i = self.head;
        for _ in 0..self.len {
            new_buffer.push(self.buffer[i].take());
            i = (i + 1) % self.capacity;
        }
        // Fill the rest with None
        for _ in self.len..new_capacity {
            new_buffer.push(None);
        }

        self.buffer = new_buffer;
        self.head = 0;
        self.tail = self.len;
        self.capacity = new_capacity;
    }
}
```

Here's the ring buffer wrapping around visually:

```
  Initial (cap=4):    [S1, S2, S3, S4]    head=0, tail=0 (full)
                        ^
                      head,tail

  After 2 dequeues:   [__, __, S3, S4]    head=2, tail=0
                                ^
                               head

  After enqueue(S5):  [S5, __, S3, S4]    head=2, tail=1
                        ^       ^
                       tail   head          <- tail WRAPPED AROUND!

  After enqueue(S6):  [S5, S6, S3, S4]    head=2, tail=2 (full again)
```

Now let's level up. Competition day needs priorities. Here's a **priority queue** built on a binary min-heap:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Normal = 2,       // Regular score submission
    JudgeCorrection = 1, // Judge fixing a score
    AdminOverride = 0,   // Gym owner command (highest priority = lowest number)
}

#[derive(Debug)]
pub struct Submission {
    pub athlete: String,
    pub score: String,
    pub priority: Priority,
}

pub struct PriorityQueue {
    heap: Vec<Submission>,
}

impl PriorityQueue {
    pub fn new() -> Self {
        PriorityQueue { heap: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Insert a submission — O(log n)
    pub fn enqueue(&mut self, submission: Submission) {
        self.heap.push(submission);
        self.sift_up(self.heap.len() - 1);
    }

    /// Remove and return the highest-priority submission — O(log n)
    pub fn dequeue(&mut self) -> Option<Submission> {
        if self.heap.is_empty() {
            return None;
        }
        let last = self.heap.len() - 1;
        self.heap.swap(0, last);
        let item = self.heap.pop();
        if !self.heap.is_empty() {
            self.sift_down(0);
        }
        item
    }

    pub fn peek(&self) -> Option<&Submission> {
        self.heap.first()
    }

    fn sift_up(&mut self, mut idx: usize) {
        while idx > 0 {
            let parent = (idx - 1) / 2;
            if self.heap[idx].priority < self.heap[parent].priority {
                self.heap.swap(idx, parent);
                idx = parent;
            } else {
                break;
            }
        }
    }

    fn sift_down(&mut self, mut idx: usize) {
        let len = self.heap.len();
        loop {
            let left = 2 * idx + 1;
            let right = 2 * idx + 2;
            let mut smallest = idx;

            if left < len && self.heap[left].priority < self.heap[smallest].priority {
                smallest = left;
            }
            if right < len && self.heap[right].priority < self.heap[smallest].priority {
                smallest = right;
            }

            if smallest != idx {
                self.heap.swap(idx, smallest);
                idx = smallest;
            } else {
                break;
            }
        }
    }
}
```

## The Payoff

Competition day, live:

```rust
fn main() {
    let mut pq = PriorityQueue::new();

    // 10:01:03 — Athletes start submitting
    pq.enqueue(Submission {
        athlete: "Alice".into(),
        score: "Fran 3:45".into(),
        priority: Priority::Normal,
    });
    pq.enqueue(Submission {
        athlete: "Bob".into(),
        score: "Fran 4:12".into(),
        priority: Priority::Normal,
    });
    pq.enqueue(Submission {
        athlete: "Carol".into(),
        score: "Fran 3:58".into(),
        priority: Priority::Normal,
    });

    // 10:01:10 — Judge catches a no-rep
    pq.enqueue(Submission {
        athlete: "Dave".into(),
        score: "Fran 4:30 (corrected: was 4:15, 2 no-reps)".into(),
        priority: Priority::JudgeCorrection,
    });

    // 10:01:11 — Admin: wrong heat assignment
    pq.enqueue(Submission {
        athlete: "Eve".into(),
        score: "REMOVE from Heat 2, move to Heat 3".into(),
        priority: Priority::AdminOverride,
    });

    // 10:01:12 — More normal submissions
    pq.enqueue(Submission {
        athlete: "Frank".into(),
        score: "Fran 5:01".into(),
        priority: Priority::Normal,
    });

    // Process in priority order
    println!("Processing competition scores:\n");
    let mut order = 1;
    while let Some(sub) = pq.dequeue() {
        println!("  {}. [{:?}] {} — {}",
            order, sub.priority, sub.athlete, sub.score);
        order += 1;
    }
    // Output:
    //   1. [AdminOverride] Eve — REMOVE from Heat 2, move to Heat 3
    //   2. [JudgeCorrection] Dave — Fran 4:30 (corrected)
    //   3. [Normal] Alice — Fran 3:45
    //   4. [Normal] Bob — Fran 4:12
    //   5. [Normal] Carol — Fran 3:58
    //   6. [Normal] Frank — Fran 5:01

    // The admin override processed first. The judge correction second.
    // The leaderboard was accurate within milliseconds.
    println!("\nLeaderboard updated correctly. No arguments. No screenshots of wrong data.");
}
```

Eve's heat reassignment processed instantly. Dave's no-rep correction went through before any normal submissions. The leaderboard stayed accurate. No drama.

## Complexity Comparison

| Operation | Vec (remove from front) | Ring Buffer Queue | Priority Queue (Heap) |
|-----------|------------------------|-------------------|-----------------------|
| Enqueue | O(1) amortized | **O(1)** amortized | O(log n) |
| Dequeue | **O(n)** (shift all) | **O(1)** | O(log n) |
| Peek | O(1) | O(1) | O(1) |
| Find highest priority | O(n) scan | O(n) scan | **O(1)** — it's always at the top |
| Space | O(n) | O(n) | O(n) |

The ring buffer turns dequeue from O(n) to O(1). The heap turns "find the most urgent item" from O(n) to O(1), with O(log n) to maintain the structure. On competition day with hundreds of submissions, that difference is the gap between a smooth event and chaos.

## Try It Yourself

1. **Tiebreaking**: When two submissions have the same priority, the one submitted first should be processed first (FIFO within priority). Add a `sequence: u64` field to `Submission` and modify the comparison to break ties by sequence number. This is how real priority queues maintain fairness.

2. **Bounded priority queue**: Modify `PriorityQueue` to have a maximum capacity. When full and a new high-priority item arrives, evict the lowest-priority item to make room. This prevents memory exhaustion during a huge competition.

3. **Double-ended queue (deque)**: Extend the ring buffer to support `push_front` and `pop_back` in addition to the existing operations. A deque lets you undo the last enqueue (athlete submitted to the wrong workout) — useful for the "oops" button in the UI.
