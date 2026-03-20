# Channels — "The Gym PA System"

It's the CrossFit Open. 200 athletes are logging scores simultaneously. Your leaderboard page refreshes every 5 seconds by hitting the database — 200 users times 12 refreshes per minute equals 2,400 database queries per minute just for the leaderboard. Your Postgres starts sweating. What if, instead of everyone *asking* for updates, the server *told* them when something changed? One athlete posts a score, the server broadcasts it to every connected client. No polling, no wasted queries. That is channels — the message-passing primitive that lets different parts of your system talk to each other without sharing memory.

---

## Why Channels? — "Don't Share Memory, Share Messages"

Concurrent programming gives you a fundamental choice: shared state or message passing. With shared state (a `Mutex`, for example), multiple threads access the same data and take turns locking it. With message passing, threads own their own data and communicate by sending messages through a channel. Rust's standard library has `mpsc` — multi-producer, single-consumer — built in.

Here is the GrindIt scenario that makes the choice concrete. An upload handler finishes processing an athlete's score. That event needs to trigger three things: the leaderboard cache refreshes, the activity feed updates, and the coach gets a notification. Three separate concerns. You could wire them together with direct function calls — the handler calls `update_leaderboard()`, then `update_feed()`, then `notify_coach()`. But now the handler knows about all three subsystems. Add a fourth concern (streak calculator) and you have to modify the handler again. Tight coupling.

Think of it like a gym. The athlete finishes a WOD. The tightly-coupled approach: they walk to the whiteboard and update their score, then walk to the coach's office and knock on the door, then walk to the TV screen and type in their result. Three trips, and the athlete has to know where everything is.

The channel approach: the athlete grabs the PA microphone and announces "Alice, 7:42 Fran Rx." Everyone who cares — the whiteboard operator, the coach, the TV screen — hears it. The athlete does not know or care who is listening. That is a broadcast channel. Or think of the **suggestion box** — athletes drop in suggestions (senders), one staff member reads them (single receiver). That is `mpsc`. The key insight: the people dropping off suggestions do not need to wait for the reader, and the reader processes them at their own pace. They are decoupled.

---

## Build a Simple Channel from Scratch

Before using the standard library's channels, let us build one. An `mpsc` channel needs three things: a buffer to hold messages, a way to add messages, and a way to retrieve them (blocking if none are available). We will use a `VecDeque` for the buffer, a `Mutex` to protect it, and a `Condvar` to wake the receiver when a message arrives.

```rust
use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;

struct MyChannel<T> {
    queue: Mutex<VecDeque<T>>,
    condvar: Condvar,
}

#[derive(Clone)]
struct Sender<T> {
    inner: Arc<MyChannel<T>>,
}

struct Receiver<T> {
    inner: Arc<MyChannel<T>>,
}

impl<T> Sender<T> {
    fn send(&self, value: T) {
        let mut queue = self.inner.queue.lock().unwrap();
        queue.push_back(value);
        self.inner.condvar.notify_one();
    }
}

impl<T> Receiver<T> {
    fn recv(&self) -> T {
        let mut queue = self.inner.queue.lock().unwrap();
        loop {
            if let Some(value) = queue.pop_front() {
                return value;
            }
            queue = self.inner.condvar.wait(queue).unwrap();
        }
    }
}

fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(MyChannel {
        queue: Mutex::new(VecDeque::new()),
        condvar: Condvar::new(),
    });
    (
        Sender { inner: Arc::clone(&inner) },
        Receiver { inner },
    )
}

fn main() {
    let (tx, rx) = channel();

    // Simulate an athlete posting a score
    std::thread::spawn(move || {
        tx.send("Alice: 7:42 Fran Rx");
        tx.send("Bob: 8:15 Fran Scaled");
    });

    // Leaderboard updater receives scores
    println!("Score received: {}", rx.recv());
    println!("Score received: {}", rx.recv());
}
```

The `Condvar` is the crucial piece. Without it, the receiver would have to spin in a loop checking the queue — burning CPU for nothing. `condvar.wait(queue)` releases the mutex lock and puts the thread to sleep until `notify_one()` wakes it. The thread then reacquires the lock and checks the queue again. This is the same wait/notify pattern used in every producer-consumer system, from Java's `BlockingQueue` to Go's buffered channels.

The PA system analogy holds: `send` is the announcer speaking into the microphone, `recv` is a speaker that plays the next announcement (or stays silent until one arrives), and `notify_one` is the signal that turns the speaker on.

---

## mpsc in Action — The Score Pipeline

Now let us use the real thing. Rust's `std::sync::mpsc::channel` gives us an unbounded multi-producer, single-consumer channel. Here is a GrindIt score processing pipeline: multiple athlete threads submit scores, and a single processor thread receives and ranks them.

```rust
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
struct Score {
    athlete: String,
    workout: String,
    time_seconds: u32,
    is_rx: bool,
}

fn main() {
    let (tx, rx) = mpsc::channel();

    // Spawn the score processor (single consumer)
    let processor = thread::spawn(move || {
        let mut leaderboard: Vec<Score> = Vec::new();

        // recv() blocks until a message arrives or all senders are dropped
        while let Ok(score) = rx.recv() {
            println!("Received: {} - {}s ({})",
                score.athlete, score.time_seconds,
                if score.is_rx { "Rx" } else { "Scaled" });
            leaderboard.push(score);
        }

        // All senders dropped — channel closed. Print final rankings.
        leaderboard.sort_by(|a, b| {
            b.is_rx.cmp(&a.is_rx)
                .then_with(|| a.time_seconds.cmp(&b.time_seconds))
        });

        println!("\n--- Final Leaderboard ---");
        for (i, s) in leaderboard.iter().enumerate() {
            println!("{}. {} - {}s {}",
                i + 1, s.athlete, s.time_seconds,
                if s.is_rx { "Rx" } else { "Scaled" });
        }
    });

    // Spawn 4 athlete threads — each gets a cloned sender
    let athletes = vec![
        ("Alice", 452, true),
        ("Bob", 515, false),
        ("Carol", 498, true),
        ("Dave", 470, true),
    ];

    let handles: Vec<_> = athletes.into_iter().map(|(name, time, rx)| {
        let tx = tx.clone(); // each thread gets its own Sender clone
        thread::spawn(move || {
            // Simulate the athlete finishing at different times
            thread::sleep(Duration::from_millis(time as u64 / 5));
            tx.send(Score {
                athlete: name.to_string(),
                workout: "Fran".to_string(),
                time_seconds: time,
                is_rx: rx,
            }).unwrap();
        })
    }).collect();

    // Drop the original sender — only the clones in threads remain
    drop(tx);

    // Wait for all athletes to finish
    for h in handles {
        h.join().unwrap();
    }

    // Wait for processor to finish
    processor.join().unwrap();
}
```

Notice the critical `drop(tx)` after spawning the threads. Each thread holds a cloned sender. The original `tx` in main still exists. If we do not drop it, the receiver's `while let Ok(score) = rx.recv()` loop will never end — it will block forever waiting for a message that never comes, because the channel is not "closed" until *all* senders are dropped. Dropping the original `tx` means only the thread-held clones remain, and once those threads finish and their senders go out of scope, the channel closes and `rx.recv()` returns `Err`.

This is the suggestion box analogy in full: four athletes each drop a score slip into the box (cloned senders), one staff member reads them (single receiver), and once all the athletes are done and walk away (senders dropped), the staff member knows there are no more slips coming.

---

## Bounded vs Unbounded — Back Pressure

The channel we just used, `mpsc::channel()`, is **unbounded**. Senders never block — they push messages into an infinitely growing buffer. If the receiver is slow and 200 athletes post scores in 10 seconds, all 200 messages sit in memory waiting to be processed. For our gym app this is fine. For a system handling millions of events, unbounded growth is a memory leak waiting to happen.

`mpsc::sync_channel(capacity)` creates a **bounded** channel. When the buffer is full, the sender blocks until the receiver consumes a message and frees a slot. This is **back pressure** — the system slows down producers when the consumer cannot keep up.

```rust
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn main() {
    // Bounded channel: only 3 messages can buffer at a time
    let (tx, rx) = mpsc::sync_channel(3);

    // Slow consumer — takes 200ms per message
    let consumer = thread::spawn(move || {
        while let Ok(score) = rx.recv() {
            println!("Processing score: {}", score);
            thread::sleep(Duration::from_millis(200));
        }
        println!("Consumer done.");
    });

    // Fast producer — tries to send 8 messages as fast as possible
    for i in 1..=8 {
        println!("Sending score {}...", i);
        tx.send(format!("Athlete {} - 5:00", i)).unwrap();
        println!("Score {} sent!", i);
    }
    drop(tx);
    consumer.join().unwrap();
}
```

Run this and watch the output. The first 3 sends complete instantly (filling the buffer). Send 4 blocks until the consumer processes one message. The producer and consumer fall into lockstep — the producer can never get more than 3 messages ahead.

The gym analogy: the unbounded channel is a suggestion box with infinite paper — you can always stuff another slip in. The bounded channel is a suggestion box that holds only 3 slips. When it is full, you stand there holding your paper until the staff member pulls one out and makes room.

| | Unbounded (`channel()`) | Bounded (`sync_channel(N)`) |
|---|---|---|
| Sender blocks? | Never | When buffer is full |
| Memory | Grows without limit | Capped at N messages |
| Use when | Consumer keeps up, or bursts are short | Consumer is slower than producer |

---

## Channel Patterns for GrindIt

### Fan-out: One event, multiple handlers

When an athlete posts a score, you want multiple things to happen:

```text
Score submitted
    |----> Leaderboard updater
    |----> Activity feed
    |----> Coach notification
    +----> Streak calculator
```

Standard `mpsc` does not support this — it has one receiver. For fan-out, you need a **broadcast** channel where every receiver gets every message. In async Rust, `tokio::sync::broadcast` provides this:

```rust,ignore
use tokio::sync::broadcast;

let (tx, _) = broadcast::channel::<String>(100);

let mut leaderboard_rx = tx.subscribe();
let mut feed_rx = tx.subscribe();
let mut coach_rx = tx.subscribe();

// One send reaches all subscribers
tx.send("Alice: 7:42 Fran Rx".to_string()).unwrap();
```

This is the PA system at its purest — one microphone, many speakers.

### Worker pool: Multiple processors

Sometimes you want the opposite: multiple consumers sharing a workload. Four threads processing video uploads, each pulling the next job from a shared queue. Standard `mpsc` only allows one `Receiver`, but you can share it with `Arc<Mutex<Receiver>>`:

```rust
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<String>();
    let shared_rx = Arc::new(Mutex::new(rx));

    // 4 worker threads processing video uploads
    let workers: Vec<_> = (0..4).map(|id| {
        let rx = Arc::clone(&shared_rx);
        thread::spawn(move || {
            while let Ok(job) = rx.lock().unwrap().recv() {
                println!("Worker {} processing: {}", id, job);
            }
        })
    }).collect();

    // Send 8 jobs
    for i in 0..8 {
        tx.send(format!("video_upload_{}.mp4", i)).unwrap();
    }
    drop(tx);

    for w in workers {
        w.join().unwrap();
    }
}
```

Each worker locks the receiver, grabs one message, releases the lock, and processes it. The mutex ensures only one worker takes each message — no duplicates.

### Request-Response: oneshot pattern

Sometimes a thread needs to *ask* a question and wait for an answer. The handler wants to read from the leaderboard cache, but the cache is managed by another thread. Solution: send the question along with a return-address channel.

```rust
use std::sync::mpsc;
use std::thread;

enum CacheRequest {
    GetLeaderboard(mpsc::Sender<Vec<String>>),
    InvalidateLeaderboard,
}

fn main() {
    let (cache_tx, cache_rx) = mpsc::channel::<CacheRequest>();

    // Cache manager thread
    thread::spawn(move || {
        let leaderboard = vec![
            "1. Alice - 7:42 Rx".to_string(),
            "2. Dave - 7:50 Rx".to_string(),
            "3. Carol - 8:18 Rx".to_string(),
        ];

        while let Ok(req) = cache_rx.recv() {
            match req {
                CacheRequest::GetLeaderboard(resp_tx) => {
                    resp_tx.send(leaderboard.clone()).unwrap();
                }
                CacheRequest::InvalidateLeaderboard => {
                    println!("Cache invalidated!");
                }
            }
        }
    });

    // Handler sends a request and waits for the response
    let (resp_tx, resp_rx) = mpsc::channel();
    cache_tx.send(CacheRequest::GetLeaderboard(resp_tx)).unwrap();
    let leaderboard = resp_rx.recv().unwrap();

    for entry in &leaderboard {
        println!("{}", entry);
    }
}
```

The handler creates a one-time response channel, bundles the sender into the request, and then blocks on `resp_rx.recv()`. The cache manager receives the request, does its work, and sends the answer back through the bundled sender. This is the oneshot pattern — one question, one answer.

---

## Channels vs Mutex — When to Use What

| Scenario | Use | Why |
|----------|-----|-----|
| Multiple threads update a counter | Mutex | Simple shared state, no message needed |
| Producer/consumer pipeline | Channel | Decouple producer from consumer |
| Background job processing | Channel | Fire-and-forget, workers process at own pace |
| Read-heavy cache | RwLock | Many readers, few writers |
| Event broadcasting | broadcast channel | One event, many listeners |
| Request-response between tasks | oneshot channel | One question, one answer |

The rule of thumb from Go applies equally in Rust: *do not communicate by sharing memory; share memory by communicating.* If your threads need to pass data from A to B, use a channel. If they need to read and write the same location, use a lock.

---

## Common Pitfalls

**Deadlock with full sync_channel.** If the sender blocks because the buffer is full, but the receiver is waiting for the sender to do something else first, both threads are stuck forever. This is the bounded-channel deadlock — always ensure the receiver is independently draining the channel.

**Receiver dropped = send fails.** If all receivers are dropped, `send()` returns `Err(SendError(value))`. The value is returned to you inside the error so it is not lost. Always handle `SendError` — ignoring it with `.unwrap()` will panic in production when a receiver shuts down unexpectedly.

**Sender dropped = recv gets `RecvError`.** When all senders are dropped, `recv()` returns `Err(RecvError)`. This is not a bug — it is the graceful shutdown signal. The `while let Ok(msg) = rx.recv()` pattern relies on this: when all senders go away, the loop exits cleanly.

**Forgetting to clone the sender.** Each thread needs its own `Sender` clone. If you try to move the same sender into two threads, the compiler will stop you — `Sender` is `Send` but not `Copy`. Clone before moving.

**Graceful shutdown pattern:**

```rust
use std::sync::mpsc;
use std::thread;

fn main() {
    let (tx, rx) = mpsc::channel::<String>();

    let processor = thread::spawn(move || {
        let mut count = 0;
        while let Ok(msg) = rx.recv() {
            count += 1;
            println!("[{}] {}", count, msg);
        }
        // All senders dropped — channel closed
        println!("Processor shutting down. Processed {} messages.", count);
    });

    // Spawn 3 senders
    let handles: Vec<_> = (1..=3).map(|i| {
        let tx = tx.clone();
        thread::spawn(move || {
            tx.send(format!("Score from athlete {}", i)).unwrap();
            // tx is dropped when this thread ends
        })
    }).collect();

    drop(tx); // drop the original sender

    for h in handles {
        h.join().unwrap();
    }
    processor.join().unwrap();
}
```

When the last sender is dropped, `rx.recv()` returns `Err` and the `while let` loop exits. The processor prints its final message and shuts down. No sentinel values, no flags, no special "stop" messages. Rust's ownership system makes shutdown automatic.

---

## Complexity & Mental Model

| Operation | Unbounded | Bounded (cap N) |
|-----------|-----------|-----------------|
| send | O(1) amortized | O(1) or blocks if full |
| recv | O(1) or blocks if empty | O(1) or blocks if empty |
| Memory | Grows without limit | Capped at N x size_of::\<T\>() |
| Sender clone | O(1) | O(1) |

Channels are not a data structure in the traditional sense — they are a *coordination mechanism*. The internal buffer is typically a `VecDeque` (as we built) or a linked list of nodes. What matters is not the asymptotic complexity but the *architectural* property: channels decouple producers from consumers in both time and knowledge. The sender does not know when its message will be processed, and it does not know who processes it. This decoupling is what makes systems scalable.

---

## Try It Yourself

### Exercise 1: Workout Score Processor

Spawn 5 athlete threads, each sending 3 scores (athlete name + score value). One processor thread receives all 15 scores, prints each as it arrives, then prints the total count.

Hint: clone the sender for each thread. Remember to drop the original sender so the processor's `while let Ok(...)` loop terminates.

### Exercise 2: Bounded Channel Back Pressure

Create a `sync_channel(3)`. Have one thread send 10 scores as fast as possible. Have the receiver process each score with a 100ms delay. Observe (via print timestamps or message ordering) that the sender blocks when the buffer is full.

Hint: print "Sending N..." before the send call and "Sent N!" after. The gap between "Sending" and "Sent" reveals when the sender is blocked.

### Exercise 3: Graceful Shutdown

Spawn 3 athlete threads that each send a random number of scores (between 1 and 5). The processor thread prints each score and, when the channel closes, prints a summary: total scores received and the highest score. The program must exit cleanly — no infinite loops, no `.unwrap()` on a closed channel.

Hint: the processor loop is `while let Ok(score) = rx.recv()`. After the loop, all data is in the processor's local state. The channel closing is the signal to print the summary.
