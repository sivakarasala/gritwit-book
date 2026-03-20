# The Front Desk Manager: Async Rust and Futures from Scratch

## Behind the Curtain

You've been writing `.await` all chapter and it just works. Your database queries return data, your server handles multiple requests. But what IS a future? When you type `async fn fetch_exercises()`, the compiler does something remarkable — it transforms your function into a state machine. Every `.await` becomes a pause point. It's time to look behind the curtain.

Picture the front desk at GrindIt HQ on a Saturday morning. Twenty athletes walk in. One needs a locker assignment. Another is waiting for a coach. A third wants the squat rack that's currently occupied. The front desk manager — let's call her Tokio — doesn't stare at one athlete until their locker opens. She checks on each one. "Ready? No? I'll come back." She moves to the next. When a locker frees up, a little buzzer goes off. Tokio circles back. "You're up."

That's async Rust. The athletes are futures. Tokio is the runtime. The buzzer is a waker. And the "are you ready?" check is called **polling**.

## 1. What Is a Future? The Trait

At its core, a `Future` is a value that doesn't exist yet but will eventually. Here's the real trait from the standard library:

```rust
pub trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

And `Poll` is just an enum:

```rust
pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

That's it. The entire foundation of async Rust is one method that returns one of two things: "here's your value" or "not yet."

Think of it this way. Tokio walks up to an athlete at the front desk. "Is your squat rack free?" The athlete either says `Ready("Rack 3 is open!")` or `Pending` — "still waiting." If pending, Tokio moves on. She has nineteen other athletes to check on. She'll be back.

The `Pin<&mut Self>` and `Context` parameters look intimidating. We'll get to both. For now, just know: `Pin` means "don't move me in memory" and `Context` carries a waker — the buzzer that tells Tokio when to come back.

## 2. Build a Future from Scratch

Let's stop reading about futures and build one. A `WorkoutTimer` that simulates a rest period between sets — counting down from N seconds to zero.

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

struct WorkoutTimer {
    seconds_remaining: u32,
}

impl WorkoutTimer {
    fn new(seconds: u32) -> Self {
        WorkoutTimer {
            seconds_remaining: seconds,
        }
    }
}

impl Future for WorkoutTimer {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.seconds_remaining == 0 {
            Poll::Ready("Rest complete! Next set!".to_string())
        } else {
            println!("  ...{} seconds remaining", self.seconds_remaining);
            self.seconds_remaining -= 1;

            // Tell the runtime: "I'm not done, but poll me again soon"
            cx.waker().wake_by_ref();

            Poll::Pending
        }
    }
}
```

Every time Tokio (the front desk manager) checks on this future, it decrements the counter. Not ready? It calls `cx.waker().wake_by_ref()` — "buzz me again, I'll be closer to done." When the counter hits zero: `Ready`. Rest period over. Get back under the bar.

Here it is running on the Tokio runtime:

```rust,ignore
// requires tokio
#[tokio::main]
async fn main() {
    let timer = WorkoutTimer::new(3);
    let message = timer.await;
    println!("{}", message);
}
// Output:
//   ...3 seconds remaining
//   ...2 seconds remaining
//   ...1 seconds remaining
// Rest complete! Next set!
```

That `.await` is doing the polling for you. Every time `WorkoutTimer` returns `Pending`, the runtime calls `poll` again (because we triggered the waker). When it returns `Ready`, `.await` gives you the `String`.

## 3. How async fn Desugars

When you write this:

```rust,ignore
async fn fetch_exercise_name() -> String {
    "Back Squat".to_string()
}
```

The compiler does not generate a function that returns a `String`. It generates a function that returns `impl Future<Output = String>`. The body becomes a state machine.

Here's where it gets interesting. Consider a real GrindIt scenario — logging a workout requires two async steps:

```rust,ignore
async fn log_workout(pool: &PgPool) -> Result<String, Error> {
    // Step 1: fetch today's WOD
    let wod = sqlx::query_as("SELECT * FROM wods WHERE date = CURRENT_DATE")
        .fetch_one(pool)
        .await?;

    // Step 2: save the athlete's score
    sqlx::query("INSERT INTO scores (wod_id, athlete, result) VALUES ($1, $2, $3)")
        .bind(wod.id)
        .bind("Coach Maya")
        .bind("4:32")
        .execute(pool)
        .await?;

    Ok("Score logged!".to_string())
}
```

Two `.await` points. The compiler transforms this into a state machine with three states:

```
 ┌─────────────────────────────────────────────────────┐
 │  State 0: START                                     │
 │  → Create the fetch_wod query                       │
 │  → Store the future, transition to State 1          │
 └──────────────────────┬──────────────────────────────┘
                        │ poll() → Pending
                        ▼
 ┌─────────────────────────────────────────────────────┐
 │  State 1: WAITING FOR WOD                           │
 │  → Poll the fetch_wod future                        │
 │  → If Pending: return Pending                       │
 │  → If Ready(wod): create save_score query,          │
 │    store wod.id, transition to State 2              │
 └──────────────────────┬──────────────────────────────┘
                        │ poll() → Pending
                        ▼
 ┌─────────────────────────────────────────────────────┐
 │  State 2: WAITING FOR SAVE                          │
 │  → Poll the save_score future                       │
 │  → If Pending: return Pending                       │
 │  → If Ready(result): return Ready("Score logged!")  │
 └─────────────────────────────────────────────────────┘
```

Under the hood, the compiler generates something resembling this (simplified, not real output):

```rust,ignore
enum LogWorkoutFuture {
    State0 { pool: PgPool },
    State1 { pool: PgPool, fetch_future: FetchOneFuture },
    State2 { save_future: ExecuteFuture },
    Done,
}
```

Each variant holds exactly the data that's alive at that stage. The `pool` is kept in State0 and State1 because we still need it. By State2, we only need the save future. The compiler is meticulous about this — it drops data the moment it's no longer needed.

When the runtime calls `poll`, the state machine checks which state it's in, polls the inner future for that state, and either stays put (`Pending`) or advances to the next state. Your neat sequential code becomes an efficient, zero-allocation state machine. That's the magic.

## 4. Pin: Why It Exists

Look at this async function:

```rust,ignore
async fn describe_exercise() -> String {
    let name = String::from("Deadlift");
    let name_ref = &name;  // reference to local variable

    fetch_details(name_ref).await;  // ← .await SUSPENDS here

    format!("Exercise: {}", name_ref)  // uses reference AFTER resumption
}
```

When this suspends at `.await`, the compiler stores both `name` and `name_ref` inside the state machine struct. But `name_ref` is a pointer to `name`. If someone moved the entire struct to a different memory address, `name_ref` would point to the old location. Dangling pointer. Undefined behavior. The kind of bug that corrupts your leaderboard at 3 AM.

The state machine struct is **self-referential** — it contains a field that points to another field within itself.

```
 Before move:                    After move:
 ┌─────────────────┐            ┌─────────────────┐
 │ name: "Deadlift"│ ←─┐       │ name: "Deadlift"│    (name_ref still
 │ name_ref: ───────┘   │       │ name_ref: ────┐ │     points to old
 └──────────────────────┘       └───────────────│─┘     address!)
   addr: 0x1000                   addr: 0x2000  │
                                                ▼
                                            0x1000 ← DANGLING!
```

`Pin<&mut Self>` is the solution. It's a wrapper that says: "I promise this value will not move in memory." The compiler enforces this — you can't get a `&mut Self` out of a `Pin<&mut Self>` for types that aren't `Unpin` (and async state machines are not `Unpin`).

Think of it like bolting the squat rack to the floor. Once you've set your bar path — once your references are configured — you cannot drag the rack to the other side of the gym. The bar path (your references) depends on the rack being exactly where it is.

You rarely interact with `Pin` directly. When you write `.await`, the compiler handles pinning for you. But now you know why that first parameter of `poll` looks the way it does: it's a guarantee to the self-referential state machine that nobody will pull the rug out from under its internal pointers.

## 5. Waker: The Callback System

There's a problem with our `WorkoutTimer`. It calls `cx.waker().wake_by_ref()` immediately, which means the runtime polls it again in a tight loop. That's like the front desk manager checking on an athlete every millisecond. Wastes CPU. Burns energy. In a real system, we want the runtime to sleep until something actually happens.

The `Waker` is a callback handle. When a future returns `Pending`, it stores the waker somewhere. Later, when the thing it's waiting for completes — a timer fires, data arrives on a socket, a database responds — that external event triggers the waker. The waker tells the runtime: "Hey, that future on task #47? Poll it again. Something changed."

Gym analogy: each athlete gets a buzzer when they check in, like the pagers at a restaurant. The front desk doesn't keep walking over to ask "is the rack free yet?" Instead, when the rack opens, the system buzzes the next athlete in line. The front desk sees the buzz and says: "Athlete on task #47 — time to check on you."

The flow looks like this:

```
 1. Runtime calls poll() on your future
 2. Future isn't ready → stores the Waker, returns Pending
 3. Runtime parks the task, works on other tasks
 4. ... time passes ...
 5. External event fires (OS: "data on socket 42!")
 6. The stored Waker is called → wake()
 7. Runtime: "Task is awake! Let me poll it again."
 8. poll() → Ready(value)
 9. Your .await resumes with the value
```

This is what makes async efficient. No busy-looping. No wasted polls. Just event-driven wake-ups.

## 6. Build a Mini Executor (The Front Desk Manager)

Tokio is a sophisticated runtime with work-stealing, thread pools, and I/O drivers. But at its heart, every executor does the same thing: poll futures until they're done. Let's build the simplest possible version — our own front desk manager:

```rust
use std::future::Future;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

fn block_on<F: Future>(mut future: F) -> F::Output {
    // Create a no-op waker — our simple executor just busy-polls
    fn dummy_raw_waker() -> RawWaker {
        fn no_op(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker { dummy_raw_waker() }
        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(std::ptr::null(), vtable)
    }

    let waker = unsafe { Waker::from_raw(dummy_raw_waker()) };
    let mut cx = Context::from_waker(&waker);

    // Pin the future to the stack
    let mut future = unsafe { std::pin::Pin::new_unchecked(&mut future) };

    // The entire executor: poll in a loop
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => {
                // A real executor would park the thread here.
                // We just spin. Don't do this in production!
            }
        }
    }
}
```

That's it. Thirty lines. A `Waker` (dummy, since we just spin), a `Context`, and a loop that calls `poll` until the future says `Ready`.

Let's run our `WorkoutTimer` on it:

```rust,ignore
fn main() {
    let result = block_on(WorkoutTimer::new(3));
    println!("{}", result);
}
// Output:
//   ...3 seconds remaining
//   ...2 seconds remaining
//   ...1 seconds remaining
// Rest complete! Next set!
```

This is what Tokio does — but with thousands of futures, smart scheduling, actual thread parking, and an I/O driver that hooks into the operating system's event system (`epoll` on Linux, `kqueue` on macOS). Our front desk manager is a one-person operation checking on one athlete. Tokio is a full-staff gym running a thousand concurrent check-ins.

## 7. Concurrency vs Parallelism in Practice

Loading the GrindIt workout page needs three things: today's WOD, the exercise list, and the leaderboard. Each takes about 100ms to fetch from the database.

Sequential:

```rust,ignore
// requires tokio, sqlx
async fn load_workout_page(pool: &PgPool) -> WorkoutPage {
    let wod = fetch_wod(pool).await;           // 100ms
    let exercises = fetch_exercises(pool).await; // 100ms
    let leaders = fetch_leaderboard(pool).await; // 100ms
    // Total: ~300ms
    WorkoutPage { wod, exercises, leaders }
}
```

Three await points, executed in sequence. 300ms total. The front desk manager checks on athlete 1, waits until they're done, then moves to athlete 2, waits, then athlete 3. Nobody overlaps.

Concurrent with `join!`:

```rust,ignore
// requires tokio, sqlx
use tokio::join;

async fn load_workout_page(pool: &PgPool) -> WorkoutPage {
    let (wod, exercises, leaders) = join!(
        fetch_wod(pool),
        fetch_exercises(pool),
        fetch_leaderboard(pool),
    );
    // Total: ~100ms (all three overlap!)
    WorkoutPage { wod, exercises, leaders }
}
```

Now Tokio fires off all three queries, and while Postgres is thinking about the WOD query, it polls the exercise query, and while both are pending, it polls the leaderboard query. All three database round-trips overlap. Total wall time: the duration of the slowest one. ~100ms instead of 300ms.

`join!` is concurrency on a single task — interleaved polling. `tokio::spawn` goes further:

```rust,ignore
// requires tokio
let handle = tokio::spawn(async {
    heavy_computation().await
});
// This task can run on a DIFFERENT thread
let result = handle.await.unwrap();
```

`spawn` creates an independent task that the runtime can schedule on any thread. That's **parallelism** — actual simultaneous execution. `join!` is one person juggling three balls. `spawn` is three people each holding one ball.

## 8. What Happens When Your SQLx Query Awaits

Let's trace the full lifecycle of a single line of GrindIt code:

```rust,ignore
let exercises = sqlx::query_as::<_, Exercise>("SELECT * FROM exercises")
    .fetch_all(&pool)
    .await;
```

Here's what actually happens, step by step:

1. **You call `.fetch_all(&pool)`** — SQLx creates a `Future`. No query has been sent yet. Futures are lazy.

2. **You write `.await`** — the runtime calls `poll()` on SQLx's future for the first time.

3. **First poll** — SQLx grabs a connection from the pool, serializes the SQL query into the Postgres wire protocol, and writes it to the TCP socket. The OS accepts the bytes into its send buffer. The socket's not readable yet (Postgres hasn't responded), so SQLx registers interest with the OS: "wake me when socket #42 has data." Returns `Poll::Pending`.

4. **Tokio parks this task** — the task goes to sleep. Tokio is now free to run other tasks. Another HTTP request comes in? Tokio handles it. Your server isn't blocked.

5. **Postgres responds** — milliseconds later, Postgres sends result rows back over the network. The OS kernel sees data arrive on socket #42. It notifies Tokio's I/O driver (via `kqueue` on your Mac). Tokio wakes the task.

6. **Second poll** — SQLx reads the bytes from the socket, deserializes Postgres rows into `Vec<Exercise>`. Returns `Poll::Ready(exercises)`.

7. **Your `.await` resumes** — you get `exercises` and your code continues.

This is why async matters for a web server. While request A waits 5ms for Postgres, requests B through Z are being served. One thread handles hundreds of connections because waiting for I/O doesn't block the thread — it just parks the task and moves on. The front desk manager doesn't stand at the locker room door waiting for locker #7 to open. She handles the next person in line. When locker #7 clicks open, the buzzer fires.

## 9. Common Pitfalls

**Blocking the runtime.** This kills your server:

```rust,ignore
async fn bad_rest_timer() {
    std::thread::sleep(Duration::from_secs(60)); // BLOCKS the entire thread!
    println!("Rest done");
}
```

`std::thread::sleep` puts the OS thread to sleep. Tokio can't run other tasks on that thread. If you're on a single-threaded runtime, everything freezes. Use the async version:

```rust,ignore
async fn good_rest_timer() {
    tokio::time::sleep(Duration::from_secs(60)).await; // yields control back
    println!("Rest done");
}
```

**Forgetting `.await`.** Futures are lazy. This does nothing:

```rust,ignore
async fn oops() {
    save_workout_score(score); // Missing .await! The future is created and dropped.
    println!("Saved!"); // Lies. Nothing was saved.
}
```

The compiler warns you (`unused Future that must be used`), but it's easy to miss in a long function.

**`Send` bounds.** When you `tokio::spawn` a future, it might run on a different thread. Every type inside the future must be `Send`. `Rc<T>` is not `Send` (it uses non-atomic reference counting). Use `Arc<T>` instead:

```rust,ignore
// Won't compile — Rc is not Send
let counter = Rc::new(RefCell::new(0));
tokio::spawn(async move {
    *counter.borrow_mut() += 1;
});

// Works — Arc + Mutex are Send
let counter = Arc::new(Mutex::new(0));
tokio::spawn(async move {
    *counter.lock().await += 1;
});
```

**Holding a `MutexGuard` across `.await`.** This is subtle and dangerous:

```rust,ignore
let mutex = Arc::new(tokio::sync::Mutex::new(vec![]));

async fn bad(mutex: &tokio::sync::Mutex<Vec<String>>) {
    let mut guard = mutex.lock().await;
    guard.push("Deadlift".to_string());
    some_async_operation().await; // guard is STILL held here!
    guard.push("Squat".to_string());
} // guard finally drops
```

While `some_async_operation` is pending, the mutex stays locked. Any other task trying to lock it will wait — potentially forever if there's a circular dependency. Fix: drop the guard before awaiting.

```rust,ignore
async fn good(mutex: &tokio::sync::Mutex<Vec<String>>) {
    {
        let mut guard = mutex.lock().await;
        guard.push("Deadlift".to_string());
    } // guard drops here
    some_async_operation().await;
    {
        let mut guard = mutex.lock().await;
        guard.push("Squat".to_string());
    }
}
```

## 10. The Mental Model

| Concept | What it is | GrindIt analogy |
|---------|-----------|-----------------|
| **Future** | A value that will exist later | Athlete waiting for equipment |
| **Poll** | Checking if the value is ready | Front desk checking on athlete |
| **Pending** | Not ready yet | "Still waiting for the squat rack" |
| **Ready** | Done! Here's your value | "Rack's free, you're up!" |
| **Waker** | Callback to re-poll | Restaurant buzzer / pager |
| **Pin** | Don't move me in memory | Squat rack bolted to the floor |
| **Executor** | The thing that drives polling | Gym front desk manager |
| **Spawn** | New independent task | New athlete checking in |
| **join!** | Wait for multiple futures | "Both athletes ready? Let's go!" |

If you remember one thing from this chapter, let it be this: **async Rust is cooperative multitasking driven by polling.** Futures don't run themselves. They sit there, inert, until an executor polls them. Each `.await` is a yield point where the executor can switch to another task. The waker system ensures no CPU is wasted — tasks only get polled when there's reason to believe they've made progress.

Your GrindIt server handles hundreds of simultaneous requests not because it has hundreds of threads, but because waiting for Postgres, waiting for the network, waiting for a file — all of that is just a `Pending` return that frees the thread to do real work elsewhere. The front desk manager never sleeps on the job. She just moves to the next athlete.

## 11. Try It Yourself

**Exercise 1: RestTimer Future**

Implement a `RestTimer` future that returns `Poll::Pending` for exactly N polls, then returns `Poll::Ready("Rest complete!")`. This is similar to `WorkoutTimer` but tracks poll count instead of simulating seconds. Test it with the `block_on` executor from section 6.

```rust,ignore
struct RestTimer {
    polls_remaining: u32,
}

impl Future for RestTimer {
    type Output = &'static str;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Your code here:
        // - If polls_remaining == 0, return Ready
        // - Otherwise, decrement, wake, return Pending
        todo!()
    }
}
```

**Exercise 2: Counting Executor**

Modify the `block_on` function to count how many times it calls `poll` before getting `Ready`. Print the count at the end. Run it with `RestTimer::new(10)` — you should see exactly 11 polls (10 Pending + 1 Ready). This is the "wasted work" metric that real executors try to minimize.

**Exercise 3: Concurrent Fetch with `join!`**

Write an async function that fetches an exercise name and its category concurrently (simulate with `tokio::time::sleep` + return values), then formats them into a single string:

```rust,ignore
// requires tokio
async fn fetch_name() -> String {
    tokio::time::sleep(Duration::from_millis(100)).await;
    "Clean & Jerk".to_string()
}

async fn fetch_category() -> String {
    tokio::time::sleep(Duration::from_millis(100)).await;
    "Olympic Weightlifting".to_string()
}

async fn describe_exercise() -> String {
    // Use tokio::join! to fetch both concurrently.
    // Return: "Clean & Jerk (Olympic Weightlifting)"
    // Total time should be ~100ms, not ~200ms.
    todo!()
}
```

Measure it with `tokio::time::Instant` to prove that `join!` actually overlaps the two sleeps.
