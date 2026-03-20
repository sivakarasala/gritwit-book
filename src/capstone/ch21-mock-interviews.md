# Chapter 21: Mock Interviews

You have built GrindIt from scratch. You understand Rust's ownership model, async patterns, and trait-based abstractions. You have solved DSA problems using fitness domain data and designed systems at scale. Now it is time to put it all together under pressure.

This chapter contains two complete mock interviews: a 45-minute coding interview and a 45-minute system design interview. Each one is written as a realistic simulation, complete with interviewer dialogue, candidate responses, and commentary on what works and what does not. Read them actively. Cover the candidate's response, try to answer yourself, then compare.

These are not abstract exercises. Every problem uses the GrindIt domain you have been building throughout this book.

---

## Pre-Interview Checklist

Before you simulate either interview, prepare the way you would for a real one.

### Environment Setup
- [ ] A quiet space with a whiteboard, paper, or tablet for diagrams
- [ ] A code editor or shared doc open (no autocomplete — most interviews disable it)
- [ ] A timer visible on your desk
- [ ] Water within reach

### Knowledge Review
- [ ] Review the DSA patterns from Chapter 19 — know when each applies
- [ ] Review the system design framework from Chapter 20 — requirements, capacity, HLD, deep dives
- [ ] Skim your GrindIt codebase — be ready to reference real decisions you made
- [ ] Practice explaining your approach out loud before writing code

### Mental Preparation
- [ ] Accept that you will not produce perfect code under time pressure
- [ ] Plan to spend at least 30% of your time talking before coding
- [ ] Remind yourself: interviewers evaluate your *process*, not just your output
- [ ] If you get stuck, say so. Silence is worse than "I am considering two approaches here."

---

## Mock Interview 1: Coding (45 Minutes)

**Setting:** You join a video call. The interviewer introduces herself as Priya, a senior engineer. She shares a collaborative code editor.

> **Priya:** "Thanks for joining. We will work through two problems today. I care more about how you think through problems than whether you get a perfect solution. Talk me through your reasoning as you go. Ready?"

> **You:** "Ready."

---

### Problem 1: Maximum Strength Gain (20 minutes)

> **Priya:** "Here is the first problem. At GrindIt, athletes log their one-rep max for exercises over time. Given a list of workout sessions in chronological order, where each session has a weight lifted, find the maximum weight increase between any earlier session and any later session. The increase must go forward in time — you cannot compare a later session to an earlier one."

She types in the editor:

```
Input: sessions = [135, 150, 120, 185, 140, 200, 155]
Output: 80

Explanation: The max increase is from session[2]=120 to session[5]=200, a gain of 80.
```

#### Step 1: Clarifying Questions

> **You:** "A few clarifying questions. First, are the sessions guaranteed to be in chronological order already?"

> **Priya:** "Yes, the input is sorted by date."

> **You:** "Can the list be empty or have just one session?"

> **Priya:** "Good edge case thinking. If there is zero or one session, return 0 — no gain is possible."

> **You:** "And the weights are always positive integers?"

> **Priya:** "Yes."

> **You:** "Can the weights only decrease? Like `[200, 180, 150]`?"

> **Priya:** "Yes, and in that case you should return 0."

**Commentary:** These questions cost you 60 seconds and saved you from writing code that crashes on edge cases. Every interviewer notices when you ask about boundaries.

#### Step 2: Brute Force

> **You:** "My first thought is the brute force approach. For every pair `(i, j)` where `i < j`, compute `sessions[j] - sessions[i]` and track the maximum. That is O(n^2) time, O(1) space."

```rust
fn max_strength_gain_brute(sessions: &[i32]) -> i32 {
    let n = sessions.len();
    let mut max_gain = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            let gain = sessions[j] - sessions[i];
            if gain > max_gain {
                max_gain = gain;
            }
        }
    }

    max_gain
}
```

> **Priya:** "That works. Can you do better?"

#### Step 3: Optimized Solution

> **You:** "Yes. The key insight is that to maximize the gain ending at position `j`, I want to subtract the smallest value seen so far — the minimum from indices `0` through `j-1`. I can track that running minimum as I scan left to right. One pass, O(n) time, O(1) space."

> **Priya:** "Walk me through why that works."

> **You:** "At each position `j`, the best possible gain is `sessions[j] - min_so_far`. If I update `min_so_far` after computing the gain at each step, I guarantee the minimum comes from an earlier index. The overall answer is the maximum of all these per-position gains."

```rust
fn max_strength_gain(sessions: &[i32]) -> i32 {
    if sessions.len() < 2 {
        return 0;
    }

    let mut min_so_far = sessions[0];
    let mut max_gain = 0;

    for &weight in &sessions[1..] {
        let gain = weight - min_so_far;
        if gain > max_gain {
            max_gain = gain;
        }
        if weight < min_so_far {
            min_so_far = weight;
        }
    }

    max_gain
}
```

> **Priya:** "Nice. Walk me through your test cases."

#### Step 4: Test Cases

> **You:** "Let me trace through the original example and then cover edge cases."

```
sessions = [135, 150, 120, 185, 140, 200, 155]

Step 0: min_so_far=135, max_gain=0
Step 1: weight=150, gain=150-135=15,  max_gain=15,  min_so_far=135
Step 2: weight=120, gain=120-135=-15, max_gain=15,  min_so_far=120
Step 3: weight=185, gain=185-120=65,  max_gain=65,  min_so_far=120
Step 4: weight=140, gain=140-120=20,  max_gain=65,  min_so_far=120
Step 5: weight=200, gain=200-120=80,  max_gain=80,  min_so_far=120
Step 6: weight=155, gain=155-120=35,  max_gain=80,  min_so_far=120

Output: 80  ✓
```

> **You:** "Edge cases:"

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_case() {
        assert_eq!(max_strength_gain(&[135, 150, 120, 185, 140, 200, 155]), 80);
    }

    #[test]
    fn empty_input() {
        assert_eq!(max_strength_gain(&[]), 0);
    }

    #[test]
    fn single_session() {
        assert_eq!(max_strength_gain(&[135]), 0);
    }

    #[test]
    fn only_decreasing() {
        assert_eq!(max_strength_gain(&[200, 180, 150, 100]), 0);
    }

    #[test]
    fn gain_at_the_end() {
        assert_eq!(max_strength_gain(&[100, 90, 80, 300]), 220);
    }

    #[test]
    fn all_same() {
        assert_eq!(max_strength_gain(&[135, 135, 135]), 0);
    }
}
```

> **Priya:** "Good coverage. One follow-up."

#### Step 5: Follow-Up

> **Priya:** "What if I also need the *dates* of the two sessions that produced the maximum gain? And what if there are ties — multiple pairs that produce the same gain?"

> **You:** "I would track `min_index` alongside `min_so_far`, and whenever I update `max_gain`, I would record both `min_index` and the current index `j` as the answer pair. For ties, I would need a policy — earliest pair, latest pair, or all pairs. Earliest pair comes for free: the first time I find a gain equal to the current max, I just do not overwrite. For all pairs, I would collect them into a `Vec`."

> **Priya:** "Great. Let us move on to the second problem."

**Commentary:** This problem is a variant of "Best Time to Buy and Sell Stock" (LeetCode 121). The candidate recognized the pattern, explained the insight clearly, wrote clean code, tested thoroughly, and handled the follow-up with a concrete plan rather than hand-waving.

---

### Problem 2: Optimal Workout Schedule (25 minutes)

> **Priya:** "GrindIt coaches want to plan a week of workouts. Each workout has a duration, an equipment set it requires, and a recovery time — the athlete cannot do another workout using the same muscle group until the recovery period has passed. Given a list of available workouts, a set of available equipment, and a 7-day week, find the schedule that maximizes total training volume while respecting equipment availability and recovery constraints."

She types:

```
struct Workout {
    name: String,
    duration_min: u32,       // minutes
    muscle_group: String,    // e.g., "legs", "upper", "cardio"
    equipment: Vec<String>,  // e.g., ["barbell", "rack"]
    volume: u32,             // arbitrary score
    recovery_days: u32,      // days before same muscle group again
}

Input:
  workouts: list of available workouts
  equipment: set of equipment in the gym
  days: 7 (one workout slot per day)

Output:
  schedule: [Option<&Workout>; 7]  — which workout (if any) on each day
  total_volume: u32

Constraints:
  - At most one workout per day
  - Workout's equipment must be a subset of available equipment
  - If a workout uses muscle group M with recovery_days=R,
    no other workout with group M can appear within R days after it
```

#### Step 1: Clarifying Questions

> **You:** "Several questions. Is it one workout per day maximum, or could we fit multiple?"

> **Priya:** "One per day maximum. Keep it simple."

> **You:** "Can we repeat the same workout on different days?"

> **Priya:** "Yes, as long as recovery constraints are satisfied."

> **You:** "Is the number of available workouts small — say under 20 — or could it be hundreds?"

> **Priya:** "Assume up to 20 workouts. The days are always 7."

> **You:** "And we want to maximize total volume across the week?"

> **Priya:** "Exactly."

**Commentary:** The constraint question is critical. With 20 workouts and 7 days, this is tractable with backtracking. With 1000 workouts, you would need a different approach.

#### Step 2: Approach Discussion

> **You:** "This is a constraint satisfaction problem with optimization. Let me think about the approaches.
>
> The brute force is to try every possible assignment of workouts to days. With `n` workouts and 7 days, each day can be one of `n+1` choices (any workout or rest day), giving `(n+1)^7` combinations. With `n=20`, that is `21^7 ≈ 1.8 billion`. Too many.
>
> But most of those are invalid due to recovery constraints. I can use **backtracking with pruning**: build the schedule day by day, and for each day, only consider workouts whose equipment is available and whose muscle group is not in recovery. This prunes the search tree dramatically.
>
> For further optimization, I can sort workouts by volume descending so we explore high-value options first, and prune any branch where the remaining days times the maximum single-workout volume cannot beat our current best."

> **Priya:** "I like the pruning idea. Go ahead and code it."

#### Step 3: Implementation

```rust
use std::collections::HashSet;

struct Workout {
    name: String,
    duration_min: u32,
    muscle_group: String,
    equipment: Vec<String>,
    volume: u32,
    recovery_days: u32,
}

struct ScheduleResult {
    schedule: [Option<usize>; 7], // index into workouts vec
    total_volume: u32,
}

fn optimal_schedule(
    workouts: &[Workout],
    available_equipment: &HashSet<String>,
) -> ScheduleResult {
    // Pre-filter: only workouts whose equipment is available
    let valid: Vec<usize> = workouts
        .iter()
        .enumerate()
        .filter(|(_, w)| w.equipment.iter().all(|e| available_equipment.contains(e)))
        .map(|(i, _)| i)
        .collect();

    let max_single_volume = valid
        .iter()
        .map(|&i| workouts[i].volume)
        .max()
        .unwrap_or(0);

    let mut best = ScheduleResult {
        schedule: [None; 7],
        total_volume: 0,
    };
    let mut current_schedule = [None; 7];

    backtrack(
        workouts,
        &valid,
        0,               // current day
        0,               // current volume
        &mut current_schedule,
        &mut best,
        max_single_volume,
    );

    best
}

fn backtrack(
    workouts: &[Workout],
    valid: &[usize],
    day: usize,
    current_volume: u32,
    current_schedule: &mut [Option<usize>; 7],
    best: &mut ScheduleResult,
    max_single_volume: u32,
) {
    if day == 7 {
        if current_volume > best.total_volume {
            best.total_volume = current_volume;
            best.schedule = *current_schedule;
        }
        return;
    }

    // Pruning: even if every remaining day has max volume, can we beat best?
    let remaining_days = (7 - day) as u32;
    if current_volume + remaining_days * max_single_volume <= best.total_volume {
        return;
    }

    // Option A: rest day (no workout)
    current_schedule[day] = None;
    backtrack(
        workouts, valid, day + 1, current_volume,
        current_schedule, best, max_single_volume,
    );

    // Option B: try each valid workout
    for &wi in valid {
        if is_recovery_ok(workouts, current_schedule, day, &workouts[wi].muscle_group, workouts[wi].recovery_days) {
            current_schedule[day] = Some(wi);
            backtrack(
                workouts, valid, day + 1,
                current_volume + workouts[wi].volume,
                current_schedule, best, max_single_volume,
            );
            current_schedule[day] = None; // backtrack
        }
    }
}

fn is_recovery_ok(
    workouts: &[Workout],
    schedule: &[Option<usize>; 7],
    current_day: usize,
    muscle_group: &str,
    recovery_days: u32,
) -> bool {
    // Check previous days for same muscle group within recovery window
    let look_back = recovery_days as usize;
    let start = if current_day >= look_back {
        current_day - look_back
    } else {
        0
    };

    for d in start..current_day {
        if let Some(wi) = schedule[d] {
            if workouts[wi].muscle_group == muscle_group {
                // Check if enough days have passed
                let days_apart = current_day - d;
                if days_apart <= workouts[wi].recovery_days as usize {
                    return false;
                }
            }
        }
    }

    true
}
```

> **Priya:** "Talk me through the complexity."

> **You:** "Worst case is still exponential — `O(n^7)` where `n` is the number of valid workouts. But in practice, the recovery constraints and the volume-based pruning cut this down dramatically. With 20 workouts and realistic recovery times of 1-2 days, most branches are pruned early. For a 7-day schedule, this runs in well under a second. If we needed to scale to 30-day planning horizons, I would switch to dynamic programming with bitmask states for muscle group recovery status."

#### Step 4: Test Case

> **You:** "Let me trace through a small example."

```
Workouts:
  0: Back Squat, legs, volume=10, recovery=2 days
  1: Bench Press, upper, volume=8, recovery=1 day
  2: Running, cardio, volume=5, recovery=0 days

Equipment: all available.

Expected optimal:
  Day 0: Back Squat (legs, vol=10)
  Day 1: Bench Press (upper, vol=8)
  Day 2: Running (cardio, vol=5)
  Day 3: Back Squat (legs, vol=10)  -- 3 days after day 0, recovery=2 OK
  Day 4: Bench Press (upper, vol=8) -- 3 days after day 1, recovery=1 OK
  Day 5: Running (cardio, vol=5)
  Day 6: Back Squat (legs, vol=10)  -- 3 days after day 3, recovery=2 OK

Total volume: 10 + 8 + 5 + 10 + 8 + 5 + 10 = 56
```

> **Priya:** "What if an athlete has a constraint you have not modeled — say, they cannot work out on Mondays?"

> **You:** "I would add a `blocked_days: HashSet<usize>` parameter. At the top of the day loop, if the day is blocked, skip straight to the next day with no workout. It does not change the structure of the algorithm."

> **Priya:** "Good. That wraps up the coding portion. Nice job communicating throughout."

**Commentary:** The candidate did not try to jump straight to code. They discussed the brute force complexity, identified why backtracking with pruning was the right tool, and explained the optimization. The recovery check function was separated cleanly rather than inlined into the backtracking loop — this is the kind of code structure interviewers notice. The test case was concrete, not hand-wavy.

---

## Mock Interview 2: System Design (45 Minutes)

**Setting:** A different call, this time with Marcus, a staff engineer. He opens a shared whiteboard.

> **Marcus:** "Today I would like you to design GrindIt — a fitness tracking application — for one million users. Take this wherever you think is interesting, but I want to see that you can reason about scale. Let us start with requirements."

---

### Phase 1: Requirements (5 minutes)

> **You:** "Let me start by confirming the core functional requirements, then define the non-functional ones."

> **You:** "For functional requirements, I see these as the core features:
> 1. **Workout programming** — coaches create WODs (Workout of the Day) with sections, movements, and prescribed weights
> 2. **Score logging** — athletes log their results against WODs (time, rounds, weight lifted)
> 3. **Leaderboard** — ranked scores per WOD, filterable by gym, gender, age
> 4. **Exercise library** — searchable catalog of movements with categories
> 5. **History** — personal workout history with timeline view and PR tracking
> 6. **Multi-tenancy** — each gym (box) has its own programming and members
>
> Which of these should I prioritize in the design?"

> **Marcus:** "Focus on workout logging, leaderboards, and multi-tenancy. Those are the hardest at scale."

> **You:** "Great. For non-functional requirements:
> - **1 million registered users**, with maybe 100K daily active
> - **Low-latency reads** — leaderboard and history should load under 200ms
> - **High write throughput** — score logging spikes around 5-7 PM local time across time zones
> - **Availability over consistency** — it is acceptable if a leaderboard is a few seconds stale, but logging a workout should never fail
> - **Multi-region** — users across North America, Europe, and Australia"

> **Marcus:** "Good. Those are reasonable assumptions. Move to capacity."

---

### Phase 2: Capacity Estimation (5 minutes)

> **You:** "Let me work through the numbers."

```
Users:
  1M registered, 100K DAU
  Average user logs 1 workout/day = 100K writes/day
  100K writes / 86,400 seconds ≈ 1.2 writes/sec average
  Peak (5-7 PM, distributed across time zones): ~5x average ≈ 6 writes/sec

  That is not a lot of write QPS. The real challenge is reads.

Reads:
  Each user checks leaderboard ~3x/day, history ~2x/day = 500K reads/day
  500K / 86,400 ≈ 6 reads/sec average
  Peak: ~30 reads/sec

  Still modest. But leaderboard queries are expensive — they involve
  sorting across all scores for a WOD, possibly filtered by gym.

Storage:
  Each workout log: ~500 bytes (scores, metadata, timestamps)
  100K logs/day × 500 bytes = 50 MB/day
  Per year: ~18 GB of workout data
  With indexes and overhead: ~50 GB/year

  Exercise library: negligible (<1 MB)
  User profiles: 1M × 1 KB = 1 GB

  Video uploads (if supported): this changes everything —
  but I will keep it out of scope unless you want me to include it.
```

> **Marcus:** "Keep video out of scope. Your numbers look reasonable. What surprised you?"

> **You:** "Honestly, the QPS is low for 1M users. The bottleneck is not raw throughput — it is query complexity. A leaderboard query that scans 100K scores and sorts them is expensive even at 30 QPS. That tells me caching is going to be the key architectural decision."

> **Marcus:** "Good instinct. Let us see the high-level design."

**Commentary:** Notice the candidate did arithmetic *out loud* and drew a conclusion from the numbers rather than just computing them mechanically. The insight about leaderboard query complexity is what separates adequate from strong candidates.

---

### Phase 3: High-Level Design (10 minutes)

> **You:** "Here is the component diagram."

```
                    ┌──────────────┐
                    │   CDN/Edge   │
                    │  (Cloudflare)│
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │ Load Balancer │
                    │   (L7/ALB)   │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────▼────┐ ┌────▼─────┐ ┌────▼─────┐
        │ App Node │ │ App Node │ │ App Node │
        │ (Leptos/ │ │  (same)  │ │  (same)  │
        │  Axum)   │ │          │ │          │
        └─────┬────┘ └────┬─────┘ └────┬─────┘
              │            │            │
              └────────────┼────────────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────▼────┐ ┌────▼─────┐ ┌────▼─────┐
        │  Redis   │ │ Postgres │ │  Queue   │
        │ (cache + │ │ (primary │ │ (async   │
        │  leaderb)│ │  store)  │ │  jobs)   │
        └──────────┘ └──────────┘ └──────────┘
```

> **You:** "The core flow for logging a workout:
> 1. Athlete submits score via the Leptos frontend
> 2. Request hits any app node (stateless — session stored in Redis)
> 3. App node validates the input, writes to PostgreSQL
> 4. App node pushes an event to the job queue: 'recalculate leaderboard for WOD X'
> 5. A background worker picks up the event, recomputes the leaderboard, writes it to Redis as a sorted set
> 6. Next leaderboard read serves directly from Redis — no database query needed"

> **Marcus:** "Why not compute the leaderboard on every read?"

> **You:** "At our scale, we *could* — 30 QPS against a well-indexed Postgres table is fine. But leaderboard queries are deceptively complex: they involve filtering by gym, gender, and score type (Rx vs Scaled), then ranking. As we grow, each of those filters multiplies the cache space or the query complexity. Pre-computing into Redis sorted sets means reads are O(log n) regardless of filter combinations."

> **Marcus:** "Fair. But you have introduced eventual consistency. How stale can the leaderboard be?"

> **You:** "I would set the worker to process events within 5 seconds. For a fitness app, seeing your score appear on the leaderboard 5 seconds after submission is perfectly acceptable. If a user just submitted, I can also optimistically insert their score into the client-side view so they see it immediately, even before the server-side recalculation."

> **Marcus:** "Good. What about the API layer?"

> **You:** "Two entry points into the same backend:
> 1. **Leptos server functions** — used by the web frontend, full SSR with hydration
> 2. **REST API** (`/api/v1/`)  — used by potential mobile apps or third-party integrations
>
> Both call the same database functions — zero business logic duplication. This is a pattern I implemented in the actual GrindIt codebase. The server functions and REST handlers are thin wrappers around shared `_db()` functions."

---

### Phase 4: Deep Dives (20 minutes)

> **Marcus:** "Let us go deeper on three areas. First: multi-tenancy."

#### Deep Dive 1: Multi-Tenant Architecture

> **You:** "Multi-tenancy for gyms. The key question is the isolation model. There are three common approaches:"

```
1. Separate database per gym
   + Complete isolation, easy compliance
   - Operational nightmare at 1000+ gyms, connection pool explosion

2. Separate schema per gym
   + Good isolation, shared infrastructure
   - Migration complexity, still many connections

3. Shared tables with tenant_id column
   + Simple operations, easy to query across gyms
   - Must enforce isolation at app layer, risk of data leaks
```

> **You:** "For GrindIt, I would use option 3 — shared tables with a `gym_id` column on every tenant-scoped table. At 1M users across maybe 2,000 gyms, the data volume does not justify database-per-tenant overhead.
>
> To enforce isolation, I would use PostgreSQL Row-Level Security (RLS):"

```sql
ALTER TABLE workout_logs ENABLE ROW LEVEL SECURITY;

CREATE POLICY gym_isolation ON workout_logs
  USING (gym_id = current_setting('app.current_gym_id')::uuid);
```

> **You:** "Every database connection sets `app.current_gym_id` at the start of the transaction. Even if application code has a bug and omits a `WHERE gym_id = ...` clause, RLS prevents cross-tenant data access. This is defense in depth."

> **Marcus:** "What about cross-gym features? Like a global leaderboard?"

> **You:** "For cross-gym queries, I would use a service-level database role that bypasses RLS. This role is only used by specific background workers — never by user-facing request handlers. The global leaderboard worker queries all gyms, aggregates scores, and writes the result to a separate `global_leaderboard` Redis key. User-facing code never queries across tenants directly."

> **Marcus:** "Good separation. Next deep dive: the workout logging write path."

#### Deep Dive 2: Workout Logging Write Path

> **You:** "The write path needs to be reliable above all else. An athlete finishes a brutal workout, opens their phone with shaky hands, logs their score. If that write fails, they lose data they cannot recreate. Trust is destroyed."

> **Marcus:** "So how do you make it reliable?"

> **You:** "Three layers of protection:"

```
Layer 1: Client-side persistence
  - Before submitting, save the score to IndexedDB (PWA offline storage)
  - If the network request fails, the score is not lost
  - A sync worker retries on reconnection

Layer 2: Idempotent writes
  - Client generates a UUID for each score submission
  - Server uses this as an idempotency key
  - If the request is retried (network timeout, user double-taps),
    the server detects the duplicate and returns success without writing again

Layer 3: Write-ahead to a durable queue
  - App node validates input, then writes to both:
    a) PostgreSQL (primary store)
    b) An event to the queue (for leaderboard recalculation)
  - If Postgres is down, write ONLY to the queue with a "pending_persist" flag
  - A recovery worker drains pending writes when Postgres recovers
```

> **Marcus:** "Layer 3 is interesting. Are you not worried about the queue also being down?"

> **You:** "Good pushback. If both Postgres and the queue are down, we fall back to Layer 1 — the client holds the score locally and retries. The probability of all three failing simultaneously is very low, and for a fitness app, a delay of minutes is acceptable. We are not processing payments."

> **Marcus:** "What about the database schema for workout logs? Walk me through it."

> **You:** "The schema is denormalized for write simplicity but normalized enough to avoid update anomalies:"

```sql
-- Core log entry
CREATE TABLE workout_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    gym_id UUID NOT NULL REFERENCES gyms(id),
    wod_id UUID REFERENCES wods(id),          -- nullable for ad-hoc workouts
    workout_date DATE NOT NULL,
    notes TEXT,
    rx BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    idempotency_key UUID UNIQUE               -- for duplicate detection
);

-- Per-section scores (a WOD can have multiple sections)
CREATE TABLE workout_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    log_id UUID NOT NULL REFERENCES workout_logs(id) ON DELETE CASCADE,
    section_index INT NOT NULL,
    score_type TEXT NOT NULL,  -- 'time', 'rounds_reps', 'weight', 'distance'
    score_value INT NOT NULL,  -- seconds, total_reps, pounds, meters
    UNIQUE(log_id, section_index)
);

-- Indexes for common queries
CREATE INDEX idx_logs_user_date ON workout_logs(user_id, workout_date DESC);
CREATE INDEX idx_logs_wod_gym ON workout_logs(wod_id, gym_id);
CREATE INDEX idx_logs_gym_date ON workout_logs(gym_id, workout_date DESC);
```

> **You:** "The `score_value` is always an integer. For time-based scores, we store seconds. For rounds+reps (AMRAP), we store total reps (`rounds * movements_per_round + extra_reps`). This makes sorting straightforward — it is always a numeric comparison, just the direction changes (lower is better for time, higher for AMRAP)."

> **Marcus:** "Smart. Last deep dive: real-time leaderboard."

#### Deep Dive 3: Real-Time Leaderboard

> **You:** "Let me walk through the evolution of the leaderboard as we scale."

```
Stage 1 (MVP, <10K users):
  Direct SQL query on every read.

  SELECT u.display_name, ws.score_value, wl.rx
  FROM workout_logs wl
  JOIN workout_scores ws ON ws.log_id = wl.id
  JOIN users u ON u.id = wl.user_id
  WHERE wl.wod_id = $1 AND wl.gym_id = $2
  ORDER BY wl.rx DESC, ws.score_value ASC  -- Rx first, then by time
  LIMIT 50;

  Works fine. Maybe 50ms per query.

Stage 2 (10K-100K users):
  Cache the query result in Redis with a TTL.
  Key: leaderboard:{wod_id}:{gym_id}:{gender}
  Value: JSON array of top 50
  TTL: 30 seconds

  Reads: O(1) from Redis
  Writes: cache invalidation on new score + background recalculation

Stage 3 (100K+ users, real-time feel):
  Redis Sorted Sets.
  Key: lb:{wod_id}:{gym_id}
  Members: user_id
  Score: composite value encoding (rx_flag * 10^9 + score_value)

  On new score: ZADD lb:{wod_id}:{gym_id} {composite_score} {user_id}
  On read: ZRANGEBYSCORE with LIMIT for pagination

  O(log n) writes, O(log n + k) reads where k is page size.
  No background worker needed — the sorted set IS the leaderboard.
```

> **Marcus:** "How do you handle the Rx vs Scaled distinction in the sorted set?"

> **You:** "I encode it in the score. Rx scores get a large offset added — say, Rx athletes get `score = 1_000_000_000 - time_seconds` while Scaled athletes get `score = time_seconds`. Since Redis sorted sets order by score, Rx athletes always rank above Scaled, and within Rx, lower times rank higher because the subtraction inverts the order. This avoids maintaining two separate sorted sets."

> **Marcus:** "Clever. What about cross-gym leaderboards?"

> **You:** "A separate sorted set keyed by just `lb:{wod_id}:global`. Every score write updates both the gym-specific and global sorted sets. Two `ZADD` commands, pipelined in a single Redis round trip."

> **Marcus:** "What happens if Redis goes down?"

> **You:** "The leaderboard degrades to Stage 1 — direct SQL queries. The application layer has a fallback: try Redis first, on connection error, query Postgres. It is slower but correct. When Redis recovers, a background job rebuilds the sorted sets from the database. No data is lost because Redis is a cache layer here, not the source of truth."

> **Marcus:** "Good. That is a solid degradation strategy."

---

### Phase 5: Wrap-Up (5 minutes)

> **Marcus:** "Let us wrap up. Summarize the key tradeoffs you made."

> **You:** "Three main tradeoffs:"

```
1. Shared-table multi-tenancy over database-per-tenant
   + Simpler operations, cheaper, cross-gym features are natural
   - Requires disciplined RLS enforcement, harder compliance story

   Right for us because: gym data is not sensitive enough to warrant
   physical isolation, and we want features like global leaderboards.

2. Eventually-consistent leaderboard over strong consistency
   + Reads are fast (Redis), writes do not block on recomputation
   - Leaderboard may be 5 seconds stale

   Right for us because: fitness leaderboards are not financial data.
   A 5-second delay is invisible to users.

3. Client-side persistence over server-only reliability
   + Workouts are never lost, even offline
   - Sync conflicts possible (rare — a user logging from two devices)

   Right for us because: the cost of losing a workout score is high
   (user frustration) and conflicts are rare (one person, one workout).
```

> **You:** "If I had more time, I would design the monitoring layer. Key metrics: write latency p99, leaderboard cache hit rate, Redis sorted set cardinality per WOD, and cross-tenant query detection alerts. I would also explore WebSocket push for live leaderboard updates during competitions — athletes love seeing positions change in real time."

> **Marcus:** "That is a strong design. Thanks."

**Commentary:** The candidate structured the 45 minutes well: crisp requirements, quick but insightful capacity math, a clear high-level diagram, three deep dives that went to real implementation depth (SQL, Redis commands, failure modes), and a summary that showed awareness of tradeoffs rather than just defending their choices.

---

## Common Mistakes

These are the patterns that trip candidates up most often. Recognizing them in advance is half the battle.

### Coding Interview Mistakes

**1. Coding before understanding the problem.**
You write 15 lines, realize you misunderstood a constraint, and start over. You have lost 5 minutes and your confidence. Spend 2-3 minutes on clarifying questions. It is never wasted time.

**2. Going silent while thinking.**
The interviewer cannot evaluate what they cannot observe. If you are thinking, say "I am considering whether a greedy approach works here, but I think there is a counterexample..." Even narrating your uncertainty is better than silence.

**3. Skipping the brute force.**
You want to impress with the optimal solution. But if you cannot get the optimal in time, you have nothing to show. State the brute force, note its complexity, then optimize. A working O(n^2) solution beats an incomplete O(n) attempt.

**4. Not testing your code.**
You finish coding with 3 minutes left and say "I think that is right." Instead, trace through a small example. Find the off-by-one error. Every interviewer has seen candidates lose offers because they did not test.

**5. Ignoring edge cases until the end.**
Empty input, single element, all duplicates, all negative. Ask about these in the clarifying phase and handle them in your code from the start.

### System Design Mistakes

**1. Diving into components without requirements.**
You start drawing boxes 30 seconds in. The interviewer thinks "they are guessing, not designing." Spend 5 minutes on requirements. It shapes everything that follows.

**2. Vague capacity estimates.**
"It is a lot of data" is not an estimate. Do the arithmetic. Even rough numbers ("about 50 GB per year") show engineering judgment.

**3. Ignoring failure modes.**
"Redis handles the leaderboard" — what happens when Redis goes down? Every component you draw should have a failure story. The interviewer will ask, and "I had not thought about that" is a bad answer.

**4. Designing for 10x the stated scale.**
If the problem says 1M users, do not design for 1B. Over-engineering is as much a red flag as under-engineering. Show that you make proportional decisions.

**5. Not making tradeoffs explicit.**
Every design decision has a cost. If you chose eventual consistency, say why and what you gave up. If you chose a relational database, acknowledge what would change with a document store. Interviewers want to see that you evaluate options, not that you have memorized a single architecture.

---

## Self-Evaluation Rubric

After completing each mock interview, score yourself in these areas. Be honest — the point is to identify what to practice, not to feel good.

### Coding Interview Rubric

| Dimension | 1 (Needs Work) | 2 (Adequate) | 3 (Strong) |
|-----------|-----------------|---------------|-------------|
| **Communication** | Long silences. Interviewer had to prompt for explanations. | Talked through approach but went quiet during coding. | Narrated thinking continuously. Explained trade-offs before being asked. |
| **Problem Analysis** | Started coding immediately. Missed constraints. | Asked 1-2 clarifying questions. Identified basic edge cases. | Asked targeted questions that shaped the solution. Identified all edge cases early. |
| **Brute Force** | Skipped it or could not articulate one. | Stated brute force verbally but did not analyze complexity. | Stated brute force, analyzed complexity, used it as stepping stone to optimal. |
| **Optimized Solution** | Could not find one, or found it but could not implement. | Found the optimization, coded it with minor bugs. | Found the optimization, explained the insight, coded it cleanly. |
| **Code Quality** | Messy variable names, no structure, copy-pasted blocks. | Readable code with reasonable structure. | Clean functions, descriptive names, separated concerns. Would pass code review. |
| **Testing** | Did not test. | Tested the happy path only. | Traced through example step by step. Tested edge cases. Found and fixed a bug. |
| **Follow-Up** | Could not extend the solution. | Described a general approach to the follow-up. | Gave a concrete implementation plan with complexity analysis. |

**Target:** Score 2 or higher in every dimension. Score 3 in at least four.

### System Design Rubric

| Dimension | 1 (Needs Work) | 2 (Adequate) | 3 (Strong) |
|-----------|-----------------|---------------|-------------|
| **Requirements** | Jumped to design. Missed key requirements. | Listed requirements but did not prioritize. | Clarified with interviewer, distinguished functional vs non-functional, prioritized. |
| **Capacity Estimation** | Skipped or hand-waved. | Computed numbers but did not draw conclusions. | Computed numbers and used them to drive design decisions. |
| **High-Level Design** | Missing components. Unclear data flow. | Drew all major components. Basic data flow described. | Clear diagram with labeled data flows. Explained why each component exists. |
| **Deep Dive Depth** | Surface-level descriptions ("use Redis for caching"). | Described the mechanism (sorted sets, cache keys). | Described the mechanism, failure modes, degradation strategy, and specific commands/queries. |
| **Tradeoffs** | Presented one solution as obviously correct. | Mentioned alternatives when asked. | Proactively compared alternatives, stated tradeoffs, justified choices. |
| **Communication** | Disorganized. Hard to follow the design. | Logical flow but some jumps. | Clear structure: requirements then capacity then HLD then deep dives. Signposted transitions. |
| **Practical Knowledge** | Abstract descriptions only. | Referenced some real technologies. | Used real tech with correct details (Redis ZADD, PostgreSQL RLS, specific index types). |

**Target:** Score 2 or higher in every dimension. Score 3 in at least three.

---

## How to Practice

Reading this chapter is not practice. Practice means doing these interviews under time pressure, out loud, with feedback.

### Solo Practice (Good)
1. Set a 45-minute timer
2. Pick a problem (use ones from Chapters 19 and 20)
3. Talk out loud to an empty chair — yes, it feels absurd, and it works
4. Record yourself if possible — watch for silence gaps and unclear explanations
5. Score yourself with the rubric above

### Partner Practice (Better)
1. Find a study partner — another engineer preparing for interviews
2. Take turns being interviewer and candidate
3. As interviewer: ask follow-ups, push back on weak points, note where the candidate went silent
4. As candidate: resist the urge to break character and ask "am I doing this right?"
5. Debrief after each session with the rubric

### Mock Interview Platforms (Best for Calibration)
1. Services like Pramp, interviewing.io, or Exercism offer real-time mock interviews
2. Use these to calibrate — your study partner may be too lenient or too harsh
3. Do at least 2-3 platform mocks before real interviews

### Recommended Schedule
- **Weeks 1-2:** Solo practice. One coding problem per day (30 min). One system design per week (45 min).
- **Weeks 3-4:** Partner practice. Two coding mocks per week. One system design mock per week.
- **Week 5:** Platform mocks for calibration. Identify weak dimensions from the rubric.
- **Week 6:** Targeted practice on weak dimensions only. Rest the day before.

---

## Final Thoughts

The two mock interviews in this chapter used GrindIt as the domain because you already understand the data model, the user flows, and the architectural decisions. In a real interview, you will not have that advantage — the domain will be unfamiliar. But the *process* is identical: clarify, plan, implement, verify, discuss tradeoffs.

The coding interview tests whether you can decompose a problem under pressure. The system design interview tests whether you can make and defend engineering decisions. Both of them really test whether you can communicate clearly while doing technical work. That is the skill that improves most with practice and degrades most without it.

Go back to Chapters 19 and 20. Pick problems you have not solved yet. Set a timer. Talk out loud. Score yourself. Repeat.

You built GrindIt from zero lines of Rust to a deployed, multi-feature fitness tracker. You understand the code because you wrote it. Now go show an interviewer what that kind of deep understanding looks like.
