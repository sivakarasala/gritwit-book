# Mutex & RwLock — "The Single-Occupancy Bathroom vs The Gym Floor"

Your GrindIt server handles 50 requests per second. Each request checks the user's role, reads the exercise cache, and maybe updates the leaderboard. That's 50 threads all reaching for the same data. Without coordination, two threads read the leaderboard count as 99, both add 1, and write 100 — but the answer should be 101. This is a data race, and in most languages it's a silent bug that corrupts your data at 3 AM on a Saturday. In Rust, it won't even compile. The compiler forces you to choose: Mutex (one at a time) or RwLock (many readers, one writer). Let's understand both by building them.

---

## 1. The Data Race Problem

Imagine two GrindIt coaches submitting scores at the exact same moment. Both threads read the leaderboard entry count, both see 99, both write 100. One score vanishes. This is a **lost update** — the classic data race.

In C or Java, this bug compiles fine and hides in production until it corrupts something important. Rust takes a different approach: if two threads can access the same data and at least one can write, the program will not compile. Period.

Rust's solution is `Mutex<T>`. Unlike C's `pthread_mutex_lock` where you lock a mutex and then access whatever memory you want, Rust's Mutex **wraps the data itself**. You literally cannot touch the data without locking. When you call `mutex.lock()`, you get back a `MutexGuard<T>` — a smart pointer that dereferences to `&mut T`. When the guard is dropped, the lock is released. The data and the lock are inseparable.

Think of it as the single-occupancy bathroom at the gym. One person goes in, locks the door. Everyone else waits in line. When they're done, the next person goes in. Simple, fair, and nobody walks in on anyone.

---

## 2. Mutex — Build One from Scratch

Let's build a Mutex using atomics and spin-locking so you can see there is no magic:

```rust
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct MyMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

// SAFETY: MyMutex synchronizes access to T via the atomic lock.
// T: Send is required because T moves between threads.
unsafe impl<T: Send> Sync for MyMutex<T> {}

impl<T> MyMutex<T> {
    pub fn new(value: T) -> Self {
        MyMutex {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> MyMutexGuard<'_, T> {
        // Spin until we swap false -> true
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Hint to the CPU that we're in a spin loop
            std::hint::spin_loop();
        }
        MyMutexGuard { mutex: self }
    }
}

pub struct MyMutexGuard<'a, T> {
    mutex: &'a MyMutex<T>,
}

impl<T> Deref for MyMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: we hold the lock, so exclusive access is guaranteed
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for MyMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: we hold the lock, so exclusive access is guaranteed
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> Drop for MyMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
    }
}

fn main() {
    use std::sync::Arc;
    use std::thread;

    let counter = Arc::new(MyMutex::new(0u64));
    let mut handles = vec![];

    for _ in 0..2 {
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            for _ in 0..1000 {
                let mut guard = counter.lock();
                *guard += 1;
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_val = *counter.lock();
    assert_eq!(final_val, 2000);
    println!("Final value: {} (expected 2000)", final_val);
}
```

Two threads, 1000 increments each, final value is always 2000. The `AtomicBool` acts as the bathroom door lock — `compare_exchange` is the atomic "try to flip the lock from unlocked to locked." If someone else already flipped it, you spin (wait in line). The `UnsafeCell` is Rust's way of saying "I know I'm doing interior mutability, and I promise to synchronize it myself."

Three key pieces make this safe:
- **`Acquire` ordering on lock**: everything the previous lock-holder wrote is visible to us.
- **`Release` ordering on unlock**: everything we wrote is visible to the next lock-holder.
- **`Drop` on the guard**: you cannot forget to unlock. When the guard goes out of scope, the lock is released automatically. The bathroom door swings open whether you remembered or not.

---

## 3. Mutex in GrindIt — Real Patterns

### Rate limiter

The gym bathroom analogy scales directly. A rate limiter is a shared counter that every request checks and updates — classic Mutex territory:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

struct RateLimiter {
    requests: Mutex<HashMap<String, Vec<Instant>>>,
    max_per_minute: usize,
}

impl RateLimiter {
    fn new(max_per_minute: usize) -> Self {
        RateLimiter {
            requests: Mutex::new(HashMap::new()),
            max_per_minute,
        }
    }

    fn check_rate(&self, ip: &str) -> bool {
        let mut map = self.requests.lock().unwrap();
        let now = Instant::now();
        let timestamps = map.entry(ip.to_string()).or_insert_with(Vec::new);

        // Remove entries older than 60 seconds
        timestamps.retain(|t| now.duration_since(*t).as_secs() < 60);

        if timestamps.len() >= self.max_per_minute {
            false // rate limited
        } else {
            timestamps.push(now);
            true // allowed
        }
    }
}

fn main() {
    let limiter = Arc::new(RateLimiter::new(5));
    // First 5 requests pass
    for i in 0..7 {
        let allowed = limiter.check_rate("192.168.1.1");
        println!("Request {}: {}", i + 1, if allowed { "allowed" } else { "BLOCKED" });
    }
}
```

One thread enters the bathroom (locks the HashMap), cleans up old timestamps, checks the count, maybe adds a new one, and leaves. Everyone else waits their turn.

### Shared counter and session store

Two more bread-and-butter patterns:

```rust,ignore
// Active request counter
let active_requests = Arc::new(Mutex::new(0u32));
// In each request handler:
{
    let mut count = active_requests.lock().unwrap();
    *count += 1;
} // MutexGuard dropped here — lock released

// In-memory session store (before you add Redis)
let sessions: Arc<Mutex<HashMap<String, UserSession>>> =
    Arc::new(Mutex::new(HashMap::new()));
```

Both follow the same bathroom pattern: enter, do your business quickly, leave. The curly braces around the counter increment are not cosmetic — they control the guard's lifetime.

---

## 4. The MutexGuard Pattern — RAII Locking

`MutexGuard` is an example of RAII (Resource Acquisition Is Initialization) — the lock is acquired when the guard is created and released when the guard is destroyed. You cannot leak the lock because Rust's ownership system guarantees the destructor runs.

But you can hold the lock too long. This is the single biggest Mutex mistake:

```rust,ignore
// BAD: holds lock across an await point
let mut guard = data.lock().unwrap();
*guard += 1;
do_slow_thing().await; // Still holding the lock! Everyone else is blocked!
drop(guard);           // Too late — the damage is done

// GOOD: scope the guard, drop before await
{
    let mut guard = data.lock().unwrap();
    *guard += 1;
} // Guard dropped here — lock released
do_slow_thing().await; // Lock is free, others can proceed
```

Think of it this way: don't take your phone into the bathroom for a 20-minute scroll session when there is a line. Get in, do what you need, get out.

---

## 5. Poisoning — What Happens When a Thread Panics

If a thread panics while holding a Mutex, the data inside might be in an inconsistent state — imagine the leaderboard update panics mid-sort. The leaderboard is now half-sorted. Should the next reader see that?

Rust says no. It **poisons** the Mutex. Future calls to `lock()` return `Err(PoisonError)` instead of the guard. The `unwrap()` you see on `lock()` is not carelessness — it is a deliberate choice that says "if this Mutex is poisoned, I want to panic too, because the data is probably corrupt."

If you are certain the data is still valid (maybe the panic happened after the modification was complete), you can recover:

```rust,ignore
let guard = data.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
```

But think carefully before doing this. Poisoning exists for a reason.

---

## 6. RwLock — Many Readers, One Writer

The gym bathroom works, but it is overkill for the leaderboard page. Fifty athletes load the leaderboard per second (reads), but only one score is submitted per second (write). With a Mutex, those fifty readers form a single-file line even though reading is perfectly safe to do simultaneously.

Enter **RwLock** — the gym floor. Many people can work out at the same time (readers). But when the cleaning crew needs to mop (writer), everyone has to leave and wait. Multiple readers OR one writer, never both.

```rust
use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let leaderboard = Arc::new(RwLock::new(vec![
        ("Alice".to_string(), 185u32),
        ("Bob".to_string(), 170),
    ]));

    let mut handles = vec![];

    // Spawn 10 readers
    for i in 0..10 {
        let board = Arc::clone(&leaderboard);
        handles.push(thread::spawn(move || {
            let data = board.read().unwrap();
            println!("Reader {}: top score = {} ({})", i, data[0].1, data[0].0);
        }));
    }

    // Spawn 1 writer
    {
        let board = Arc::clone(&leaderboard);
        handles.push(thread::spawn(move || {
            let mut data = board.write().unwrap();
            data.push(("Charlie".to_string(), 190));
            data.sort_by(|a, b| b.1.cmp(&a.1)); // sort descending
            println!("Writer: added Charlie, leaderboard re-sorted");
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_board = leaderboard.read().unwrap();
    println!("Final leaderboard: {:?}", *final_board);
}
```

All ten readers can hold `read()` locks simultaneously — zero contention. The writer calls `write()` and waits until every reader finishes, then gets exclusive access. This is a massive throughput win for read-heavy workloads like leaderboards, exercise caches, and configuration data.

---

## 7. Build a Simple RwLock from Scratch

The trick: use a single atomic integer. Positive values count active readers. -1 means a writer holds the lock. Zero means nobody is inside.

```rust
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicI32, Ordering};

pub struct MyRwLock<T> {
    // > 0: number of readers, -1: writer holds lock, 0: free
    state: AtomicI32,
    data: UnsafeCell<T>,
}

// SAFETY: MyRwLock synchronizes all access via the atomic state.
unsafe impl<T: Send + Sync> Sync for MyRwLock<T> {}

impl<T> MyRwLock<T> {
    pub fn new(value: T) -> Self {
        MyRwLock {
            state: AtomicI32::new(0),
            data: UnsafeCell::new(value),
        }
    }

    pub fn read_lock(&self) -> ReadGuard<'_, T> {
        loop {
            let current = self.state.load(Ordering::Acquire);
            if current >= 0 {
                // Try to increment reader count
                if self
                    .state
                    .compare_exchange(current, current + 1, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    return ReadGuard { lock: self };
                }
            }
            std::hint::spin_loop();
        }
    }

    pub fn write_lock(&self) -> WriteGuard<'_, T> {
        // Wait until state is 0 (no readers, no writer), then set to -1
        while self
            .state
            .compare_exchange(0, -1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
        WriteGuard { lock: self }
    }
}

pub struct ReadGuard<'a, T> {
    lock: &'a MyRwLock<T>,
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: readers only get &T, and writers are excluded
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

pub struct WriteGuard<'a, T> {
    lock: &'a MyRwLock<T>,
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: writer has exclusive access
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: writer has exclusive access
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
    }
}

fn main() {
    use std::sync::Arc;
    use std::thread;

    let data = Arc::new(MyRwLock::new(vec![1, 2, 3]));
    let mut handles = vec![];

    // 5 readers
    for i in 0..5 {
        let data = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            let guard = data.read_lock();
            println!("Reader {}: {:?}", i, *guard);
        }));
    }

    // 1 writer
    {
        let data = Arc::clone(&data);
        handles.push(thread::spawn(move || {
            let mut guard = data.write_lock();
            guard.push(4);
            println!("Writer: pushed 4");
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let guard = data.read_lock();
    println!("Final: {:?}", *guard);
}
```

The gym floor model in code: `read_lock` checks that the state is not -1 (no cleaner mopping), then increments the reader count. `write_lock` waits until the state is exactly 0 (nobody on the floor), then sets it to -1 to claim exclusive access. When a reader leaves, they decrement. When the writer leaves, they reset to 0.

---

## 8. Mutex vs RwLock — Decision Guide

| Factor | Mutex | RwLock |
|--------|-------|--------|
| Read-heavy workload | Readers block each other | Readers run concurrently |
| Write-heavy workload | Simpler, less overhead | Writers starve readers or vice versa |
| Lock duration | Better for short critical sections | Better when reads are slow |
| Complexity | Simpler | Writer starvation risk |
| GrindIt use | Rate limiter, session store, counters | Leaderboard cache, exercise cache, config |

**Rule of thumb:** if more than 80% of accesses are reads, use RwLock. Otherwise, Mutex is simpler and has less overhead. When in doubt, start with Mutex — you can always upgrade to RwLock later if profiling shows reader contention.

---

## 9. Deadlock — The Classic Bug

Deadlock is what happens when two people each block the other. Thread 1 locks the bathroom and shouts "I need a towel from the gym floor!" Thread 2 is on the gym floor holding the last towel, shouting "I need the bathroom!" Neither can proceed.

```rust,ignore
// Thread 1: lock leaderboard, then lock exercises
let _lb = leaderboard.lock().unwrap();
let _ex = exercises.lock().unwrap(); // Blocks — Thread 2 holds this!

// Thread 2: lock exercises, then lock leaderboard
let _ex = exercises.lock().unwrap();
let _lb = leaderboard.lock().unwrap(); // Blocks — Thread 1 holds this!
// DEADLOCK: both threads wait forever
```

Prevention strategies:

1. **Always lock in the same order.** If every thread locks `exercises` before `leaderboard`, deadlock is impossible. Pick an order (alphabetical works) and stick to it.
2. **Use `try_lock()` and back off.** Instead of blocking forever, try to acquire the lock. If it fails, release what you have and retry.
3. **Minimize lock scope.** The shorter you hold a lock, the smaller the window for deadlock. Get in, get out.
4. **Prefer channels when the pattern is complex.** If you find yourself juggling three or four locks, it might be simpler to send messages between threads instead.

---

## 10. Async-Aware Locks — tokio::sync::Mutex

`std::sync::Mutex` blocks the thread. In async code, that's a problem — blocking a thread in a Tokio runtime starves other tasks that share that thread.

```rust,ignore
// std::Mutex in async — blocks the OS thread
let guard = data.lock().unwrap(); // Other async tasks on this thread are frozen

// tokio::sync::Mutex — yields to the runtime
let guard = data.lock().await; // Other tasks can run while we wait
```

The rules:

- Use `std::sync::Mutex` for synchronous code or very short critical sections (nanoseconds, not milliseconds).
- Use `tokio::sync::Mutex` when you must hold a lock across an `.await` point.
- Best practice: avoid holding ANY lock across `.await`. Use the scoping pattern from section 4 — lock, read/write, drop, then await.

In GrindIt's Axum handlers, most Mutex usage is for quick in-memory lookups (rate limiter, session cache). These are fast enough that `std::sync::Mutex` is fine. If you ever need to hold a lock while making a database call, reach for `tokio::sync::Mutex` — or better yet, restructure so you don't hold the lock that long.

---

## 11. Complexity Table

| Operation | Mutex | RwLock |
|-----------|-------|--------|
| lock / read_lock (uncontended) | O(1) — one atomic CAS | O(1) — one atomic CAS |
| lock / write_lock (uncontended) | O(1) | O(1) |
| Under contention | Spins or parks — depends on OS | Readers concurrent, writer waits for all readers |
| Memory overhead | ~1 AtomicBool + UnsafeCell | ~1 AtomicI32 + UnsafeCell |

Both are extremely lightweight. The cost is not the lock itself — it is the contention. A Mutex that is rarely contested is nearly free. A Mutex that is constantly contested is a bottleneck regardless of how clever the implementation is. The fix is not a fancier lock — it is reducing how long you hold it.

---

## 12. Try It Yourself

### Exercise 1: Thread-Safe Rate Limiter

Build a rate limiter using `Arc<Mutex<HashMap<String, Vec<Instant>>>>` that allows a maximum of 5 requests per 10 seconds per IP address. Spawn 10 threads, each simulating requests from one of 3 different IP addresses. Print which requests are allowed and which are blocked.

**Hint:** use `Instant::now()` for timestamps and `retain()` to clean up entries older than 10 seconds.

### Exercise 2: Cached Leaderboard with RwLock

Build a cached leaderboard using `Arc<RwLock<Vec<(String, u32)>>>`. Spawn 20 reader threads that each print the current top score, and 2 writer threads that each add 5 new scores (with random names and values) and re-sort the leaderboard after each insertion. Verify that no thread panics and no data is corrupted.

**Hint:** readers use `read().unwrap()`, writers use `write().unwrap()`. Add a small `thread::sleep` in writers to simulate real work and give readers a chance to interleave.

### Exercise 3: Deadlock Detection

Create two Mutexes (`lock_a` and `lock_b`). Spawn two threads that lock them in opposite order. Instead of `lock()` (which would hang forever), use `try_lock()` with a retry loop to detect the deadlock and print a warning. Then fix it by having both threads lock in the same order (A then B).

**Hint:** `try_lock()` returns `Err(TryLockError::WouldBlock)` if the lock is held by another thread. Use this to detect the problem instead of deadlocking.
