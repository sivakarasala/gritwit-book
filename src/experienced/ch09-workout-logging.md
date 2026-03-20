# Chapter 9: Workout Logging & Scoring

Athletes need to record what they did. A WOD without scores is just a whiteboard; a WOD with scores becomes training data — PRs, progressions, leaderboards, and accountability. This chapter builds the workout logging system: athletes select a WOD, score each section (time, rounds, weight), log per-movement results, and submit everything to the database.

The spotlight concept is **traits and generics** — the mechanisms Rust uses to write code that works across multiple types. You will see `Serialize`/`Deserialize` (serde) for data crossing the client-server boundary, `impl IntoView` as a generic return type, callback props with `impl Fn() + Copy + 'static`, and the `Resource` pattern for async data loading. These are not abstract — they are the traits you use in every Leptos component.

By the end of this chapter, you will have:

- Database tables for `workout_logs`, `section_logs`, and `movement_logs` with the full scoring hierarchy
- Scoring structs (`SectionScoreInput`, `MovementLogInput`, `MovementLogSetInput`) with serde derives
- A `WodScoreForm` that loads a WOD's sections and renders per-section scoring cards
- A `SectionScoreCard` with time/rounds/weight inputs, RX toggle, and per-movement tracking
- A `submit_wod_scores` server function that inserts the workout log, section logs, and movement logs in a single transaction
- Tabbed navigation between WOD Score and Custom Log modes

---

## Spotlight: Traits & Generics

### What is a trait?

A trait defines shared behavior — a contract that types can implement. If you come from TypeScript, traits are like interfaces. If you come from Go, they are like interface types. But Rust traits are resolved at compile time, not runtime.

```rust
// The Serialize trait from serde
pub trait Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer;
}
```

Every type that implements `Serialize` can be converted to JSON, MessagePack, TOML, or any other format that has a `Serializer`. You do not need to know which format at compile time — the generic `S` is filled in when the code is used.

### `derive` macros: automatic trait implementations

Rust can automatically implement certain traits using `derive`:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionScoreInput {
    pub section_id: String,
    pub finish_time_seconds: Option<i32>,
    pub rounds_completed: Option<i32>,
    pub extra_reps: Option<i32>,
    pub weight_kg: Option<f32>,
    pub notes: Option<String>,
    pub is_rx: bool,
    pub skipped: bool,
    pub movement_logs: Vec<MovementLogInput>,
}
```

That single `#[derive(Serialize, Deserialize)]` line generates hundreds of lines of implementation code. It knows how to serialize every field because each field's type (`String`, `Option<i32>`, `bool`, `Vec<MovementLogInput>`) also implements `Serialize`. The derive macro walks the struct recursively — if `MovementLogInput` has its own nested types, they must also implement `Serialize`.

This is why GrindIt marks every struct that crosses the client-server boundary with `Serialize, Deserialize`. Server functions serialize their arguments to JSON on the client, send them over HTTP, and deserialize them on the server. The return value follows the reverse path.

### `impl IntoView` — the universal Leptos return type

Every Leptos component returns `impl IntoView`:

```rust
#[component]
pub fn SectionScoreCard(state: SectionScoreState, focused: bool) -> impl IntoView {
    // ... can return view!, ().into_view(), String, etc.
}
```

`impl IntoView` means "some type that implements the `IntoView` trait." The caller does not know the concrete type — it could be a `HtmlDiv`, a `Fragment`, a `String`, or any other renderable element. The compiler figures out the concrete type and optimizes accordingly.

This is different from JavaScript's `React.ReactNode`, which is a runtime union type. In Rust, `impl IntoView` is resolved at compile time — there is zero runtime cost for the abstraction.

### `.into_any()` — when branches return different types

You have already seen this pattern:

```rust
if condition {
    view! { <div>"A"</div> }.into_any()
} else {
    view! { <span>"B"</span> }.into_any()
}
```

The two branches return different concrete types (`HtmlDiv` vs `HtmlSpan`), but `into_any()` erases the concrete type and returns a common `AnyView`. Without this, Rust would complain that the `if` arms have different types — because they do. JavaScript does not have this problem because everything is a runtime value; Rust needs this bridge because types are resolved at compile time.

### Callback props: `impl Fn() + Copy + 'static`

When a parent component passes an event handler to a child:

```rust
#[component]
fn DeleteModal(
    show: RwSignal<bool>,
    on_confirm: impl Fn() + Copy + 'static,
) -> impl IntoView {
    view! {
        <button on:click=move |_| {
            on_confirm();
            show.set(false);
        }>"Confirm"</button>
    }
}
```

The `impl Fn() + Copy + 'static` means:
- `Fn()` — it is a callable function taking no arguments
- `Copy` — it can be copied (not just moved), so closures that capture only `Copy` types work
- `'static` — it does not borrow any short-lived data (it owns everything it needs)

In GrindIt, most callback props are closures that capture `RwSignal`s. Signals are `Copy`, so closures over them are also `Copy`. This is why Leptos works so well with closures — signals are cheap to copy, and closures over signals are cheap to pass around.

### `Resource::new()` — generic async data loading

```rust
let wods = Resource::new(
    move || (selected_date.get(), create_action.version().get()),
    |(date, _)| list_wods_for_date(date),
);
```

`Resource::new` is generic over:
- The **source** type (the tuple of signals that trigger refetching)
- The **future** type (the async function that loads data)
- The **result** type (what the future returns)

The compiler infers all three from usage. You never write `Resource::<(String, usize), _, Vec<Wod>>::new(...)` — the types flow from the closure bodies.

> **Coming from JS?**
>
> | Concept | TypeScript | Rust |
> |---------|-----------|------|
> | Generic function | `function identity<T>(x: T): T` | `fn identity<T>(x: T) -> T` |
> | Generic constraint | `function f<T extends Serializable>(x: T)` | `fn f<T: Serialize>(x: T)` |
> | Derive/auto-implement | Not available (must write manually) | `#[derive(Serialize, Clone)]` |
> | Type erasure | Runtime `any` or union types | `impl Trait` (compile-time) or `dyn Trait` (runtime) |
> | Callback type | `() => void` | `impl Fn() + Copy + 'static` |
>
> The biggest difference: TypeScript generics exist only at compile time (erased at runtime). Rust generics are **monomorphized** — the compiler generates specialized code for each concrete type. `Resource<(String, usize), Vec<Wod>>` and `Resource<String, AuthUser>` are two different types with two different compiled code paths. This means zero runtime overhead from generics.

---

## Exercise 1: Define the Scoring Structs

**Goal:** Create the database tables and Rust structs for workout logs, section scores, and movement logs.

### Step 1: The workout_logs table

This table already exists from Chapter 5 (basic logging), but we need to extend it:

```sql
-- migrations/XXXXXX_rework_workout_logs.sql
ALTER TABLE workout_logs ADD COLUMN wod_id UUID REFERENCES wods(id) ON DELETE SET NULL;
ALTER TABLE workout_logs ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE INDEX idx_workout_logs_wod ON workout_logs (wod_id);
```

A workout log can now optionally reference a WOD. Custom logs (free-form workouts) have `wod_id = NULL`. WOD scores link to the WOD they were programmed against.

### Step 2: The section_logs table

```sql
-- migrations/XXXXXX_create_section_logs.sql
CREATE TABLE section_logs (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workout_log_id      UUID NOT NULL REFERENCES workout_logs(id) ON DELETE CASCADE,
    section_id          UUID NOT NULL REFERENCES wod_sections(id) ON DELETE CASCADE,
    finish_time_seconds INTEGER,
    rounds_completed    INTEGER,
    extra_reps          INTEGER,
    weight_kg           REAL,
    notes               TEXT,
    is_rx               BOOLEAN NOT NULL DEFAULT true,
    skipped             BOOLEAN NOT NULL DEFAULT false,
    score_value         INTEGER,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_section_logs_workout_log ON section_logs (workout_log_id);
CREATE INDEX idx_section_logs_section ON section_logs (section_id);
```

Each `section_log` records one athlete's result for one section. The scoring fields are polymorphic — different section types use different fields:

| Section Type | Primary Score Field | Secondary Fields |
|-------------|-------------------|-----------------|
| For Time | `finish_time_seconds` | `is_rx` |
| AMRAP | `rounds_completed` | `extra_reps`, `is_rx` |
| EMOM | `rounds_completed` | `extra_reps` |
| Strength | `weight_kg` | |
| Static | (none) | `notes`, `skipped` |

The `score_value` column stores a computed integer for leaderboard ranking (e.g., seconds for For Time, `rounds * 1000 + extra_reps` for AMRAP). This avoids recomputing the comparison logic every time the leaderboard is loaded.

### Step 3: The movement_logs table

```sql
-- migrations/XXXXXX_create_movement_logs.sql
CREATE TABLE movement_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    section_log_id  UUID NOT NULL REFERENCES section_logs(id) ON DELETE CASCADE,
    movement_id     UUID NOT NULL REFERENCES wod_movements(id) ON DELETE CASCADE,
    reps            INTEGER,
    sets            INTEGER,
    weight_kg       REAL,
    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_movement_logs_section_log ON movement_logs(section_log_id);
```

And for set-by-set tracking:

```sql
-- migrations/XXXXXX_create_movement_log_sets.sql
CREATE TABLE movement_log_sets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    movement_log_id UUID NOT NULL REFERENCES movement_logs(id) ON DELETE CASCADE,
    set_number      INTEGER NOT NULL,
    reps            INTEGER,
    weight_kg       REAL,
    distance_meters REAL,
    calories        INTEGER,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_movement_log_sets_log ON movement_log_sets(movement_log_id);
```

This is the deepest level of the scoring hierarchy: workout_log -> section_log -> movement_log -> movement_log_sets. A single submission creates records across all four tables.

### Step 4: The Rust input structs

```rust
// src/db.rs — structs for submitting scores

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionScoreInput {
    pub section_id: String,
    pub finish_time_seconds: Option<i32>,
    pub rounds_completed: Option<i32>,
    pub extra_reps: Option<i32>,
    pub weight_kg: Option<f32>,
    pub notes: Option<String>,
    pub is_rx: bool,
    pub skipped: bool,
    #[serde(default)]
    pub movement_logs: Vec<MovementLogInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementLogInput {
    pub movement_id: String,
    pub reps: Option<i32>,
    pub sets: Option<i32>,
    pub weight_kg: Option<f32>,
    pub notes: Option<String>,
    #[serde(default)]
    pub set_details: Vec<MovementLogSetInput>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MovementLogSetInput {
    pub set_number: i32,
    pub reps: Option<i32>,
    pub weight_kg: Option<f32>,
    pub distance_meters: Option<f32>,
    pub calories: Option<i32>,
}
```

The `#[serde(default)]` attribute on `movement_logs` and `set_details` means: if the field is missing in the JSON, use the default value (`Vec::new()` for `Vec`). This is important because older clients or simple submissions might not include movement-level data.

Notice that these are *input* structs — they carry data from the client to the server. The *output* structs (for reading back from the database) are different:

```rust
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionLog {
    pub id: String,
    pub workout_log_id: String,
    pub section_id: String,
    pub finish_time_seconds: Option<i32>,
    pub rounds_completed: Option<i32>,
    pub extra_reps: Option<i32>,
    pub weight_kg: Option<f32>,
    pub notes: Option<String>,
    pub is_rx: bool,
    pub skipped: bool,
    pub score_value: Option<i32>,
}
```

The input struct does not have `id`, `workout_log_id`, or `score_value` — those are generated by the server. The output struct does not have `movement_logs` — movements are loaded separately. This separation of input/output types is a common pattern in Rust web applications.

<details>
<summary>Hint: If serde deserialization fails with "missing field movement_logs"</summary>

Add `#[serde(default)]` to the `movement_logs` and `set_details` fields. Without this attribute, serde requires the field to be present in the JSON. With `default`, a missing field is deserialized as the type's default value (`Vec::new()` for `Vec`, `0` for integers, `false` for booleans).

You can also set a specific default: `#[serde(default = "default_true")]` where `fn default_true() -> bool { true }`.

</details>

---

## Exercise 2: Build the WodScoreForm

**Goal:** A form that loads a WOD's sections and renders a scoring card for each one, with a submit button that sends all scores to the server.

### Step 1: The page structure with tabs

```rust
// src/pages/log_workout/mod.rs
use leptos::prelude::*;

#[component]
pub fn LogWorkoutPage() -> impl IntoView {
    let params = leptos_router::hooks::use_query_map();

    let section_id = Memo::new(move |_| {
        params.read().get("section_id").unwrap_or_default().to_string()
    });
    let wod_id_param = Memo::new(move |_| {
        params.read().get("wod_id").unwrap_or_default().to_string()
    });

    let active_tab = RwSignal::new("wod".to_string());

    view! {
        <div class="log-workout-page">
            <div class="log-tabs">
                <button
                    class="log-tab"
                    class:active=move || active_tab.get() == "wod"
                    on:click=move |_| active_tab.set("wod".to_string())
                >"WOD Score"</button>
                <button
                    class="log-tab"
                    class:active=move || active_tab.get() == "custom"
                    on:click=move |_| active_tab.set("custom".to_string())
                >"Custom Log"</button>
            </div>

            {move || {
                if active_tab.get() == "wod" {
                    view! {
                        <WodScoreFlow
                            section_id=section_id.get()
                            wod_id=wod_id_param.get()
                        />
                    }.into_any()
                } else {
                    view! { <CustomLogFlow /> }.into_any()
                }
            }}
        </div>
    }
}
```

The page reads query parameters to determine which WOD to score. Navigating from a WOD card (`/log?wod_id=abc-123`) pre-selects the WOD. Navigating from a specific section (`/log?section_id=xyz-789`) focuses on that one section.

### Step 2: The WOD score flow

```rust
#[component]
fn WodScoreFlow(section_id: String, wod_id: String) -> impl IntoView {
    let selected_wod_id = RwSignal::new(wod_id.clone());

    let resolved_wod = Resource::new(
        move || (section_id.clone(), selected_wod_id.get()),
        |(sid, wid)| async move {
            if !sid.is_empty() {
                return get_wod_by_section(sid).await.map(Some);
            }
            if !wid.is_empty() {
                return get_wod_for_scoring(wid).await.map(Some);
            }
            Ok(None)
        },
    );

    let todays_wods = Resource::new(|| (), |_| get_todays_wods());

    view! {
        <Suspense fallback=|| view! { <p class="loading">"Loading..."</p> }>
            {move || {
                let wod_data = resolved_wod.get().and_then(|r| r.ok()).flatten();

                if let Some((wod, sections, _movements)) = wod_data {
                    view! {
                        <WodScoreForm wod=wod sections=sections focus_section=section_id.clone() />
                    }.into_any()
                } else {
                    // Show WOD picker if no specific WOD is selected
                    let wods = todays_wods.get()
                        .and_then(|r| r.ok())
                        .unwrap_or_default();

                    if wods.is_empty() {
                        view! {
                            <div class="empty-state">
                                <p class="empty-title">"No WODs Today"</p>
                                <p class="empty-sub">"Use the Custom Log tab or check back later."</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="wod-picker">
                                <p class="picker-label">"Select a workout to log:"</p>
                                {wods.into_iter().map(|w| {
                                    let wid = w.id.clone();
                                    view! {
                                        <button class="wod-pick-card"
                                            on:click=move |_| selected_wod_id.set(wid.clone())
                                        >
                                            <span class="wod-pick-title">{w.title.clone()}</span>
                                            <span class="wod-pick-type">{w.workout_type.clone()}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                }
            }}
        </Suspense>
    }
}
```

This component has three states:
1. **WOD loaded** — show the scoring form
2. **No WOD selected, today has WODs** — show a picker
3. **No WOD selected, no WODs today** — show empty state

The Resource pattern chains through these: first it tries to load a specific WOD (by section ID or WOD ID), then falls back to listing today's WODs for the picker.

### Step 3: The WodScoreForm

```rust
// src/pages/log_workout/wod_score_form.rs
use crate::db::{MovementLogInput, MovementLogSetInput, SectionScoreInput, Wod, WodSection};
use leptos::prelude::*;

#[component]
pub fn WodScoreForm(wod: Wod, sections: Vec<WodSection>, focus_section: String) -> impl IntoView {
    let submit_result = RwSignal::new(Option::<Result<String, String>>::None);
    let submitting = RwSignal::new(false);
    let workout_date = RwSignal::new(wod.programmed_date.clone());
    let overall_notes = RwSignal::new(String::new());

    // Filter to focused section if specified
    let visible_sections: Vec<WodSection> = if !focus_section.is_empty() {
        sections.into_iter().filter(|s| s.id == focus_section).collect()
    } else {
        sections
    };

    // Create reactive state for each section
    let section_states: Vec<SectionScoreState> = visible_sections
        .iter()
        .map(|s| SectionScoreState {
            section_id: s.id.clone(),
            section_type: s.section_type.clone(),
            phase: s.phase.clone(),
            title: s.title.clone().unwrap_or_else(|| phase_label(&s.phase)),
            time_cap: s.time_cap_minutes,
            rounds: s.rounds,
            is_rx: RwSignal::new(true),
            skipped: RwSignal::new(false),
            minutes: RwSignal::new(String::new()),
            seconds: RwSignal::new(String::new()),
            rounds_completed: RwSignal::new(String::new()),
            extra_reps: RwSignal::new(String::new()),
            weight_kg: RwSignal::new(String::new()),
            notes: RwSignal::new(String::new()),
            movement_states: RwSignal::new(Vec::new()),
            existing_movement_logs: vec![],
        })
        .collect();

    // ... submission handler (covered in Exercise 4) ...

    view! {
        <div class="wod-score-form">
            <div class="score-header">
                <h2 class="score-wod-title">{wod.title.clone()}</h2>
                {wod.description.clone().map(|d| view! {
                    <p class="score-wod-desc">{d}</p>
                })}
            </div>

            <div class="score-sections">
                {section_states.into_iter().map(|state| {
                    let focused = state.section_id == focus_section;
                    view! { <SectionScoreCard state=state focused=focused /> }
                }).collect_view()}
            </div>

            <div class="score-footer">
                <div class="score-field">
                    <label class="score-label">"Notes (optional)"</label>
                    <textarea
                        class="score-textarea"
                        placeholder="How did it feel?"
                        prop:value=move || overall_notes.get()
                        on:input=move |ev| overall_notes.set(event_target_value(&ev))
                    ></textarea>
                </div>

                <button
                    class="score-submit"
                    class:btn--loading=move || submitting.get()
                    class:btn--success=move || matches!(submit_result.get(), Some(Ok(_)))
                    disabled=move || submitting.get()
                        || matches!(submit_result.get(), Some(Ok(_)))
                    on:click=on_submit
                >
                    {move || if matches!(submit_result.get(), Some(Ok(_))) {
                        "Saved!".to_string()
                    } else if submitting.get() {
                        "Submitting...".to_string()
                    } else {
                        "Log Score".to_string()
                    }}
                </button>
            </div>
        </div>
    }
}
```

The `SectionScoreState` struct holds all reactive state for scoring one section. Notice the pattern: non-reactive data (`section_id`, `section_type`, `phase`, `title`, `time_cap`) and reactive data (`is_rx`, `minutes`, `seconds`, etc.) coexist in the same struct. The non-reactive fields identify the section; the reactive fields capture user input.

<details>
<summary>Hint: If sections appear empty with no movement tracking</summary>

The movement states are loaded lazily — the `SectionScoreCard` component fetches movements for its section when it mounts. If the fetch fails (e.g., because the section has no movements), the movement tracking UI will not appear. Check the browser console for server function errors.

Also verify that movements were added to the sections in the WOD programming step (Chapter 8). A section without movements is valid (e.g., a "Static" cool-down with just notes), but most scoring sections should have movements.

</details>

---

## Exercise 3: Build the SectionScoreCard

**Goal:** A card for scoring one section of a WOD, with inputs that adapt to the section type and per-movement tracking.

### Step 1: The reactive state struct

```rust
// src/pages/log_workout/section_score_card.rs
#[derive(Clone)]
pub(super) struct SectionScoreState {
    pub section_id: String,
    pub section_type: String,
    pub phase: String,
    pub title: String,
    pub time_cap: Option<i32>,
    pub rounds: Option<i32>,
    pub is_rx: RwSignal<bool>,
    pub skipped: RwSignal<bool>,
    pub minutes: RwSignal<String>,
    pub seconds: RwSignal<String>,
    pub rounds_completed: RwSignal<String>,
    pub extra_reps: RwSignal<String>,
    pub weight_kg: RwSignal<String>,
    pub notes: RwSignal<String>,
    pub movement_states: RwSignal<Vec<MovementLogState>>,
    pub existing_movement_logs: Vec<MovementLog>,
}

#[derive(Clone)]
pub(super) struct MovementLogState {
    pub movement_id: String,
    pub exercise_name: String,
    pub scoring_type: String,
    pub prescribed_reps: Option<String>,
    pub prescribed_weight_male: Option<f32>,
    pub prescribed_weight_female: Option<f32>,
    pub reps: RwSignal<String>,
    pub sets: RwSignal<String>,
    pub weight_kg: RwSignal<String>,
    pub distance_meters: RwSignal<String>,
    pub calories: RwSignal<String>,
    pub duration_seconds: RwSignal<String>,
    pub notes: RwSignal<String>,
    pub set_rows: Vec<MovementSetState>,
}
```

The `MovementLogState` adapts to the exercise's scoring type. A weightlifting movement shows reps/weight inputs. A conditioning movement (like rowing) might show calories or distance. The `scoring_type` field (inherited from the exercise library) determines which inputs are shown.

### Step 2: The component with type-dependent inputs

```rust
#[component]
pub fn SectionScoreCard(state: SectionScoreState, focused: bool) -> impl IntoView {
    let section_type = state.section_type.clone();

    // Load movements for this section
    let movements = Resource::new(
        || (),
        {
            let sid = state.section_id.clone();
            move |_| {
                let sid = sid.clone();
                async move { get_section_movements_for_log(sid).await }
            }
        },
    );

    // Initialize movement states when data arrives
    Effect::new({
        let ms = state.movement_states;
        move |_| {
            if let Some(Ok(mvts)) = movements.get() {
                if ms.get_untracked().is_empty() {
                    let states: Vec<MovementLogState> = mvts.iter().map(|m| {
                        MovementLogState {
                            movement_id: m.id.clone(),
                            exercise_name: m.exercise_name.clone(),
                            scoring_type: m.scoring_type.clone(),
                            prescribed_reps: m.rep_scheme.clone(),
                            prescribed_weight_male: m.weight_kg_male,
                            prescribed_weight_female: m.weight_kg_female,
                            reps: RwSignal::new(String::new()),
                            sets: RwSignal::new(String::new()),
                            weight_kg: RwSignal::new(String::new()),
                            distance_meters: RwSignal::new(String::new()),
                            calories: RwSignal::new(String::new()),
                            duration_seconds: RwSignal::new(String::new()),
                            notes: RwSignal::new(String::new()),
                            set_rows: vec![],
                        }
                    }).collect();
                    ms.set(states);
                }
            }
        }
    });

    view! {
        <div class="section-score-card" class:focused=focused>
            <div class="section-score-header">
                <span class=format!("phase-badge {}", phase_class(&state.phase))>
                    {phase_label(&state.phase)}
                </span>
                <span class="section-title">{state.title.clone()}</span>
            </div>

            // RX / Scaled toggle
            <div class="rx-toggle">
                <button
                    class="rx-btn" class:active=move || state.is_rx.get()
                    on:click=move |_| state.is_rx.set(true)
                >"RX"</button>
                <button
                    class="rx-btn" class:active=move || !state.is_rx.get()
                    on:click=move |_| state.is_rx.set(false)
                >"Scaled"</button>
            </div>

            // Type-specific scoring inputs
            {match section_type.as_str() {
                "fortime" => view! {
                    <div class="time-inputs">
                        <input type="number" placeholder="Min"
                            prop:value=move || state.minutes.get()
                            on:input=move |ev| state.minutes.set(event_target_value(&ev))
                        />
                        <span>":"</span>
                        <input type="number" placeholder="Sec"
                            prop:value=move || state.seconds.get()
                            on:input=move |ev| state.seconds.set(event_target_value(&ev))
                        />
                    </div>
                }.into_any(),
                "amrap" | "emom" => view! {
                    <div class="rounds-inputs">
                        <input type="number" placeholder="Rounds"
                            prop:value=move || state.rounds_completed.get()
                            on:input=move |ev| state.rounds_completed.set(
                                event_target_value(&ev)
                            )
                        />
                        <span>"+"</span>
                        <input type="number" placeholder="Reps"
                            prop:value=move || state.extra_reps.get()
                            on:input=move |ev| state.extra_reps.set(event_target_value(&ev))
                        />
                    </div>
                }.into_any(),
                "strength" => view! {
                    <div class="weight-input">
                        <input type="number" step="0.5" placeholder="Weight (kg)"
                            prop:value=move || state.weight_kg.get()
                            on:input=move |ev| state.weight_kg.set(event_target_value(&ev))
                        />
                    </div>
                }.into_any(),
                _ => ().into_view().into_any(),
            }}

            // Per-movement tracking
            <Suspense fallback=|| ()>
                {move || {
                    let mvt_states = state.movement_states.get();
                    if mvt_states.is_empty() {
                        ().into_view().into_any()
                    } else {
                        view! {
                            <div class="movement-logs">
                                {mvt_states.into_iter().map(|m| {
                                    view! { <MovementLogRow state=m /> }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                }}
            </Suspense>
        </div>
    }
}
```

The `match` on `section_type.as_str()` is the strategy pattern in action — each section type has its own scoring UI. For Time shows minutes:seconds. AMRAP shows rounds+reps. Strength shows weight. Static sections show nothing (just a skip/notes option).

The movement states are loaded lazily via a `Resource` and initialized in an `Effect`. This avoids blocking the initial render — the section card appears immediately with the scoring inputs, and the movement tracking rows appear once the data arrives.

<details>
<summary>Hint: If the RX toggle does not switch</summary>

Verify that the `is_rx` signal is being set in the click handler, not toggled. The two buttons are mutually exclusive: one sets `true`, the other sets `false`. If you use a single toggle (`is_rx.update(|v| *v = !*v)`), both buttons would do the same thing.

Also check that `class:active` is bound to the correct condition: the RX button should be active when `is_rx.get()` is `true`, and the Scaled button should be active when `is_rx.get()` is `false`.

</details>

---

## Exercise 4: Implement the Submit Server Function

**Goal:** A server function that receives the scoring data as JSON, validates it, and inserts records across workout_logs, section_logs, movement_logs, and movement_log_sets tables.

### Step 1: The submit handler on the client

```rust
let on_submit = move |_| {
    submitting.set(true);
    submit_result.set(None);

    // Collect scores from all section states
    let scores: Vec<(SectionScoreInput, String)> = section_states_submit
        .iter()
        .map(|s| {
            let finish_time = if s.section_type == "fortime" {
                let mins: i32 = s.minutes.get_untracked().parse().unwrap_or(0);
                let secs: i32 = s.seconds.get_untracked().parse().unwrap_or(0);
                let total = mins * 60 + secs;
                if total > 0 { Some(total) } else { None }
            } else {
                None
            };

            let rounds_completed = if s.section_type == "amrap"
                || s.section_type == "emom"
            {
                s.rounds_completed.get_untracked().parse().ok()
            } else {
                None
            };

            let weight_kg = if s.section_type == "strength" {
                s.weight_kg.get_untracked().parse().ok()
            } else {
                None
            };

            // Collect movement log inputs
            let movement_logs: Vec<MovementLogInput> = s.movement_states
                .get_untracked()
                .iter()
                .filter_map(|m| {
                    let reps = m.reps.get_untracked().parse().ok();
                    let sets = m.sets.get_untracked().parse().ok();
                    let w: Option<f32> = m.weight_kg.get_untracked().parse().ok();
                    let n = m.notes.get_untracked();

                    if reps.is_some() || sets.is_some() || w.is_some() || !n.is_empty() {
                        Some(MovementLogInput {
                            movement_id: m.movement_id.clone(),
                            reps,
                            sets,
                            weight_kg: w,
                            notes: if n.is_empty() { None } else { Some(n) },
                            set_details: vec![],
                        })
                    } else {
                        None
                    }
                })
                .collect();

            (SectionScoreInput {
                section_id: s.section_id.clone(),
                finish_time_seconds: finish_time,
                rounds_completed,
                extra_reps: s.extra_reps.get_untracked().parse().ok(),
                weight_kg,
                notes: {
                    let n = s.notes.get_untracked();
                    if n.is_empty() { None } else { Some(n) }
                },
                is_rx: s.is_rx.get_untracked(),
                skipped: s.skipped.get_untracked(),
                movement_logs,
            }, s.section_type.clone())
        })
        .collect();

    let scores_json = serde_json::to_string(&scores).unwrap_or_default();
    let wod_id = wod_id.clone();
    let date = workout_date.get_untracked();
    let notes = overall_notes.get_untracked();

    let nav = navigate.clone();
    leptos::task::spawn_local(async move {
        let result = submit_wod_scores(wod_id, date.clone(), notes, scores_json).await;
        submitting.set(false);
        match result {
            Ok(_) => {
                submit_result.set(Some(Ok("Score logged!".to_string())));
                set_timeout(
                    move || nav(&format!("/history?date={}", date), Default::default()),
                    std::time::Duration::from_millis(800),
                );
            }
            Err(e) => {
                let msg = friendly_error(&e.to_string());
                submit_result.set(Some(Err(msg)));
            }
        }
    });
};
```

The client serializes the scores to JSON and sends them as a single string. This is a pragmatic choice — Leptos server functions work best with simple types (strings, numbers, booleans). Sending a `Vec<SectionScoreInput>` directly would require Leptos to serialize each field individually as form data. Sending the entire batch as a JSON string keeps it simple.

**`spawn_local`** runs an async block on the client without blocking the UI. The submitting signal is set to `true` before the async call and `false` after, driving the loading state on the submit button.

### Step 2: The server function

```rust
// src/pages/log_workout/server_fns.rs
#[server]
pub async fn submit_wod_scores(
    wod_id: String,
    workout_date: String,
    notes: String,
    scores_json: String,
) -> Result<String, ServerFnError> {
    let user = crate::auth::session::require_auth().await?;
    validate_date(&workout_date)?;

    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = user.id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
    let wod_uuid: uuid::Uuid = wod_id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    let sections: Vec<(SectionScoreInput, String)> = serde_json::from_str(&scores_json)
        .map_err(|e| ServerFnError::new(format!("Invalid scores data: {}", e)))?;

    // Check if user already has a log for this WOD on this date
    let log_id = if let Some(existing_id) =
        crate::db::has_wod_score_db(&pool, user_uuid, wod_uuid, &workout_date)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
    {
        // Append section scores to existing log
        let existing_uuid: uuid::Uuid = existing_id.parse()
            .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
        crate::db::add_section_scores_db(&pool, existing_uuid, &sections)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        existing_id
    } else {
        // Create new workout log + section scores
        let notes_opt = if notes.is_empty() { None } else { Some(notes.as_str()) };
        crate::db::submit_wod_score_db(
            &pool, user_uuid, wod_uuid, &workout_date, notes_opt, &sections,
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .to_string()
    };

    Ok(log_id)
}
```

### Step 3: The database layer

The `submit_wod_score_db` function inserts across four tables:

```rust
// src/db.rs (simplified)
#[cfg(feature = "ssr")]
pub async fn submit_wod_score_db(
    pool: &PgPool,
    user_id: uuid::Uuid,
    wod_id: uuid::Uuid,
    workout_date: &str,
    notes: Option<&str>,
    sections: &[(SectionScoreInput, String)],
) -> Result<uuid::Uuid, sqlx::Error> {
    // 1. Create the workout log
    let (log_id,): (uuid::Uuid,) = sqlx::query_as(
        r#"INSERT INTO workout_logs (user_id, workout_date, notes, wod_id, is_rx)
           VALUES ($1, $2::date, $3, $4, true)
           RETURNING id"#,
    )
    .bind(user_id)
    .bind(workout_date)
    .bind(notes)
    .bind(wod_id)
    .fetch_one(pool)
    .await?;

    // 2. Insert section logs and movement logs
    add_section_scores_db(pool, log_id, sections).await?;

    Ok(log_id)
}

#[cfg(feature = "ssr")]
pub async fn add_section_scores_db(
    pool: &PgPool,
    log_id: uuid::Uuid,
    sections: &[(SectionScoreInput, String)],
) -> Result<(), sqlx::Error> {
    for (section, section_type) in sections {
        let section_uuid: uuid::Uuid = section.section_id.parse()
            .map_err(|e: uuid::Error| sqlx::Error::Protocol(e.to_string()))?;

        // Compute score_value for leaderboard ranking
        let score_value = compute_score_value(section, section_type);

        let (section_log_id,): (uuid::Uuid,) = sqlx::query_as(
            r#"INSERT INTO section_logs
               (workout_log_id, section_id, finish_time_seconds,
                rounds_completed, extra_reps, weight_kg,
                notes, is_rx, skipped, score_value)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING id"#,
        )
        .bind(log_id)
        .bind(section_uuid)
        .bind(section.finish_time_seconds)
        .bind(section.rounds_completed)
        .bind(section.extra_reps)
        .bind(section.weight_kg)
        .bind(&section.notes)
        .bind(section.is_rx)
        .bind(section.skipped)
        .bind(score_value)
        .fetch_one(pool)
        .await?;

        // 3. Insert movement logs
        for mvt in &section.movement_logs {
            let mvt_uuid: uuid::Uuid = mvt.movement_id.parse()
                .map_err(|e: uuid::Error| sqlx::Error::Protocol(e.to_string()))?;

            let (mvt_log_id,): (uuid::Uuid,) = sqlx::query_as(
                r#"INSERT INTO movement_logs
                   (section_log_id, movement_id, reps, sets, weight_kg, notes)
                   VALUES ($1, $2, $3, $4, $5, $6)
                   RETURNING id"#,
            )
            .bind(section_log_id)
            .bind(mvt_uuid)
            .bind(mvt.reps)
            .bind(mvt.sets)
            .bind(mvt.weight_kg)
            .bind(&mvt.notes)
            .fetch_one(pool)
            .await?;

            // 4. Insert per-set details
            for set in &mvt.set_details {
                sqlx::query(
                    r#"INSERT INTO movement_log_sets
                       (movement_log_id, set_number, reps, weight_kg,
                        distance_meters, calories)
                       VALUES ($1, $2, $3, $4, $5, $6)"#,
                )
                .bind(mvt_log_id)
                .bind(set.set_number)
                .bind(set.reps)
                .bind(set.weight_kg)
                .bind(set.distance_meters)
                .bind(set.calories)
                .execute(pool)
                .await?;
            }
        }
    }

    Ok(())
}
```

The nested insert pattern: each level creates a row and returns its ID, which becomes the foreign key for the next level. This is the imperative version of the tree structure: `workout_log` -> `section_log` -> `movement_log` -> `movement_log_set`.

The `compute_score_value` function converts each section's score into a comparable integer:

```rust
fn compute_score_value(section: &SectionScoreInput, section_type: &str) -> Option<i32> {
    if section.skipped { return None; }
    match section_type {
        "fortime" => section.finish_time_seconds,
        "amrap" | "emom" => {
            let rounds = section.rounds_completed.unwrap_or(0);
            let reps = section.extra_reps.unwrap_or(0);
            Some(rounds * 1000 + reps)
        }
        "strength" => section.weight_kg.map(|w| (w * 100.0) as i32),
        _ => None,
    }
}
```

For AMRAP, the score is `rounds * 1000 + extra_reps`. An athlete who completed 5 rounds + 3 reps scores 5003. An athlete who completed 4 rounds + 15 reps scores 4015. This makes leaderboard sorting a simple `ORDER BY score_value DESC`.

<details>
<summary>Hint: If scores are not saved (no error, but nothing appears in history)</summary>

Check the network tab in your browser's dev tools. The `submit_wod_scores` request should return a log ID. If it returns an error, check:
1. Is the user authenticated? (`require_auth()` must succeed)
2. Is the date valid? (must be YYYY-MM-DD format)
3. Is the JSON valid? (`serde_json::from_str` must succeed)

A common issue is the date format. HTML date inputs produce `"2026-03-20"`, which is correct. But if the date signal is empty or the input was not filled, the validation will fail.

Also verify that the `scores_json` string is not empty — if all section states have empty fields, `filter_map` may produce an empty vec, and `serde_json::to_string` of an empty vec is `"[]"`, which is valid JSON but inserts no section logs.

</details>

---

## Rust Gym

### Drill 1: Define a Trait

Define a `Scorable` trait with a method `score_value(&self) -> Option<i32>`. Implement it for `ForTimeScore`, `AmrapScore`, and `StrengthScore` structs.

```rust
struct ForTimeScore { seconds: i32 }
struct AmrapScore { rounds: i32, extra_reps: i32 }
struct StrengthScore { weight_kg: f32 }

// Define the trait and implement it for each struct
```

<details>
<summary>Solution</summary>

```rust
trait Scorable {
    fn score_value(&self) -> Option<i32>;
}

impl Scorable for ForTimeScore {
    fn score_value(&self) -> Option<i32> {
        if self.seconds > 0 { Some(self.seconds) } else { None }
    }
}

impl Scorable for AmrapScore {
    fn score_value(&self) -> Option<i32> {
        Some(self.rounds * 1000 + self.extra_reps)
    }
}

impl Scorable for StrengthScore {
    fn score_value(&self) -> Option<i32> {
        Some((self.weight_kg * 100.0) as i32)
    }
}

// Now you can write generic code:
fn display_score(score: &impl Scorable) {
    match score.score_value() {
        Some(v) => println!("Score: {}", v),
        None => println!("No score"),
    }
}
```

The `&impl Scorable` syntax means "any reference to a type that implements Scorable." The compiler generates a specialized version of `display_score` for each concrete type — there is no runtime dispatch.

</details>

### Drill 2: Implement a Trait for Multiple Types

Given a `Displayable` trait with `fn label(&self) -> String`, implement it for both `WodSection` and `WodMovement`.

```rust
trait Displayable {
    fn label(&self) -> String;
}

// WodSection label: phase + title (e.g., "Warm-Up: Mobility")
// WodMovement label: exercise_name + rep_scheme (e.g., "Back Squat 5x5")
```

<details>
<summary>Solution</summary>

```rust
impl Displayable for WodSection {
    fn label(&self) -> String {
        let phase = phase_label(&self.phase);
        match &self.title {
            Some(t) if !t.is_empty() => format!("{}: {}", phase, t),
            _ => phase.to_string(),
        }
    }
}

impl Displayable for WodMovement {
    fn label(&self) -> String {
        match &self.rep_scheme {
            Some(reps) if !reps.is_empty() => {
                format!("{} {}", self.exercise_name, reps)
            }
            _ => self.exercise_name.clone(),
        }
    }
}

// Both types can now be used with generic code:
fn print_items(items: &[impl Displayable]) {
    for item in items {
        println!("{}", item.label());
    }
}
```

</details>

### Drill 3: Derive Macros

Which of these derive combinations will compile? For each that fails, explain why.

```rust
// A
#[derive(Clone, Serialize)]
struct Scores { data: Vec<i32> }

// B
#[derive(Clone, Serialize)]
struct Wrapper { data: std::sync::Mutex<String> }

// C
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
struct WorkoutLog {
    pub id: String,
    pub notes: Option<String>,
}
```

<details>
<summary>Solution</summary>

**A: Compiles.** `Vec<i32>` implements both `Clone` and `Serialize`. All field types must implement the derived trait for the derive to work.

**B: Does NOT compile.** `Mutex<String>` does not implement `Clone` (cloning a mutex does not make sense — which thread owns the clone?). It also does not implement `Serialize` (what does it mean to serialize a lock?). Both derives fail.

**C: Compiles.** In both feature configurations:
- Without `ssr`: derives `Clone, Debug, Serialize, Deserialize` — all implemented for `String` and `Option<String>`.
- With `ssr`: additionally derives `sqlx::FromRow` — which requires the field types to be extractable from a database row. `String` and `Option<String>` both work.

The rule: a derive macro generates `impl Trait for YourStruct` by requiring each field's type to also implement the trait. If any field does not implement it, the derive fails with a (sometimes cryptic) error pointing at the field.

</details>

---

## DSA in Context: The Strategy Pattern

Different section types use different scoring strategies. This is the **Strategy Pattern** — defining a family of algorithms, encapsulating each one, and making them interchangeable.

In object-oriented languages, you would create a `ScoringStrategy` interface with `ForTimeStrategy`, `AmrapStrategy`, etc. In Rust, you can use either traits or match expressions:

**Trait-based (when you need extensibility):**

```rust
trait ScoringStrategy {
    fn compute_score(&self, input: &SectionScoreInput) -> Option<i32>;
    fn render_inputs(&self) -> impl IntoView;
}
```

**Match-based (when types are known, as in GrindIt):**

```rust
let score = match section_type {
    "fortime" => input.finish_time_seconds,
    "amrap" => Some(rounds * 1000 + extra_reps),
    "strength" => input.weight_kg.map(|w| (w * 100.0) as i32),
    _ => None,
};
```

GrindIt uses the match-based approach because:
1. The section types are fixed (defined as a PostgreSQL enum)
2. Adding a new type requires database migration + scoring logic + UI changes — all tightly coupled
3. The match expression is exhaustive — the compiler catches missing types

The trait-based approach would be better if section types were pluggable (e.g., gym owners could define custom scoring algorithms). But for GrindIt's fixed set of types, the match is simpler and equally safe.

---

## System Design Corner: Form State Management

### The problem

A workout score form has many inputs across multiple sections and movements. Each input needs:
- A reactive signal for the current value
- Validation (is this a valid number?)
- Aggregation (collect all values into a submission payload)
- Feedback (show errors, pending state, success)

### GrindIt's approach: signals in structs

GrindIt puts `RwSignal` values inside state structs. Each section gets a `SectionScoreState` with signals for every input. The submit handler reads all signals with `get_untracked()` and assembles the payload.

**Pros:**
- Each input is independently reactive (typing in one does not re-render others)
- State is colocated with the component that manages it
- No global form state to synchronize

**Cons:**
- Many signal allocations (one per input per section per movement)
- Aggregation requires iterating all states manually
- No built-in validation framework

### Alternative: JSON serialization

GrindIt serializes the entire score payload as a JSON string before sending it to the server function. This is simpler than defining a server function with dozens of parameters:

```rust
// Instead of:
submit_score(section_id: String, minutes: i32, seconds: i32, rounds: i32, ...)

// GrindIt uses:
submit_wod_scores(wod_id: String, date: String, notes: String, scores_json: String)
```

The JSON string carries the entire tree of scores. On the server, `serde_json::from_str` deserializes it back into typed structs. This "serialize on client, deserialize on server" pattern is common in Leptos applications when the data structure is complex or variable.

### Optimistic updates vs. pending states

GrindIt uses **pending states** (not optimistic updates) for score submission:
1. User clicks "Log Score"
2. Button shows "Submitting..."
3. Server processes the request
4. On success: button shows "Saved!" and navigates to history after 800ms
5. On error: inline error message appears

Optimistic updates (showing the score as saved immediately, before the server confirms) would be faster but riskier — if the server rejects the score, you would need to roll back the UI. For score submission, where accuracy matters more than speed, pending states are the right choice.

---

## Design Insight: Complexity Budget

Every feature has a complexity cost. The workout scoring system is one of the most complex features in GrindIt:

- **4 database tables** with nested foreign keys
- **3 levels of input structs** (section -> movement -> set)
- **5 section types** with different scoring UIs
- **6+ exercise scoring types** (reps/weight, distance, calories, time)
- **JSON serialization** for the client-server bridge
- **Idempotent submission** (append to existing log if one exists)

This complexity is inherent — it comes from the problem domain, not from poor design choices. CrossFit scoring IS complex. A "For Time" workout is scored differently from an "AMRAP" workout, and both are different from a "Strength" block. Each movement might have different metrics (weight vs. distance vs. calories).

The key architectural decision is *where to put the complexity*. GrindIt puts it in three places:
1. **State structs** — `SectionScoreState` and `MovementLogState` hold all reactive state, keeping the component code focused on rendering
2. **The submit handler** — one function collects all state into a typed payload
3. **The database layer** — nested inserts that mirror the data structure

Each layer has one job. The UI components do not know about database tables. The submit handler does not know about signals. The database layer does not know about React-like rendering. The complexity is real but organized.

---

## What You Built

In this chapter, you:

1. **Created scoring tables** — `section_logs` and `movement_logs` (and `movement_log_sets`) with the full scoring hierarchy and computed `score_value` for leaderboards
2. **Built the WodScoreForm** with a WOD picker, tab switcher, and per-section rendering
3. **Built the SectionScoreCard** with type-dependent inputs (time, rounds, weight), RX toggle, and per-movement tracking with lazy loading
4. **Implemented submit_wod_scores** — client-side aggregation, JSON serialization, and server-side nested inserts across four tables
5. **Practiced traits and generics** — `Serialize`/`Deserialize` derives, `impl IntoView`, `impl Fn()` callbacks, and `Resource::new()`

Athletes can now score WODs with detailed per-section and per-movement results. In Chapter 10, we will build the history page that displays these scores on a timeline, and the leaderboard that ranks athletes per section.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter has two data structure deep dives:

The first builds the Strategy pattern using traits — making scoring types extensible without editing existing code.

**→ [Strategy Scoring](../ds-narratives/ch09-strategy-scoring.md)**

The second builds a Queue (ring buffer) and Priority Queue for competition day score processing.

**→ [Queue Scoring](../ds-narratives/ch09-queue-scoring.md)**

When you write `Box<dyn ScoringStrategy>`, what does the compiler actually generate? This deep dive opens the vtable — fat pointers, function pointer tables, object safety rules, and when to choose static vs dynamic dispatch.

**→ [Trait Objects & Vtables — "The Workout Card and Its Instruction Sheet"](../ds-narratives/ch09-trait-objects-vtable.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| Scoring migrations | [`migrations/20260311120009_rework_workout_logs.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120009_rework_workout_logs.sql), [`..._010_create_section_logs.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120010_create_section_logs_rework_workout_exercises.sql), [`..._create_movement_logs.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260314120003_create_movement_logs_table.sql) |
| `src/pages/log_workout/mod.rs` | [`src/pages/log_workout/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/mod.rs) — `LogWorkoutPage` with tabs and `WodScoreFlow` |
| `src/pages/log_workout/wod_score_form.rs` | [`src/pages/log_workout/wod_score_form.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/wod_score_form.rs) — `WodScoreForm` with submission logic |
| `src/pages/log_workout/section_score_card.rs` | [`src/pages/log_workout/section_score_card.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/section_score_card.rs) — `SectionScoreCard` with type-dependent inputs |
| `src/pages/log_workout/exercise_entry_card.rs` | [`src/pages/log_workout/exercise_entry_card.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/exercise_entry_card.rs) — per-exercise card with sets |
| `src/pages/log_workout/server_fns.rs` | [`src/pages/log_workout/server_fns.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/server_fns.rs) — `submit_wod_scores`, `get_wod_for_scoring`, `get_todays_wods` |
| `src/db.rs` (scoring structs) | [`src/db.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/db.rs) — `SectionScoreInput`, `MovementLogInput`, `MovementLogSetInput`, `SectionLog`, `MovementLog` |
