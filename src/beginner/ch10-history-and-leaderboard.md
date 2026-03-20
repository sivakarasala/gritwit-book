# Chapter 10: History & Leaderboard

Your app can log workouts. Now it needs to answer: *what did I do this week? How long is my streak? Who is training the hardest?* History and leaderboards turn raw data into motivation. This chapter builds the workout history timeline (grouped by day, navigable by week), a leaderboard that ranks athletes by weekly volume, and a streak calculator that counts consecutive training days.

The spotlight concept is **collections and sorting** --- the standard library tools for grouping, ordering, and aggregating data. You will use `HashMap` and `BTreeMap` for grouping logs by date, write custom sort comparators with `Ordering` chains for leaderboard ranking, and build a greedy consecutive-day algorithm for streak calculation. These are the same patterns behind every analytics dashboard and feed.

By the end of this chapter, you will have:

- A `get_history_for_week` server function that loads logs for a date range, enriches each with WOD title and exercise details, and groups them into a `BTreeMap<String, Vec<HistoryEntry>>`
- A `HistoryPage` component with a weekly calendar, day-section headers, and workout cards for each entry
- A `streak_days_db` function that calculates the current training streak using a greedy consecutive-day algorithm
- A `leaderboard_db` function that ranks athletes by weekly workout count with Rx-first, score-descending sorting
- A home page stats bar displaying total workouts, exercise count, and day streak

---

## Spotlight: Collections & Sorting Deep Dive

Before we build anything, let us understand the four core programming concepts that power this chapter.

> **Programming Concept: What is Sorting?**
>
> Sorting means arranging items in a specific order. You sort things every day without thinking about it:
>
> - Arranging race results from fastest to slowest
> - Organizing files alphabetically
> - Stacking plates from largest (bottom) to smallest (top)
>
> In programming, sorting takes a list of items and rearranges them according to a rule. The simplest rule is numerical order: `[3, 1, 4, 1, 5]` becomes `[1, 1, 3, 4, 5]`. But the rule can be anything --- alphabetical, by date, by score, or by multiple criteria at once (like "Rx athletes first, then by score").
>
> Rust provides `.sort()` for simple cases and `.sort_by()` when you need a custom rule. We will use `.sort_by()` for the leaderboard, where the rule is more complex than just "bigger number first."

> **Programming Concept: What is a Comparator?**
>
> A comparator is a rule for deciding which of two items comes first when sorting. Think of it as a referee in a competition:
>
> - Given two athletes, the referee says: "Alice wins" or "Bob wins" or "It is a tie"
> - The sorting algorithm asks the referee over and over until every pair is in the right order
>
> In Rust, a comparator is a function that takes two items and returns one of three values:
>
> - `Ordering::Less` --- the first item comes before the second
> - `Ordering::Greater` --- the first item comes after the second
> - `Ordering::Equal` --- they are tied (order does not matter)
>
> ```rust
> // A comparator that sorts numbers in descending order
> |a, b| b.cmp(&a)  // "b before a" means larger numbers first
> ```
>
> You can chain comparators for multi-criteria sorting: "first compare by Rx status, and if tied, compare by score." This is like a race tiebreaker: "first check finish time, and if tied, check bib number."

> **Programming Concept: What is an Algorithm?**
>
> An algorithm is a step-by-step recipe for solving a problem. Just like a cooking recipe tells you exactly what to do ("chop onions, heat oil, fry onions..."), an algorithm tells the computer exactly what steps to follow.
>
> The streak calculation in this chapter is your first real algorithm:
>
> 1. Get all workout dates, sorted from newest to oldest
> 2. Start with today as the "expected" date
> 3. Look at the newest date. Does it match the expected date?
> 4. If yes: add 1 to the streak, move expected back by one day, go to step 3 with the next date
> 5. If no: the streak is broken, stop
>
> This is called a **greedy algorithm** --- at each step, you greedily grab the next consecutive day if possible. No going back, no second-guessing. It is simple, fast, and correct.

> **Programming Concept: What is a HashMap?**
>
> A HashMap is a lookup table where you can find values by their key instantly. Think of a real dictionary:
>
> - You want the definition of "algorithm" --- you do not read every page from A to Z. You jump straight to the "A" section and find it.
> - A HashMap works the same way: given a key (like a date string "2026-03-15"), it jumps straight to the associated value (like a list of workouts from that day).
>
> ```rust
> use std::collections::HashMap;
>
> let mut workouts: HashMap<String, Vec<String>> = HashMap::new();
> workouts.insert("Monday".to_string(), vec!["Fran".to_string()]);
> workouts.insert("Tuesday".to_string(), vec!["Grace".to_string()]);
>
> // Instant lookup --- no scanning through the whole collection
> if let Some(monday_wods) = workouts.get("Monday") {
>     println!("Monday workouts: {:?}", monday_wods);
> }
> ```
>
> Rust has two map types:
> - **`HashMap`** --- fast (instant lookup), but items come out in random order
> - **`BTreeMap`** --- slightly slower, but items always come out in sorted order
>
> For dates, we want sorted order (Monday before Tuesday), so we use `BTreeMap`.

### HashMap vs BTreeMap in practice

Rust's standard library offers two primary map types. The difference is simple: `HashMap` is unordered and `BTreeMap` keeps keys sorted.

```rust
use std::collections::HashMap;
use std::collections::BTreeMap;

// HashMap: O(1) average lookup, unordered
let mut scores: HashMap<String, Vec<i32>> = HashMap::new();
scores.entry("Alice".to_string()).or_default().push(95);

// BTreeMap: O(log n) lookup, keys always sorted
let mut by_date: BTreeMap<String, Vec<WorkoutLog>> = BTreeMap::new();
by_date.entry("2026-03-15".to_string()).or_default().push(log);
```

What does "O(1)" and "O(log n)" mean? These describe how fast the operation is:

- **O(1)** means "constant time" --- it takes the same amount of time whether you have 10 items or 10 million. Like looking up a word in a dictionary when you know the exact page number.
- **O(log n)** means "logarithmic time" --- it gets slightly slower as the collection grows, but very slowly. With 1,000 items, it takes about 10 steps. With 1,000,000 items, only about 20 steps.

Both are fast. The real difference is ordering.

The GrindIt history page groups workout logs by date. Because dates in `YYYY-MM-DD` format sort alphabetically the same way they sort chronologically (2026-03-14 comes before 2026-03-15), a `BTreeMap<String, Vec<HistoryEntry>>` guarantees that when you iterate, Monday comes before Tuesday:

```rust
use std::collections::BTreeMap;

let mut by_date: BTreeMap<String, Vec<HistoryEntry>> = BTreeMap::new();

// Pre-populate all 7 days of the week (even rest days)
let mut d = start;
while d <= end {
    by_date.insert(d.to_string(), vec![]);
    d += chrono::Duration::days(1);
}

// Slot each entry into its date
for entry in entries {
    by_date
        .entry(entry.log.workout_date.clone())
        .or_default()
        .push(entry);
}

// Iteration is guaranteed chronological
for (date, logs) in &by_date {
    println!("{}: {} workouts", date, logs.len());
}
```

If you used a `HashMap` here, the days would appear in random order. You would need to collect into a `Vec` and sort --- extra work that `BTreeMap` gives you for free.

### The .entry() API

Both map types share the `entry()` API, one of Rust's most elegant collection patterns. Let us see the problem it solves:

```rust
// Without entry(): awkward check-then-insert
if let Some(list) = map.get_mut(&key) {
    list.push(value);
} else {
    map.insert(key, vec![value]);
}

// With entry(): one expression
map.entry(key).or_default().push(value);
```

What is happening here? `entry()` looks up the key and returns one of two things:

- **`Occupied`** --- the key exists, here is a reference to its value
- **`Vacant`** --- the key does not exist

`.or_default()` says: "if the entry is vacant, insert the default value for this type." For `Vec<T>`, the default is an empty vector. Then it gives you a mutable reference to the value, so you can push directly.

This is a common pattern in Rust. You will see `entry().or_default()` whenever you need to group items into collections.

### Custom sorting with Ordering

In Rust, `sort_by()` takes a comparator that returns `std::cmp::Ordering`:

```rust
use std::cmp::Ordering;

// Sort scores in descending order (highest first)
scores.sort_by(|a, b| b.count.cmp(&a.count));
```

For multi-criteria sorting, Rust uses `then_with()` to chain comparisons. The leaderboard needs: Rx athletes first, then higher score beats lower score:

```rust
entries.sort_by(|a, b| {
    // Rx first: true > false, so reverse for Rx-first
    b.is_rx.cmp(&a.is_rx)
        // Higher score is better (for AMRAP/EMOM)
        .then_with(|| b.score_value.cmp(&a.score_value))
        // Lower time is better (for ForTime)
        .then_with(|| a.finish_time_seconds.cmp(&b.finish_time_seconds))
});
```

How does `then_with()` work? It only evaluates the next comparison if the previous one returned `Equal`. Think of it like tiebreakers in a race:

1. First, check Rx status. If one is Rx and the other is not, we are done.
2. If both are Rx (or both scaled), check score. If scores differ, we are done.
3. If scores are also equal, check time.

The chain stops as soon as it finds a difference. This is called **lazy evaluation** --- it does not do unnecessary work.

### Iterator chains for stats aggregation

Rust iterators provide tools for processing collections step by step:

```rust
// Count total workouts
let total: usize = week_data.iter()
    .map(|(_, entries)| entries.len())  // extract the count from each day
    .sum();                              // add them all up

// Count Rx workouts
let rx_count = entries.iter()
    .filter(|e| e.log.is_rx)  // keep only Rx entries
    .count();                   // count them

// filter_map: filter and transform in one step
let total_reps: i32 = movement_logs.iter()
    .filter_map(|m| m.reps)  // skip None values, unwrap Some values
    .sum();
```

`filter_map()` is especially useful for `Option<T>` fields. It skips `None` values and unwraps `Some` values in one step --- like saying "give me all the reps that exist."

---

## Building the History Data Layer

### The HistoryEntry struct

A workout log in isolation is not enough for display. You need the WOD title, the exercises performed, section scores, and movement-level details. The `HistoryEntry` struct bundles all of this:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct HistoryEntry {
    pub log: WorkoutLog,
    pub wod_title: Option<String>,
    pub exercises: Vec<WorkoutExercise>,
    pub section_scores: Vec<SectionScoreWithMeta>,
    pub movement_logs: Vec<MovementLogWithName>,
    pub movement_log_sets: Vec<MovementLogSet>,
}
```

This is a *view model* --- a struct shaped for the UI, not for the database. The database stores `workout_logs`, `workout_exercises`, `section_logs`, and `movement_logs` in separate tables. The server function assembles them into a single `HistoryEntry` per log, like a waiter assembling your meal from multiple kitchen stations.

### The get_history_for_week server function

This function loads all workout logs in a date range, enriches each one, and groups by date:

```rust
#[server]
async fn get_history_for_week(
    start_date: String,
    end_date: String,
) -> Result<Vec<(String, Vec<HistoryEntry>)>, ServerFnError> {
    use chrono::NaiveDate;
    use std::collections::BTreeMap;

    let user = crate::auth::session::require_auth().await?;
    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = user.id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    let start: NaiveDate = start_date.parse()
        .map_err(|e| ServerFnError::new(format!("Invalid start date: {}", e)))?;
    let end: NaiveDate = end_date.parse()
        .map_err(|e| ServerFnError::new(format!("Invalid end date: {}", e)))?;

    let logs = crate::db::list_workouts_by_date_range_db(&pool, user_uuid, start, end)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Enrich each log with WOD title, exercises, scores
    let mut entries = Vec::with_capacity(logs.len());
    for log in logs {
        let wod_title = if let Some(ref wod_id) = log.wod_id {
            let uuid: uuid::Uuid = wod_id.parse()
                .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
            let title: Option<(String,)> =
                sqlx::query_as("SELECT title FROM wods WHERE id = $1")
                    .bind(uuid)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
            title.map(|(t,)| t)
        } else {
            None
        };

        let log_uuid: uuid::Uuid = log.id.parse()
            .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

        let exercises = crate::db::list_workout_exercises_db(&pool, log_uuid)
            .await.map_err(|e| ServerFnError::new(e.to_string()))?;

        // Load section data concurrently with tokio::join!
        let (section_scores, movement_logs, movement_log_sets) = if log.wod_id.is_some() {
            let (scores, movements, sets) = tokio::join!(
                crate::db::get_section_scores_with_meta_db(&pool, log_uuid),
                crate::db::get_movement_logs_with_names_db(&pool, log_uuid),
                crate::db::get_movement_log_sets_db(&pool, log_uuid),
            );
            (
                scores.map_err(|e| ServerFnError::new(e.to_string()))?,
                movements.map_err(|e| ServerFnError::new(e.to_string()))?,
                sets.map_err(|e| ServerFnError::new(e.to_string()))?,
            )
        } else {
            (vec![], vec![], vec![])
        };

        entries.push(HistoryEntry {
            log, wod_title, exercises, section_scores,
            movement_logs, movement_log_sets,
        });
    }

    // Group by date using BTreeMap for chronological order
    let mut by_date: BTreeMap<String, Vec<HistoryEntry>> = BTreeMap::new();
    let mut d = start;
    while d <= end {
        by_date.insert(d.to_string(), vec![]);
        d += chrono::Duration::days(1);
    }
    for entry in entries {
        by_date.entry(entry.log.workout_date.clone())
            .or_default()
            .push(entry);
    }

    Ok(by_date.into_iter().collect())
}
```

Let us walk through the three collection patterns in this function:

1. **`Vec::with_capacity(logs.len())`** --- pre-allocates the exact size. When you know how many items you will add, this avoids the vector needing to grow (and copy everything) as you push items in. Think of it like getting a box that is the right size instead of starting with a tiny box and upgrading to bigger ones.

2. **`BTreeMap` with pre-populated keys** --- every day of the week gets an entry, even rest days with empty vectors. This makes the UI simpler: it can iterate over the map without checking for missing dates. A rest day just has zero entries.

3. **`.into_iter().collect()`** --- consumes the `BTreeMap` into a `Vec<(String, Vec<HistoryEntry>)>`. The return type is a `Vec` of pairs because server functions need types that serialize cleanly, and the ordered-pair format is explicit about the structure.

### Concurrent enrichment with tokio::join!

For WOD logs, three queries are needed: section scores, movement logs, and movement log sets. These are independent of each other, so `tokio::join!` runs them concurrently:

```rust
let (scores, movements, sets) = tokio::join!(
    crate::db::get_section_scores_with_meta_db(&pool, log_uuid),
    crate::db::get_movement_logs_with_names_db(&pool, log_uuid),
    crate::db::get_movement_log_sets_db(&pool, log_uuid),
);
```

Think of this like ordering three dishes at a restaurant. Without `tokio::join!`, you would order dish 1, wait for it to arrive, then order dish 2, wait, then order dish 3. With `tokio::join!`, you order all three at once and they arrive as they are ready. The total wait time is the time of the slowest dish, not the sum of all three.

This is the async equivalent of JavaScript's `Promise.all()`.

---

## Streak Calculation: A Greedy Algorithm

The streak counter answers: "how many consecutive days (ending today or yesterday) has this user logged a workout?" This is your first real algorithm.

### The algorithm step by step

Imagine you have workout dates on a calendar: March 18, 17, 16, 15, and March 10. Today is March 18.

1. Start with `expected = March 18` (today) and `streak = 0`
2. Look at March 18 --- it matches! `streak = 1`, `expected = March 17`
3. Look at March 17 --- it matches! `streak = 2`, `expected = March 16`
4. Look at March 16 --- it matches! `streak = 3`, `expected = March 15`
5. Look at March 15 --- it matches! `streak = 4`, `expected = March 14`
6. Look at March 10 --- it is before March 14. Gap found! Stop.
7. Result: 4-day streak

Here is the code:

```rust
pub async fn streak_days_db(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<i64, sqlx::Error> {
    let dates: Vec<(chrono::NaiveDate,)> = sqlx::query_as(
        r#"SELECT DISTINCT workout_date
           FROM workout_logs
           WHERE user_id = $1 AND workout_date <= CURRENT_DATE
           ORDER BY workout_date DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    if dates.is_empty() {
        return Ok(0);
    }

    let today = chrono::Local::now().date_naive();
    let mut streak = 0i64;
    let mut expected = today;

    for (date,) in dates {
        // Allow starting from today or yesterday
        if streak == 0 && date == today - chrono::Duration::days(1) {
            expected = today - chrono::Duration::days(1);
        }
        if date == expected {
            streak += 1;
            expected -= chrono::Duration::days(1);
        } else if date < expected {
            break; // gap found --- streak ends
        }
    }

    Ok(streak)
}
```

Let us break down the key parts:

- **Dates come sorted descending** from the database (`ORDER BY workout_date DESC`). This means the most recent date is first, which is exactly what we need --- we walk backward from today.
- **The `expected` variable** tracks which date should come next. It starts at today and moves back by one day each time we find a match.
- **The special case at `streak == 0`**: if it is 10 AM and you have not worked out today, but you worked out yesterday, your streak should still count. So if the first date is yesterday, we adjust `expected` to start from yesterday instead of today.
- **`break` on a gap**: as soon as the date is before our expected date, there is a gap and the streak is over. No need to check the remaining dates.

This is a **greedy** algorithm: at each step, we greedily extend the streak by one day if possible. No going back, no complex calculations. One pass through the data, O(n) time.

### DSA connection: Longest Consecutive Sequence

This streak calculation is a variation of a classic problem (LeetCode 128): given an unsorted array of integers, find the length of the longest consecutive sequence. The classic O(n) solution uses a `HashSet`:

```rust
fn longest_consecutive(nums: Vec<i32>) -> i32 {
    use std::collections::HashSet;
    let set: HashSet<i32> = nums.into_iter().collect();
    let mut best = 0;

    for &n in &set {
        // Only start counting from the beginning of a sequence
        if !set.contains(&(n - 1)) {
            let mut len = 1;
            while set.contains(&(n + len)) {
                len += 1;
            }
            best = best.max(len);
        }
    }
    best
}
```

Our streak calculation is simpler because we only care about the sequence anchored at today (or yesterday). But the pattern --- checking consecutive values greedily --- is the same.

---

## The Leaderboard

### Ranking by workout count

The weekly leaderboard ranks athletes by how many workouts they logged this week:

```rust
pub async fn leaderboard_db(
    pool: &sqlx::PgPool,
    limit: i64,
    viewer_email: &str,
    is_viewer_admin: bool,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    let test_emails: &[&str] = &["test@coach.com", "test@athlete.com"];
    let rows: Vec<(String, Option<String>, i64)> = sqlx::query_as(
        r#"SELECT u.display_name, u.avatar_url, COUNT(wl.id) as workout_count
           FROM users u
           LEFT JOIN workout_logs wl ON wl.user_id = u.id
               AND wl.workout_date >= date_trunc('week', CURRENT_DATE)::date
           WHERE ($2 OR u.email = $3 OR u.email != ALL($4))
           GROUP BY u.id, u.display_name, u.avatar_url
           HAVING COUNT(wl.id) > 0
           ORDER BY workout_count DESC
           LIMIT $1"#,
    )
    .bind(limit)
    .bind(is_viewer_admin)
    .bind(viewer_email)
    .bind(test_emails)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(display_name, avatar_url, workout_count)| {
        LeaderboardEntry { display_name, avatar_url, workout_count }
    }).collect())
}
```

The SQL handles the ranking with `ORDER BY workout_count DESC`. The WHERE clause hides test accounts from regular users but shows them to admins --- a common production concern.

Let us read the SQL piece by piece:

- **`LEFT JOIN workout_logs`** --- includes users even if they have zero workouts (though `HAVING COUNT > 0` filters them out)
- **`date_trunc('week', CURRENT_DATE)`** --- PostgreSQL function that gives the start of the current week
- **`HAVING COUNT(wl.id) > 0`** --- only show users who have at least one workout this week
- **`ORDER BY workout_count DESC`** --- highest workout count first
- **`LIMIT $1`** --- only return the top N results

### Section leaderboard: multi-criteria sorting

The section-level leaderboard ranks athletes by their best score on a specific WOD section. The sorting rule depends on the workout type:

```rust
pub async fn section_leaderboard_db(
    pool: &sqlx::PgPool,
    section_id: uuid::Uuid,
    section_type: &str,
    limit: i64,
) -> Result<Vec<SectionLeaderboardEntry>, sqlx::Error> {
    // ForTime: lower is better; AMRAP/EMOM/Strength: higher is better
    let order = if section_type == "fortime" { "ASC" } else { "DESC" };

    let query = format!(
        r#"SELECT u.display_name, u.avatar_url, sl.score_value,
                  sl.is_rx, sl.finish_time_seconds, sl.rounds_completed,
                  sl.extra_reps, sl.weight_kg
           FROM section_logs sl
           JOIN workout_logs wl ON wl.id = sl.workout_log_id
           JOIN users u ON u.id = wl.user_id
           WHERE sl.section_id = $1 AND sl.score_value IS NOT NULL
           ORDER BY sl.is_rx DESC, sl.score_value {}
           LIMIT $2"#,
        order
    );

    // ... execute and map rows
}
```

The `ORDER BY sl.is_rx DESC, sl.score_value {order}` clause implements two-tier ranking in SQL: Rx first (PostgreSQL sorts `true` before `false` when descending), then by score in the appropriate direction.

If you needed to do this sorting in Rust instead of SQL:

```rust
entries.sort_by(|a, b| {
    b.is_rx.cmp(&a.is_rx)
        .then_with(|| {
            if section_type == "fortime" {
                a.score_value.cmp(&b.score_value)  // lower is better
            } else {
                b.score_value.cmp(&a.score_value)  // higher is better
            }
        })
});
```

---

## Building the History Page

### The HistoryPage component

The history page combines a weekly calendar with a scrollable timeline of workout cards:

```rust
#[component]
pub fn HistoryPage() -> impl IntoView {
    let today = crate::pages::wod::week_calendar::today_iso();
    let params = leptos_router::hooks::use_query_map();
    let initial_date = {
        let d = params.read_untracked().get("date")
            .unwrap_or_default().to_string();
        if d.is_empty() { today.clone() } else { d }
    };
    let selected_date = RwSignal::new(initial_date);
    let anchor = RwSignal::new(String::new());
    let is_loading = RwSignal::new(false);

    let delete_action = ServerAction::<DeleteHistoryEntry>::new();
    let show_delete = RwSignal::new(false);
    let pending_delete_log_id = RwSignal::new(String::new());

    let week_range = Memo::new(move |_| {
        let (_, dates) = compute_week_dates(&anchor.get());
        let start = dates.first().cloned().unwrap_or_default();
        let end = dates.last().cloned().unwrap_or_default();
        (start, end)
    });

    let history = Resource::new(
        move || {
            let (start, end) = week_range.get();
            (start, end, delete_action.version().get())
        },
        move |(start, end, _)| async move {
            is_loading.set(true);
            let result = if start.is_empty() || end.is_empty() {
                Ok(vec![])
            } else {
                get_history_for_week(start, end).await
            };
            is_loading.set(false);
            result
        },
    );

    // ... view rendering
}
```

There are three important reactive patterns here. Let us understand each one:

**1. `Memo` for derived state.** A `Memo` is like a formula in a spreadsheet. Cell A1 has a value, and cell B1 has a formula `= A1 * 2`. When A1 changes, B1 automatically recalculates.

Here, `week_range` is derived from `anchor`. When the user swipes the calendar to a new week, `anchor` changes. This causes `week_range` to recompute, which causes `history` to refetch data. It is a chain reaction, and Leptos handles it automatically.

**2. `Resource::new` with a composite dependency tuple.** The first closure returns `(start, end, version)`. The Resource refetches whenever any element of this tuple changes. Including `delete_action.version()` means the history also refetches after a deletion --- the version number increments, which tells the Resource "something changed, reload."

**3. Loading state managed separately.** The `is_loading` signal is set to `true` before the fetch and `false` after. This drives a thin loading bar at the top of the page --- subtle feedback without a full-screen spinner.

### Rendering the weekly timeline

The view iterates over the grouped data and renders day sections:

```rust
view! {
    <div class="weekly-timeline">
        {week_data.into_iter().map(|(date, entries)| {
            let header = format_day_header(&date);
            let day_id = format!("day-{}", date);
            let is_future = date > today;
            view! {
                <div class="day-section" id=day_id>
                    <div class="day-header">{header}</div>
                    {if entries.is_empty() {
                        if is_future {
                            view! { <p class="upcoming-day">"Upcoming"</p> }.into_any()
                        } else {
                            view! { <p class="rest-day">"No workouts logged"</p> }.into_any()
                        }
                    } else {
                        view! {
                            <div class="results-feed">
                                {entries.into_iter().map(|entry| {
                                    view! { <HistoryCard entry=entry
                                        show_delete=show_delete
                                        pending_delete_log_id=pending_delete_log_id/> }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </div>
            }
        }).collect_view()}
    </div>
}
```

Notice the `into_any()` calls on each branch of the if/else. Remember from Chapter 9: in Rust, both branches of an if/else must return the same type. `.into_any()` converts different view types into a common `AnyView` type, resolving the mismatch.

The `format_day_header` function converts `"2026-03-15"` to `"SUNDAY, MAR 15"` using integer date arithmetic (no chrono on the client).

### Scroll-to-day with Effect

When the user taps a day in the calendar, the page scrolls to that day's section:

```rust
Effect::new(move |_| {
    let _date = selected_date.get();
    #[cfg(feature = "hydrate")]
    {
        let _ = js_sys::eval(&format!(
            "setTimeout(function(){{var el=document.getElementById('day-{_date}');\
             if(el){{el.scrollIntoView({{behavior:'smooth',block:'start'}})}}}},50)"
        ));
    }
});
```

The `#[cfg(feature = "hydrate")]` guard ensures this JavaScript only runs in the browser, not during server-side rendering (where there is no DOM to scroll). The `setTimeout` with 50ms delay gives the DOM time to render the new week's data before scrolling.

---

## The Home Page Dashboard

The home page aggregates stats into a quick-glance dashboard:

```rust
#[server]
async fn get_dashboard() -> Result<DashboardData, ServerFnError> {
    let user = crate::auth::session::require_auth().await?;
    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = user.id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    let exercises = crate::db::count_exercises_db(&pool).await.unwrap_or(0);
    let workouts = crate::db::count_workouts_db(&pool, user_uuid).await.unwrap_or(0);
    let streak = crate::db::streak_days_db(&pool, user_uuid).await.unwrap_or(0);
    let is_admin = matches!(user.role, crate::auth::UserRole::Admin);
    let leaderboard = crate::db::leaderboard_db(
        &pool, 5, user.email.as_deref().unwrap_or(""), is_admin
    ).await.unwrap_or_default();

    let today = chrono::Local::now().date_naive();
    Ok(DashboardData {
        day_name: today.format("%A").to_string(),
        full_date: today.format("%B %-d, %Y").to_string(),
        exercises, workouts, streak, leaderboard,
    })
}
```

Notice the `unwrap_or(0)` and `unwrap_or_default()` calls. These are a **graceful degradation** pattern. Dashboard stats should never crash the page --- if a count query fails, showing 0 is better than showing an error. This is like a car dashboard where a broken speedometer shows "---" instead of crashing the whole car.

The stats bar renders three cards:

```rust
<div class="stats-bar">
    <div class="stats-bar-item">
        <span class="stats-bar-num">{data.workouts}</span>
        <span class="stats-bar-label">"Workouts"</span>
    </div>
    <div class="stats-bar-divider"></div>
    <div class="stats-bar-item">
        <span class="stats-bar-num">{data.exercises}</span>
        <span class="stats-bar-label">"Exercises"</span>
    </div>
    <div class="stats-bar-divider"></div>
    <div class="stats-bar-item">
        <span class="stats-bar-num">{data.streak}</span>
        <span class="stats-bar-label">"Day Streak"</span>
    </div>
</div>
```

---

## Rust Gym

### Sorting with custom comparators

```rust
// Sort athletes: Rx first, then by score descending, then by name alphabetically
fn sort_leaderboard(entries: &mut Vec<(String, i32, bool)>) {
    entries.sort_by(|a, b| {
        b.2.cmp(&a.2)                    // Rx first (bool: true > false)
            .then_with(|| b.1.cmp(&a.1)) // higher score wins
            .then_with(|| a.0.cmp(&b.0)) // alphabetical name as tiebreaker
    });
}
```

Let us trace through this with an example. Given athletes:
- Alice (score 150, Rx)
- Bob (score 180, Scaled)
- Carol (score 150, Rx)
- Dave (score 200, Rx)

Step 1: Compare Rx status. Dave, Alice, and Carol are Rx; Bob is Scaled. Bob goes last.
Step 2: Among Rx athletes, compare scores. Dave (200) beats Alice (150) and Carol (150).
Step 3: Alice and Carol are tied on everything except name. "Alice" < "Carol" alphabetically, so Alice comes first.

Result: Dave, Alice, Carol, Bob.

<details>
<summary>Test your understanding</summary>

```rust
fn main() {
    let mut athletes = vec![
        ("Alice".to_string(), 150, true),
        ("Bob".to_string(), 180, false),
        ("Carol".to_string(), 150, true),
        ("Dave".to_string(), 200, true),
    ];
    sort_leaderboard(&mut athletes);
    // Result: Dave (Rx, 200), Alice (Rx, 150), Carol (Rx, 150), Bob (scaled, 180)
    assert_eq!(athletes[0].0, "Dave");
    assert_eq!(athletes[1].0, "Alice");
    assert_eq!(athletes[2].0, "Carol");
    assert_eq!(athletes[3].0, "Bob");
    println!("All assertions passed!");
}
```
</details>

### Group-by pattern

The group-by pattern collects items into buckets based on a key:

```rust
use std::collections::HashMap;

fn group_by_category(items: Vec<(String, String)>) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for (name, category) in items {
        groups.entry(category).or_default().push(name);
    }
    groups
}
```

This is the same `entry().or_default().push()` pattern from the history function. It appears everywhere in data processing.

<details>
<summary>Exercise: implement group_by_first_letter</summary>

Group a list of exercise names by their first letter, uppercased. Use a `BTreeMap` so the letters come out in alphabetical order.

```rust
use std::collections::BTreeMap;

fn group_by_first_letter(names: Vec<&str>) -> BTreeMap<char, Vec<String>> {
    let mut groups: BTreeMap<char, Vec<String>> = BTreeMap::new();
    for name in names {
        if let Some(first) = name.chars().next() {
            groups.entry(first.to_ascii_uppercase())
                .or_default()
                .push(name.to_string());
        }
    }
    groups
}

fn main() {
    let names = vec!["Back Squat", "Bench Press", "Deadlift", "Dumbbell Row"];
    let grouped = group_by_first_letter(names);
    assert_eq!(grouped[&'B'].len(), 2);  // Back Squat, Bench Press
    assert_eq!(grouped[&'D'].len(), 2);  // Deadlift, Dumbbell Row
    println!("{:?}", grouped);
}
```
</details>

### Consecutive sequence detection

<details>
<summary>Exercise: find the longest training streak in a list of dates</summary>

Given a list of day-of-year numbers, find the longest consecutive streak:

```rust
fn longest_streak(mut dates: Vec<i32>) -> i32 {
    if dates.is_empty() { return 0; }
    dates.sort();       // put dates in order
    dates.dedup();      // remove duplicates

    let mut best = 1;
    let mut current = 1;

    // .windows(2) gives us pairs of consecutive elements
    for window in dates.windows(2) {
        if window[1] == window[0] + 1 {
            // consecutive! extend the streak
            current += 1;
            best = best.max(current);
        } else {
            // gap found, reset the counter
            current = 1;
        }
    }
    best
}

fn main() {
    // Day-of-year numbers for workout dates
    let dates = vec![1, 2, 3, 10, 11, 12, 13, 14, 20];
    assert_eq!(longest_streak(dates), 5); // days 10-14
    println!("Longest streak: 5 days");
}
```

How `.windows(2)` works: it slides a window of size 2 across the vector. For `[1, 2, 3]`, it produces `[1, 2]`, then `[2, 3]`. This lets you compare each element with the one before it --- perfect for finding consecutive sequences.
</details>

---

## Exercises

### Exercise 1: Build the history page with weekly calendar and grouped workout cards

Build the `HistoryPage` component with a `WeeklyCalendar` at the top and a scrollable timeline below. The server function should load all workout logs for the selected week, enrich each with WOD title and exercise details, and group them by date using a `BTreeMap`.

<details>
<summary>Hints</summary>

- Use `Memo::new` to derive `(start, end)` from the calendar's `anchor` signal
- Use `Resource::new` with a dependency tuple that includes the delete action's version
- Pre-populate the `BTreeMap` with all 7 days of the week so rest days appear too
- Use `format_day_header()` to convert ISO dates to human-readable headers
- Use `into_any()` for the conditional branches (rest day vs workout day vs future day)
</details>

<details>
<summary>Solution</summary>

See the full implementation in `src/pages/history/mod.rs`. The core pattern is:

1. `compute_week_dates(&anchor.get())` derives the 7 dates for the current week
2. `get_history_for_week(start, end)` fetches and enriches all logs in that range
3. `BTreeMap` groups entries by date, with empty vectors for rest days
4. The view iterates `week_data` and renders `<HistoryCard>` for each entry

The key learning: `BTreeMap` guarantees chronological iteration without an explicit sort step.
</details>

### Exercise 2: Implement streak calculation using the greedy consecutive-day algorithm

Write the `streak_days_db` function. It should query distinct workout dates in descending order and walk them greedily, counting consecutive days from today (or yesterday if today has no workout yet).

<details>
<summary>Hints</summary>

- Query with `ORDER BY workout_date DESC` so the most recent date comes first
- Initialize `expected = today`
- Special case: if `streak == 0` and the first date is yesterday, start from yesterday
- Break as soon as you find a gap (`date < expected`)
</details>

<details>
<summary>Solution</summary>

```rust
pub async fn streak_days_db(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<i64, sqlx::Error> {
    let dates: Vec<(chrono::NaiveDate,)> = sqlx::query_as(
        r#"SELECT DISTINCT workout_date
           FROM workout_logs
           WHERE user_id = $1 AND workout_date <= CURRENT_DATE
           ORDER BY workout_date DESC"#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    if dates.is_empty() {
        return Ok(0);
    }

    let today = chrono::Local::now().date_naive();
    let mut streak = 0i64;
    let mut expected = today;

    for (date,) in dates {
        if streak == 0 && date == today - chrono::Duration::days(1) {
            expected = today - chrono::Duration::days(1);
        }
        if date == expected {
            streak += 1;
            expected -= chrono::Duration::days(1);
        } else if date < expected {
            break;
        }
    }

    Ok(streak)
}
```

The algorithm is O(n) in the number of distinct dates. The `ORDER BY DESC` from the database avoids needing to sort in Rust.
</details>

### Exercise 3: Build the leaderboard with multi-criteria sorting

Build a `LeaderboardPreview` component that shows the top 5 athletes ranked by weekly workout count. For section-level leaderboards, implement multi-criteria sorting: Rx athletes first, then higher score wins for AMRAP, lower time wins for ForTime.

<details>
<summary>Hints</summary>

- The weekly leaderboard query uses `ORDER BY workout_count DESC` --- the sorting happens in SQL
- For section leaderboards, use `ORDER BY sl.is_rx DESC, sl.score_value {ASC|DESC}`
- The `order` variable flips between `"ASC"` for ForTime and `"DESC"` for AMRAP/EMOM/Strength
- Use `format!()` to interpolate the order direction into the SQL string --- this is safe because the value is always `"ASC"` or `"DESC"`, never user input
</details>

<details>
<summary>Solution</summary>

```rust
#[component]
pub fn LeaderboardPreview(entries: Vec<LeaderboardEntry>) -> impl IntoView {
    view! {
        <div class="leaderboard-preview">
            <div class="leaderboard-header">
                <h3>"This Week"</h3>
                <span class="leaderboard-wod">"Leaderboard"</span>
            </div>
            {if entries.is_empty() {
                view! {
                    <div class="leaderboard-empty">
                        <p>"No workouts logged this week yet."</p>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div class="leaderboard-list">
                        {entries.into_iter().enumerate().map(|(i, entry)| {
                            let ini = initials(&entry.display_name);
                            let rank = i + 1;
                            let count_label = if entry.workout_count == 1 {
                                "workout"
                            } else {
                                "workouts"
                            };
                            view! {
                                <div class="leaderboard-entry">
                                    <span class="lb-rank">{rank}</span>
                                    <span class="lb-avatar">{ini}</span>
                                    <span class="lb-name">{entry.display_name}</span>
                                    <span class="lb-score">
                                        {format!("{} {}", entry.workout_count, count_label)}
                                    </span>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
```

The `initials()` helper extracts the first character of each word in the name --- used for the avatar circle when no profile image exists.
</details>

### Exercise 4: Add a stats dashboard with total workouts, day streak, and exercises count

Build the `get_dashboard` server function that aggregates total workouts, current streak, exercise count, and leaderboard data. Display them in the `HomePage` component with a stats bar and quick action cards.

<details>
<summary>Hints</summary>

- Use `unwrap_or(0)` and `unwrap_or_default()` for resilient stats --- a failed count should show 0, not crash
- The `DashboardData` struct bundles all dashboard values into a single server function response
- Use `matches!(user.role, UserRole::Admin)` to check if the current user should see test accounts on the leaderboard
- Format the date with `chrono::Local::now().date_naive().format("%A")` for the day name
</details>

<details>
<summary>Solution</summary>

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DashboardData {
    pub day_name: String,
    pub full_date: String,
    pub exercises: i64,
    pub workouts: i64,
    pub streak: i64,
    pub leaderboard: Vec<LeaderboardEntry>,
}

#[server]
async fn get_dashboard() -> Result<DashboardData, ServerFnError> {
    let user = crate::auth::session::require_auth().await?;
    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = user.id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    let exercises = crate::db::count_exercises_db(&pool).await.unwrap_or(0);
    let workouts = crate::db::count_workouts_db(&pool, user_uuid)
        .await.unwrap_or(0);
    let streak = crate::db::streak_days_db(&pool, user_uuid)
        .await.unwrap_or(0);
    let is_admin = matches!(user.role, crate::auth::UserRole::Admin);
    let leaderboard = crate::db::leaderboard_db(
        &pool, 5, user.email.as_deref().unwrap_or(""), is_admin
    ).await.unwrap_or_default();

    let today = chrono::Local::now().date_naive();
    Ok(DashboardData {
        day_name: today.format("%A").to_string(),
        full_date: today.format("%B %-d, %Y").to_string(),
        exercises, workouts, streak, leaderboard,
    })
}
```

The `HomePage` component uses `Resource::new(|| (), |_| get_dashboard())` to load the data once on mount, then renders the stats bar, quick action cards, and leaderboard preview. The single server function call avoids a waterfall of separate requests.
</details>

---

## Summary

This chapter introduced three core collection patterns:

- **`BTreeMap`** for ordered grouping --- dates iterate chronologically without explicit sorting
- **Custom `sort_by` with `Ordering` chains** --- multi-criteria ranking expressed as `.then_with()` chains
- **Greedy iteration** for streak calculation --- a single pass through sorted dates with an `expected` pointer

You also learned four foundational programming concepts: sorting (arranging items by a rule), comparators (the rule itself), algorithms (step-by-step recipes), and HashMaps (instant-lookup tables). These concepts power everything from leaderboards to analytics dashboards.

On the practical side, you saw `tokio::join!` for concurrent data enrichment, `unwrap_or` for resilient stats, and `Memo` for derived reactive state. The history page demonstrates that data fetching, transformation, and display are three separate concerns: the server function fetches and groups, the component renders, and the signals coordinate navigation and loading state.

In the next chapter, you will extract the reusable UI components --- `DeleteModal`, `SingleSelect`, `MultiSelect` --- that appear across multiple pages, and you will confront Rust's ownership and borrowing rules head-on in the context of component props and closures.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Binary Heap from scratch — the magic podium that always keeps the top K scores ready from scratch in Rust — no libraries, just std.

**→ [Heap Leaderboard](../ds-narratives/ch10-heap-leaderboard.md)**

200 athletes posting scores simultaneously. Instead of everyone polling the database, what if the server just TOLD them? This deep dive builds channels from scratch and shows the message-passing patterns that power real-time features.

**→ [Channels — "The Gym PA System"](../ds-narratives/ch10-channels-live-feed.md)**

---
