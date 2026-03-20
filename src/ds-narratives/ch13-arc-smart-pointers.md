# Arc & Smart Pointers — "The Equipment Checkout System"

## The Ownership Problem — Why Arc Exists

Your video upload handler needs access to the `StorageBackend`. So does the health check. And the thumbnail generator. And the cleanup job. Five different parts of your server all need to read the same `StorageBackend` config — but Rust says you can't have five owners. `Clone` it five times? That copies the data. What if it holds a connection pool? You can't clone a connection pool. You need SHARED ownership — multiple owners, one piece of data, and when the last owner leaves, the data gets cleaned up automatically. That's `Arc` — Atomic Reference Counting. It's like a gym membership card that tracks how many people are using a piece of equipment, and only puts it away when the last person is done.

Rust's ownership model is strict: every value has exactly one owner. When the owner goes out of scope, the value is dropped. This is what makes Rust memory-safe without a garbage collector. But sometimes one owner is not enough.

```rust,ignore
let storage = StorageBackend::Local(PathBuf::from("/uploads"));

// Handler 1 needs storage
let app = Router::new()
    .route("/upload", post(upload_handler));  // moves storage here...
// Handler 2 needs storage
    .route("/health", get(health_check));     // ERROR: storage already moved!
// Handler 3 needs storage
    .route("/cleanup", post(cleanup_job));    // ERROR: storage already moved!
// Can't move `storage` three times!
```

You could `Clone` the `StorageBackend`. But cloning copies the data. If your backend holds an R2 bucket with credentials, an endpoint URL, and a region config, every clone duplicates all of that. Worse — some types *cannot* be cloned. A database connection pool manages open TCP connections. Cloning it would mean... what? Opening duplicate connections? Sharing half the pool? There is no sensible `Clone` for a connection pool.

What we actually need is multiple READ access to the same data, with automatic cleanup when the last reader is done. Think of it like the gym's shared equipment checkout system. The cable machine does not get duplicated for each athlete. Instead, there is a counter on the clipboard: how many people are currently using it? When you grab the cable, counter goes up. When you finish, counter goes down. When it hits zero, the gym staff knows they can wheel it away for cleaning. Nobody copies the cable machine. Everyone shares the one that exists.

That is reference counting. And in Rust, the single-threaded version is called `Rc<T>`.

---

## Reference Counting — Build Rc\<T\> from Scratch

Before we get to `Arc`, let's build its simpler cousin: `Rc` (Reference Counted). This is a single-threaded smart pointer that tracks how many owners exist for a value.

The idea: allocate the value on the heap alongside a counter. Every time someone "clones" the `Rc`, we increment the counter. Every time an `Rc` is dropped, we decrement the counter. When it hits zero, we deallocate.

```rust
use std::ops::Deref;

struct RcInner<T> {
    value: T,
    ref_count: usize, // single-threaded, so a plain usize is fine
}

struct MyRc<T> {
    ptr: *mut RcInner<T>,
}

impl<T> MyRc<T> {
    fn new(value: T) -> Self {
        // Allocate the inner struct on the heap using Box,
        // then convert to a raw pointer so we manage the lifetime ourselves.
        let inner = Box::new(RcInner {
            value,
            ref_count: 1,
        });
        MyRc {
            // SAFETY: Box::into_raw gives us a valid, heap-allocated pointer.
            // We take ownership of the allocation and will free it in Drop.
            ptr: Box::into_raw(inner),
        }
    }

    fn ref_count(&self) -> usize {
        // SAFETY: self.ptr is valid as long as any MyRc exists,
        // and we only create valid pointers in new() and clone().
        unsafe { (*self.ptr).ref_count }
    }
}

impl<T> Clone for MyRc<T> {
    fn clone(&self) -> Self {
        // SAFETY: self.ptr is valid (ref_count >= 1 means the allocation exists).
        // We increment the count so Drop knows not to free it yet.
        unsafe {
            (*self.ptr).ref_count += 1;
        }
        MyRc { ptr: self.ptr }
    }
}

impl<T> Deref for MyRc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: self.ptr is valid and points to a live RcInner<T>.
        // We return a shared reference — multiple Deref calls are fine
        // because MyRc only provides &T, never &mut T.
        unsafe { &(*self.ptr).value }
    }
}

impl<T> Drop for MyRc<T> {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: self.ptr is valid. We decrement the count.
            (*self.ptr).ref_count -= 1;
            if (*self.ptr).ref_count == 0 {
                // Last owner — reclaim the heap allocation.
                // Box::from_raw reconstructs the Box, which drops the inner
                // value and frees the memory when this Box goes out of scope.
                drop(Box::from_raw(self.ptr));
            }
        }
    }
}

fn main() {
    let a = MyRc::new("Cable Machine".to_string());
    println!("Created. ref_count = {}", a.ref_count()); // 1

    let b = a.clone();
    println!("Cloned once. ref_count = {}", a.ref_count()); // 2

    let c = a.clone();
    println!("Cloned twice. ref_count = {}", a.ref_count()); // 3

    // Deref lets us call String methods directly
    println!("Equipment: {} ({} bytes)", *a, a.len());

    drop(b);
    println!("Dropped b. ref_count = {}", a.ref_count()); // 2

    drop(c);
    println!("Dropped c. ref_count = {}", a.ref_count()); // 1

    // When `a` drops at end of main, ref_count hits 0 and the String is freed.
}
```

Every `unsafe` block here exists because we are working with raw pointers. Rust cannot verify at compile time that `self.ptr` points to valid memory — but *we* maintain the invariant that it does, because we only create it via `Box::into_raw` and only free it when the count reaches zero.

Notice what `MyRc` gives us: the string `"Cable Machine"` exists exactly once on the heap, but `a`, `b`, and `c` all have access to it. No copies. When the last one drops, the memory is freed. This is shared ownership.

---

## The Thread Safety Problem — Why Rc Isn't Enough

Our `MyRc` uses a plain `usize` for the reference count. On a single thread, that works perfectly. But Axum spawns handler tasks on a thread pool. Each incoming HTTP request runs on a potentially different thread, and each one needs the `StorageBackend`.

Here is the problem with `usize` across threads. Imagine two threads dropping their `Rc` at the same time:

```text
Thread A reads ref_count: 3
Thread B reads ref_count: 3     (before A writes!)
Thread A writes ref_count: 2
Thread B writes ref_count: 2    (should be 1!)
```

Both threads read 3, both subtract 1, both write 2. But the count should be 1. We lost a decrement. Later, when the actual last owner drops and decrements to 1 instead of 0, the memory is never freed. Memory leak. Or worse — if the race goes the other direction, the count reaches 0 while an owner still exists, and you get a use-after-free.

Rust prevents this at compile time. The standard library's `Rc<T>` is marked `!Send` — the compiler will refuse to transfer it across thread boundaries. If you try to pass `Rc<StorageBackend>` into a `tokio::spawn`, you get:

```text
error[E0277]: `Rc<StorageBackend>` cannot be sent between threads safely
```

The compiler saves you from the data race before your code ever runs. But you still need shared ownership across threads. Enter atomic operations.

---

## Atomic Operations — The Magic of Arc

The solution is `AtomicUsize` — a special integer type where increment and decrement are *indivisible* at the hardware level. The CPU guarantees that even if two cores execute an atomic increment at the exact same nanosecond, both increments are applied. No read-modify-write race. No lost updates.

The key operations:

- `fetch_add(1, Ordering::Relaxed)` — atomically add 1 and return the old value. "Relaxed" means we do not need to synchronize other memory accesses around this operation — we just need the counter itself to be correct.
- `fetch_sub(1, Ordering::Release)` — atomically subtract 1. The "Release" ordering on drop ensures that all writes to the inner value (from any thread) are visible before we potentially deallocate.
- `fence(Acquire)` — on the last drop (when `fetch_sub` returns 1, meaning we just decremented from 1 to 0), we issue an Acquire fence. This pairs with the Release ordering on all previous drops, ensuring we see all modifications before we free the memory.

Back to the gym analogy: even if two athletes release the cable machine at the EXACT same instant — one finishes their lat pulldown on the east cable, the other finishes their tricep pushdown on the west cable — the atomic counter correctly decrements by 2, not 1. The hardware guarantees it. That is what "Atomic" in `Arc` means.

---

## Build MyArc\<T\> from Scratch

Now let's upgrade `MyRc` to be thread-safe. The changes are surgical: swap `usize` for `AtomicUsize`, and use atomic operations instead of plain reads and writes.

```rust
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering, fence};

struct ArcInner<T> {
    value: T,
    ref_count: AtomicUsize,
}

struct MyArc<T> {
    ptr: *mut ArcInner<T>,
}

// SAFETY: MyArc can be sent across threads if T can.
// The atomic ref_count ensures the pointer management is thread-safe.
// We require T: Send + Sync because multiple threads will read T through &T.
unsafe impl<T: Send + Sync> Send for MyArc<T> {}
unsafe impl<T: Send + Sync> Sync for MyArc<T> {}

impl<T> MyArc<T> {
    fn new(value: T) -> Self {
        let inner = Box::new(ArcInner {
            value,
            ref_count: AtomicUsize::new(1),
        });
        MyArc {
            ptr: Box::into_raw(inner),
        }
    }

    fn ref_count(&self) -> usize {
        // SAFETY: ptr is valid while any MyArc exists.
        // Relaxed is fine — we just want to read the current count,
        // no synchronization with other memory operations needed.
        unsafe { (*self.ptr).ref_count.load(Ordering::Relaxed) }
    }
}

impl<T> Clone for MyArc<T> {
    fn clone(&self) -> Self {
        // SAFETY: ptr is valid (ref_count >= 1).
        // Relaxed ordering is sufficient for increment — the worst case
        // is that a concurrent drop sees a slightly stale count, but
        // fetch_add is atomic so the count itself is always correct.
        unsafe {
            (*self.ptr).ref_count.fetch_add(1, Ordering::Relaxed);
        }
        MyArc { ptr: self.ptr }
    }
}

impl<T> Deref for MyArc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: ptr is valid and the value is initialized.
        // Multiple threads can hold &T simultaneously — that's
        // the whole point of Arc. T: Sync guarantees this is safe.
        unsafe { &(*self.ptr).value }
    }
}

impl<T> Drop for MyArc<T> {
    fn drop(&mut self) {
        unsafe {
            // Release ordering: ensures all our writes to the inner value
            // are visible to whichever thread performs the final drop.
            let old_count = (*self.ptr).ref_count.fetch_sub(1, Ordering::Release);

            if old_count == 1 {
                // We just decremented from 1 to 0 — we are the last owner.
                // Acquire fence: synchronize with all the Release stores
                // from other threads' drops. This ensures we see all their
                // writes before we deallocate.
                fence(Ordering::Acquire);
                drop(Box::from_raw(self.ptr));
            }
        }
    }
}

fn main() {
    let storage = MyArc::new("Local(/uploads)".to_string());
    println!("Created storage. ref_count = {}", storage.ref_count());

    // Simulate five handlers sharing the same storage
    let h1 = storage.clone();
    let h2 = storage.clone();
    let h3 = storage.clone();
    let h4 = storage.clone();
    println!("Four handler clones. ref_count = {}", storage.ref_count()); // 5

    // Each handler can read the storage
    println!("Handler 1 sees: {}", *h1);
    println!("Handler 2 sees: {}", *h2);

    // As handlers complete, they drop their Arc
    drop(h1);
    drop(h2);
    println!("Two handlers done. ref_count = {}", storage.ref_count()); // 3

    drop(h3);
    drop(h4);
    println!("All handlers done. ref_count = {}", storage.ref_count()); // 1

    // storage drops at end of main — ref_count hits 0, memory freed.
}
```

The two `unsafe impl` lines are the key difference from `Rc`. They tell the compiler: "Yes, `MyArc<T>` can cross thread boundaries." We are making a promise that our atomic operations are correct. The compiler trusts us — and if we are wrong, we get data races. This is the contract of `unsafe`: the programmer takes responsibility for invariants the compiler cannot verify.

The `Ordering` parameters deserve a closer look:

- **Clone uses `Relaxed`** — incrementing the count does not need to synchronize with reads or writes to the inner value. We just need the count itself to be atomically correct.
- **Drop uses `Release`** — this is like saying "before I release my claim on this data, make sure all my writes are flushed." It ensures no thread sees a half-written value after deallocation.
- **Last drop uses `Acquire` fence** — this pairs with all the `Release` drops. Before we free the memory, we need to see every write that every other thread made to the inner value. The `Acquire` fence guarantees this.

---

## Arc in GrindIt — Real Usage Patterns

### Arc\<StorageBackend\> — sharing config across handlers

The most common pattern: wrap a read-only value in `Arc` and clone it for each consumer.

```rust,ignore
let storage = Arc::new(StorageBackend::Local(PathBuf::from("/uploads")));
let s1 = storage.clone(); // for upload handler
let s2 = storage.clone(); // for health check
let s3 = storage.clone(); // for cleanup job
// All three point to the SAME StorageBackend on the heap.
// Cost: three atomic increments. The StorageBackend is never copied.
```

Think of it as the gym posting one equipment manual on the wall. Every athlete reads the same manual. Nobody gets their own copy. When the gym closes (last reference drops), the manual comes down.

### Arc\<RwLock\<T\>\> — shared mutable state

`Arc` alone gives you shared *immutable* access. What if multiple threads need to *write*? Wrap the inner value in `RwLock`:

```rust,ignore
use std::sync::{Arc, RwLock};

let leaderboard = Arc::new(RwLock::new(Vec::<Score>::new()));

// Reader threads (leaderboard page) — many can read simultaneously
let lb = leaderboard.read().unwrap();
println!("Top score: {:?}", lb.first());

// Writer thread (score submission) — exclusive access
let mut lb = leaderboard.write().unwrap();
lb.push(Score { athlete: "Maya".into(), value: 205 });
```

Multiple readers can hold the lock simultaneously — checking the leaderboard does not block other viewers. A writer gets exclusive access — while a score is being submitted, all readers wait. This is ideal for data that is read often and written rarely.

### Arc in Axum's State

Axum's `with_state()` is the real-world payoff. When you pass an `Arc` as the app state, Axum clones it for each request handler automatically:

```rust,ignore
let app_state = Arc::new(AppState { db_pool, storage, config });
let app = Router::new()
    .route("/upload", post(upload_handler))
    .route("/health", get(health_check))
    .route("/cleanup", post(cleanup_job))
    .with_state(app_state); // Axum clones the Arc for each handler invocation
```

Every request gets its own `Arc<AppState>` — a pointer to the same data. A thousand concurrent requests share one `AppState`. The atomic counter handles all of it, correctly, at hardware speed. Like a thousand athletes checking out the same cable machine simultaneously, each one incrementing and decrementing the counter without a single lost update.

---

## The Smart Pointer Family — When to Use What

| Pointer | Thread-safe? | Mutability | Use case | GrindIt example |
|---------|-------------|------------|----------|-----------------|
| `Box<T>` | Yes (owned) | Owner has full control | Single owner, heap allocation | `Box<dyn Error>` in error types |
| `Rc<T>` | No (single thread) | Read-only shared | Multiple owners, same thread | -- (not used, we are multi-threaded) |
| `Arc<T>` | Yes | Read-only shared | Multiple owners, multi-threaded | `Arc<StorageBackend>`, `Arc<PgPool>` |
| `Arc<Mutex<T>>` | Yes | Shared mutable (exclusive) | Write access from multiple threads | Rate limiter counter |
| `Arc<RwLock<T>>` | Yes | Shared mutable (readers/writer) | Many readers, few writers | Cached leaderboard |
| `Cow<T>` | N/A | Clone-on-write | Maybe owned, maybe borrowed | Exercise name processing |

The progression is: `Box` when you need heap allocation with a single owner. `Rc` when you need shared ownership on one thread. `Arc` when you need shared ownership across threads. Add `Mutex` or `RwLock` inside the `Arc` when you need shared *mutable* access.

---

## Common Pitfalls

**Arc cycles = memory leak.** If struct A holds an `Arc` to B, and B holds an `Arc` back to A, both reference counts are stuck at 1 forever. Neither can reach 0. This is a memory leak — not a dangling pointer, but memory that is never reclaimed. The fix is `Weak<T>`: a non-owning reference that does not increment the strong count. Call `weak.upgrade()` to get an `Option<Arc<T>>` — it returns `None` if the data has already been freed.

**Arc\<Mutex\<T\>\> deadlock.** If thread 1 locks mutex A then waits for mutex B, while thread 2 locks mutex B then waits for mutex A, both threads block forever. This is the classic deadlock. The rule: always acquire multiple mutexes in the same order across all threads. In GrindIt, we avoid this by keeping each `Arc<Mutex<T>>` independent — no handler ever holds two mutexes at once.

**Cloning Arc is cheap, cloning T is not.** `Arc::clone()` increments an atomic counter — roughly one nanosecond. Cloning the inner data (say, a 10MB video buffer) would take milliseconds and megabytes of RAM. When you see `arc.clone()`, know that it is almost free. But calling `.clone()` on the *dereferenced* inner value is a deep copy. The distinction matters:

```rust,ignore
let a = Arc::new(huge_vec);
let b = a.clone();          // Cheap: just increments the counter
let c = (*a).clone();       // Expensive: clones the entire Vec
```

**Don't use Arc when you can pass references.** If a function just needs to read the data for the duration of a call, take `&T`, not `Arc<T>`. Arc is for *ownership* — when the function might outlive the caller (like a spawned task). For synchronous function calls, a plain borrow is simpler and has zero overhead.

---

## Complexity Table

| Operation | Cost | Notes |
|-----------|------|-------|
| `Arc::new(value)` | O(1) + heap alloc | One allocation for value + counter |
| `Arc::clone()` | O(1) | Atomic increment -- ~1 nanosecond |
| `Arc::drop()` | O(1) usually, O(dealloc) on last drop | Atomic decrement, dealloc when count hits 0 |
| `Deref` (read) | O(1) | Pointer follow -- same cost as `Box` |
| `Mutex` lock | O(1) amortized | May block if contended |
| `RwLock` read | O(1) amortized | Multiple simultaneous readers OK |

The takeaway: `Arc::clone()` is so cheap that you should never hesitate to clone an `Arc`. It is not like cloning a `Vec` or a `String`. It is a single atomic instruction. The gym counter flips from 3 to 4 in a nanosecond. Treat it like passing a pointer — because that is exactly what it is.

---

## Try It Yourself

### Exercise 1: Observe the ref count

Add a `ref_count(&self) -> usize` method to `MyArc` (we already did this above), then write a test that asserts the count changes as clones are created and dropped:

```rust
# use std::ops::Deref;
# use std::sync::atomic::{AtomicUsize, Ordering, fence};
#
# struct ArcInner<T> {
#     value: T,
#     ref_count: AtomicUsize,
# }
#
# struct MyArc<T> {
#     ptr: *mut ArcInner<T>,
# }
#
# unsafe impl<T: Send + Sync> Send for MyArc<T> {}
# unsafe impl<T: Send + Sync> Sync for MyArc<T> {}
#
# impl<T> MyArc<T> {
#     fn new(value: T) -> Self {
#         let inner = Box::new(ArcInner {
#             value,
#             ref_count: AtomicUsize::new(1),
#         });
#         MyArc { ptr: Box::into_raw(inner) }
#     }
#     fn ref_count(&self) -> usize {
#         unsafe { (*self.ptr).ref_count.load(Ordering::Relaxed) }
#     }
# }
#
# impl<T> Clone for MyArc<T> {
#     fn clone(&self) -> Self {
#         unsafe { (*self.ptr).ref_count.fetch_add(1, Ordering::Relaxed); }
#         MyArc { ptr: self.ptr }
#     }
# }
#
# impl<T> Deref for MyArc<T> {
#     type Target = T;
#     fn deref(&self) -> &T { unsafe { &(*self.ptr).value } }
# }
#
# impl<T> Drop for MyArc<T> {
#     fn drop(&mut self) {
#         unsafe {
#             let old = (*self.ptr).ref_count.fetch_sub(1, Ordering::Release);
#             if old == 1 {
#                 fence(Ordering::Acquire);
#                 drop(Box::from_raw(self.ptr));
#             }
#         }
#     }
# }
#
fn main() {
    let a = MyArc::new(42);
    assert_eq!(a.ref_count(), 1);

    let b = a.clone();
    assert_eq!(a.ref_count(), 2);

    let c = a.clone();
    let d = b.clone();
    assert_eq!(a.ref_count(), 4);

    drop(c);
    assert_eq!(a.ref_count(), 3);

    drop(d);
    drop(b);
    assert_eq!(a.ref_count(), 1);

    // Verify the value is still accessible through the last owner
    assert_eq!(*a, 42);
    println!("All ref_count assertions passed!");
}
```

### Exercise 2: Build MyWeak\<T\>

A `Weak` reference does not prevent deallocation. It holds a pointer to the same `ArcInner`, but uses a *separate* weak count. When you call `upgrade()`, it checks whether the strong count is still above zero — if so, it increments the strong count and returns `Some(MyArc<T>)`. If the strong count has already reached zero, the data is gone and `upgrade()` returns `None`.

Hints:
- Add a `weak_count: AtomicUsize` field to `ArcInner`
- `MyWeak::upgrade()` must atomically check the strong count and increment it only if it is greater than zero — use a compare-and-swap loop with `compare_exchange_weak`
- The `ArcInner` allocation is freed when BOTH strong count and weak count reach zero (not just the strong count)
- Start by modifying `MyArc::drop` to only drop the *value* when the strong count hits zero, but keep the allocation alive if weak references exist

### Exercise 3: Shared workout log with Arc\<RwLock\<Vec\<String\>\>\>

Spawn three threads that each push 10 entries into a shared workout log, then verify the final Vec has 30 entries:

```rust
use std::sync::{Arc, RwLock};
use std::thread;

fn main() {
    let log = Arc::new(RwLock::new(Vec::<String>::new()));
    let mut handles = vec![];

    for athlete in 0..3 {
        let log = log.clone();
        let handle = thread::spawn(move || {
            for rep in 0..10 {
                let mut entries = log.write().unwrap();
                entries.push(format!("Athlete {} - Rep {}", athlete, rep));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let entries = log.read().unwrap();
    assert_eq!(entries.len(), 30);
    println!("Workout log has {} entries. First: {}", entries.len(), entries[0]);
    println!("Last: {}", entries[entries.len() - 1]);
}
```

Notice how each thread calls `log.write().unwrap()` to get exclusive access, pushes one entry, and then the lock is released when `entries` goes out of scope. The `RwLock` ensures that no two threads push simultaneously — each push is serialized. After all threads complete, we read the log with `log.read()` and verify all 30 entries are present.

This is the `Arc<RwLock<Vec<T>>>` pattern in miniature — the same pattern GrindIt would use for a cached leaderboard that multiple request handlers read and a background job occasionally updates.
