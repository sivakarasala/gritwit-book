# Chapter 10: History & Leaderboard

Your app can log workouts. Now it needs to answer: *what did I do this week? How long is my streak? Who is training the hardest?* History and leaderboards turn raw data into motivation. This chapter builds the workout history timeline (grouped by day, navigable by week), a leaderboard that ranks athletes by weekly volume, and a streak calculator that counts consecutive training days.

The spotlight concept is **collections and sorting** — the standard library tools for grouping, ordering, and aggregating data. You will use `HashMap` and `BTreeMap` for grouping logs by date, write custom sort comparators with `Ordering` chains for leaderboard ranking, and build a greedy consecutive-day algorithm for streak calculation. These are the same patterns behind every analytics dashboard and feed.

By the end of this chapter, you will have:

- A `get_history_for_week` server function that loads logs for a date range, enriches each with WOD title and exercise details, and groups them into a `BTreeMap<String, Vec<HistoryEntry>>`
- A `HistoryPage` component with a weekly calendar, day-section headers, and workout cards for each entry
- A `streak_days_db` function that calculates the current training streak using a greedy consecutive-day algorithm
- A `leaderboard_db` function that ranks athletes by weekly workout count with Rx-first, score-descending sorting
- A home page stats bar displaying total workouts, exercise count, and day streak

---

## Spotlight: Collections & Sorting Deep Dive

### HashMap vs BTreeMap

Rust's standard library offers two primary map types:

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

`HashMap` is your default choice when you do not care about key order. It uses hashing internally and provides amortized constant-time insert and lookup. `BTreeMap` uses a balanced tree and keeps keys in sorted order. Iteration over a `BTreeMap` always yields entries in key order — exactly what you want when displaying dates chronologically.

The GrindIt history page groups workout logs by date. Because dates in `YYYY-MM-DD` format sort lexicographically the same way they sort chronologically, a `BTreeMap<String, Vec<HistoryEntry>>` guarantees that when you iterate, Monday comes before Tuesday:

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

If you used a `HashMap` here, the days would appear in random order. You would need to collect into a `Vec` and sort — extra work that `BTreeMap` gives you for free.

### The .entry() API

Both map types share the `entry()` API, one of Rust's most elegant collection patterns:

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

`entry()` returns an `Entry` enum that is either `Occupied` (key exists) or `Vacant` (key does not exist). `or_default()` calls `Default::default()` on the value type if vacant — for `Vec<T>`, that is an empty vector. The result is a mutable reference to the value, so you can push directly. No double lookup, no temporary variables.

### Custom sorting with Ordering

JavaScript's `Array.sort()` takes a comparator that returns a number: negative for "a comes first", positive for "b comes first", zero for equal. Rust's `sort_by()` takes a comparator that returns `std::cmp::Ordering` — an enum with three variants: `Less`, `Equal`, `Greater`.

```rust
use std::cmp::Ordering;

// JavaScript
// scores.sort((a, b) => b.count - a.count);

// Rust
scores.sort_by(|a, b| b.count.cmp(&a.count)); // descending
```

For multi-criteria sorting, Rust uses the `then()` and `then_with()` methods on `Ordering` to chain comparisons. The leaderboard needs: Rx athletes first, then higher score beats lower score (for AMRAP), then lower time beats higher time (for ForTime):

```rust
entries.sort_by(|a, b| {
    // Rx first: true > false, so reverse for Rx-first
    b.is_rx.cmp(&a.is_rx)
        // Higher score is better (for AMRAP/EMOM)
        .then_with(|| b.score_value.cmp(&a.score_value))
        // Lower time is better (for ForTime) — only matters if scores are equal
        .then_with(|| a.finish_time_seconds.cmp(&b.finish_time_seconds))
});
```

The `then_with()` method only evaluates the closure if the previous comparison returned `Equal`. This is lazy chaining — the third criterion is never computed if the first two already determined the order.

> **Coming from JS?** JavaScript's `Array.sort((a, b) => ...)` comparator returns a number, and developers write chains like `(b.rx - a.rx) || (b.score - a.score) || (a.time - b.time)`. The `||` operator shortcuts on nonzero values, just like Rust's `.then_with()` shortcuts on non-`Equal` values. The logic is identical — Rust just makes it type-safe with the `Ordering` enum.

### Iterator chains for stats aggregation

Rust iterators provide `fold()`, `sum()`, `count()`, and `filter()` for aggregation:

```rust
// Count total workouts
let total: usize = week_data.iter()
    .map(|(_, entries)| entries.len())
    .sum();

// Count Rx workouts
let rx_count = entries.iter()
    .filter(|e| e.log.is_rx)
    .count();

// Sum total reps across all movement logs
let total_reps: i32 = movement_logs.iter()
    .filter_map(|m| m.reps)
    .sum();

// fold() for custom accumulation
let (total_weight, set_count) = sets.iter()
    .filter_map(|s| s.weight_kg)
    .fold((0.0_f32, 0_u32), |(sum, count), w| (sum + w, count + 1));
let avg_weight = if set_count > 0 { total_weight / set_count as f32 } else { 0.0 };
```

`filter_map()` is especially useful for `Option<T>` fields — it filters out `None` values and unwraps `Some` in one step.

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

This is a *view model* — a struct shaped for the UI, not for the database. The database stores `workout_logs`, `workout_exercises`, `section_logs`, and `movement_logs` in separate tables. The server function assembles them into a single `HistoryEntry` per log.

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

Three collection patterns appear here:

1. **`Vec::with_capacity(logs.len())`** — pre-allocates the exact size. When you know the count upfront, this avoids reallocations.
2. **`BTreeMap` with pre-populated keys** — every day of the week gets an entry, even rest days. This makes the UI simpler: it can iterate without checking for missing dates.
3. **`.into_iter().collect()`** — consumes the `BTreeMap` into a `Vec<(String, Vec<HistoryEntry>)>`. The return type cannot be a `BTreeMap` directly because Leptos server functions require types that implement `Serialize`/`Deserialize`, and while `BTreeMap` does implement both, the `Vec<(K, V)>` form is more explicit about the ordered-pair structure.

### Concurrent enrichment with tokio::join!

For WOD logs, three queries are needed: section scores, movement logs, and movement log sets. These are independent of each other, so `tokio::join!` runs them concurrently:

```rust
let (scores, movements, sets) = tokio::join!(
    crate::db::get_section_scores_with_meta_db(&pool, log_uuid),
    crate::db::get_movement_logs_with_names_db(&pool, log_uuid),
    crate::db::get_movement_log_sets_db(&pool, log_uuid),
);
```

Without `tokio::join!`, three sequential `await`s would take the sum of all three query times. With it, they overlap on the event loop and finish in the time of the slowest query. This is the async equivalent of `Promise.all()` in JavaScript.

---

## Streak Calculation: A Greedy Algorithm

The streak counter answers: "how many consecutive days (ending today or yesterday) has this user logged a workout?" This is the **longest consecutive sequence from the end** problem — a variation of LeetCode 128 (Longest Consecutive Sequence).

### The algorithm

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
            break; // gap found — streak ends
        }
    }

    Ok(streak)
}
```

The key insight: dates come back sorted descending. Walk them from most recent to least recent, maintaining an `expected` date. If the current date matches the expected date, increment the streak and move expected back by one day. If the date is before the expected date, there is a gap and the streak ends.

The initial `expected` is today. But if today has no workout yet and yesterday does, the streak should start from yesterday — hence the special case at `streak == 0`.

This is a **greedy** algorithm: at each step, we greedily extend the streak by one day if possible. No backtracking, no dynamic programming, no hash set. Time complexity is O(n) where n is the number of distinct workout dates, plus the cost of the database query.

### DSA connection: Longest Consecutive Sequence

LeetCode 128 asks: given an unsorted array of integers, find the length of the longest consecutive sequence. The classic O(n) solution uses a `HashSet`:

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

Our streak calculation is simpler because we only care about the sequence anchored at today (or yesterday). But the pattern — checking consecutive values greedily — is the same.

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

The SQL handles the ranking: `ORDER BY workout_count DESC`. The WHERE clause hides test accounts from regular users but shows them to admins — a common production concern.

### Section leaderboard: multi-criteria sorting

The section-level leaderboard ranks athletes by their best score on a specific WOD section. The sorting is more complex: Rx athletes always rank above scaled athletes, and the score direction depends on the workout type:

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

The `ORDER BY sl.is_rx DESC, sl.score_value {order}` clause implements the two-tier ranking in SQL: Rx first (DESC sorts `true` before `false` in PostgreSQL), then by score in the appropriate direction.

If you needed to do this sorting in Rust instead of SQL (for example, merging results from multiple queries):

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

### System Design: Leaderboard at Scale

A workout app with 100 users can compute leaderboards with a SQL query. What about 100,000 users?

**Redis sorted sets** are the industry standard for real-time leaderboards. A sorted set maps members to scores, and Redis maintains them in sorted order with O(log n) insert and O(log n) rank lookup:

```
ZADD weekly:2026-w12 1 "user:alice"   # alice logged 1 workout
ZINCRBY weekly:2026-w12 1 "user:alice" # alice logged another
ZREVRANK weekly:2026-w12 "user:alice"  # get rank (0-indexed)
ZREVRANGE weekly:2026-w12 0 9 WITHSCORES  # top 10
```

At GrindIt's scale (single gym, dozens of athletes), PostgreSQL is more than sufficient. But the pattern is worth knowing: when reads vastly outnumber writes and you need sub-millisecond rank lookups, move the leaderboard to a purpose-built data structure.

---

## Building the History Page

### The HistoryPage component

The history page combines a weekly calendar (built in Chapter 8) with a scrollable timeline of workout cards:

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

Key reactive patterns:

1. **`Memo` for derived state.** `week_range` derives start/end dates from the `anchor` signal. When the user swipes the calendar to a new week, `anchor` changes, which triggers `week_range` to recompute, which triggers `history` to refetch.

2. **`Resource::new` with a composite dependency tuple.** The first closure returns `(start, end, version)`. The Resource refetches whenever any element of this tuple changes. Including `delete_action.version()` means the history also refetches after a deletion.

3. **Loading state managed separately.** The `is_loading` signal is set to `true` before the fetch and `false` after. This drives a thin loading bar at the top of the page — subtle feedback without a full-screen spinner.

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

The `format_day_header` function converts `"2026-03-15"` to `"SUNDAY, MAR 15"` using integer date arithmetic (no chrono on the client). The `into_any()` calls resolve the type mismatch between the if/else branches — a pattern we explored in Chapter 6.

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

The `#[cfg(feature = "hydrate")]` guard ensures this JavaScript only runs in the browser, not during SSR. The `setTimeout` with 50ms delay gives the DOM time to render the new week's data before scrolling.

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

Notice the `unwrap_or(0)` and `unwrap_or_default()` calls. Dashboard stats should never crash the page — if a count query fails, showing 0 is better than an error. This is a common resilience pattern: let non-critical data degrade gracefully.

The stats bar component renders three cards:

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
        b.2.cmp(&a.2)                    // Rx first
            .then_with(|| b.1.cmp(&a.1)) // higher score
            .then_with(|| a.0.cmp(&b.0)) // alphabetical name
    });
}
```

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
    // Alice before Carol because "Alice" < "Carol" alphabetically
    assert_eq!(athletes[0].0, "Dave");
    assert_eq!(athletes[1].0, "Alice");
    assert_eq!(athletes[2].0, "Carol");
    assert_eq!(athletes[3].0, "Bob");
    println!("All assertions passed!");
}
```
</details>

### Group-by pattern

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

<details>
<summary>Exercise: implement group_by_first_letter</summary>

Group a list of exercise names by their first letter, uppercased.

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
    assert_eq!(grouped[&'B'].len(), 2);
    assert_eq!(grouped[&'D'].len(), 2);
    println!("{:?}", grouped);
}
```
</details>

### Consecutive sequence detection

<details>
<summary>Exercise: find the longest training streak in a list of dates</summary>

```rust
fn longest_streak(mut dates: Vec<i32>) -> i32 {
    if dates.is_empty() { return 0; }
    dates.sort();
    dates.dedup();

    let mut best = 1;
    let mut current = 1;

    for window in dates.windows(2) {
        if window[1] == window[0] + 1 {
            current += 1;
            best = best.max(current);
        } else {
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

- The weekly leaderboard query uses `ORDER BY workout_count DESC` — the sorting happens in SQL
- For section leaderboards, use `ORDER BY sl.is_rx DESC, sl.score_value {ASC|DESC}`
- The `order` variable flips between `"ASC"` for ForTime and `"DESC"` for AMRAP/EMOM/Strength
- Use `format!()` to interpolate the order direction into the SQL string — this is safe because the value is always `"ASC"` or `"DESC"`, never user input
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

The `initials()` helper extracts the first character of each word in the name — used for the avatar circle when no profile image exists.
</details>

### Exercise 4: Add a stats dashboard with total workouts, day streak, and exercises count

Build the `get_dashboard` server function that aggregates total workouts, current streak, exercise count, and leaderboard data. Display them in the `HomePage` component with a stats bar and quick action cards.

<details>
<summary>Hints</summary>

- Use `unwrap_or(0)` and `unwrap_or_default()` for resilient stats — a failed count should show 0, not crash
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

- **`BTreeMap`** for ordered grouping — dates iterate chronologically without explicit sorting
- **Custom `sort_by` with `Ordering` chains** — multi-criteria ranking expressed as `.then_with()` chains
- **Greedy iteration** for streak calculation — a single pass through sorted dates with an `expected` pointer

You also saw `tokio::join!` for concurrent data enrichment, `unwrap_or` for resilient stats, and `Memo` for derived reactive state. The history page demonstrates that data fetching, transformation, and display are three separate concerns: the server function fetches and groups, the component renders, and the signals coordinate navigation and loading state.

In the next chapter, you will extract the reusable UI components — `DeleteModal`, `SingleSelect`, `MultiSelect` — that appear across multiple pages, and you will confront Rust's ownership and borrowing rules head-on in the context of component props and closures.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a Binary Heap from scratch — the magic podium that always keeps the top K scores ready.

**→ [Heap Leaderboard](../ds-narratives/ch10-heap-leaderboard.md)**

200 athletes posting scores simultaneously. Instead of everyone polling the database, what if the server just TOLD them? This deep dive builds channels from scratch and shows the message-passing patterns that power real-time features.

**→ [Channels — "The Gym PA System"](../ds-narratives/ch10-channels-live-feed.md)**

---
