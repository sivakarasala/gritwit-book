# Chapter 9: Workout Logging & Scoring

Athletes need to record what they did. A WOD without scores is just a whiteboard; a WOD with scores becomes training data --- PRs, progressions, leaderboards, and accountability. This chapter builds the workout logging system: athletes select a WOD, score each section (time, rounds, weight), log per-movement results, and submit everything to the database.

The spotlight concept is **traits and generics** --- the mechanisms Rust uses to write code that works across multiple types. You have been using traits all along (every `#[derive(Serialize)]` is a trait implementation), and this chapter makes the concept explicit. You will see how traits define shared behavior, how generics let you write code that adapts to any type, and why `impl IntoView` appears on every Leptos component.

By the end of this chapter, you will have:

- Database tables for `workout_logs`, `section_logs`, and `movement_logs` with the full scoring hierarchy
- Scoring structs (`SectionScoreInput`, `MovementLogInput`, `MovementLogSetInput`) with serde derives
- A `WodScoreForm` that loads a WOD's sections and renders per-section scoring cards
- A `SectionScoreCard` with time/rounds/weight inputs, RX toggle, and per-movement tracking
- A `submit_wod_scores` server function that inserts the workout log, section logs, and movement logs in a single transaction
- Tabbed navigation between WOD Score and Custom Log modes

---

## Spotlight: Traits & Generics

### The problem: different things that behave similarly

Consider scoring. A "For Time" section is scored in seconds. An "AMRAP" section is scored in rounds + extra reps. A "Strength" section is scored in kilograms. These are fundamentally different data types, but they share a common need: they all need to produce a comparable score value for the leaderboard.

How do you write code that works with all of them?

> **Programming Concept: What is a Trait?**
>
> A trait is a promise that a type can do certain things. Think of it like a job qualification:
>
> - A "Driver" qualification means you can operate a vehicle
> - A "Chef" qualification means you can cook food
> - A "Lifeguard" qualification means you can swim and rescue people
>
> A person can have multiple qualifications. A type can implement multiple traits.
>
> In Rust, a trait defines one or more methods that any type can implement. The `Serialize` trait says: "I can convert myself to a portable format." The `Display` trait says: "I can show myself as text." The `Clone` trait says: "I can make a copy of myself."
>
> When you write `#[derive(Clone, Debug, Serialize)]`, you are telling the compiler: "automatically implement the Clone, Debug, and Serialize traits for this type." The compiler generates the implementation code for you.

### Traits you have already been using

You have been using traits since Chapter 2 without realizing it:

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

That `#[derive(Serialize, Deserialize)]` line generates hundreds of lines of implementation code. The `Serialize` trait knows how to convert this struct to JSON because every field's type (`String`, `Option<i32>`, `bool`, `Vec<MovementLogInput>`) also implements `Serialize`. The derive macro walks the struct recursively --- if `MovementLogInput` has nested types, they must implement `Serialize` too.

This is why GrindIt marks every struct that crosses the client-server boundary with `Serialize, Deserialize`. Server functions serialize their arguments to JSON on the client, send them over HTTP, and deserialize them on the server. The return value follows the reverse path.

> **Programming Concept: What are Generics?**
>
> Generics are code that works with many types. Think of a vending machine: it does not care whether it dispenses chips, candy, or drinks. The mechanism (insert coin, press button, item drops) is the same regardless of what is inside.
>
> In code, generics look like this:
>
> ```rust
> fn first_item<T>(list: &[T]) -> Option<&T> {
>     list.first()
> }
> ```
>
> The `<T>` means "this function works with any type T." You can call `first_item(&vec_of_strings)` or `first_item(&vec_of_numbers)` --- the same function works for both. The compiler generates a specialized version for each concrete type you use.
>
> Generics with traits are even more powerful:
>
> ```rust
> fn print_it<T: Display>(item: &T) {
>     println!("{}", item);
> }
> ```
>
> The `T: Display` means "T can be any type, as long as it implements the Display trait." This is like saying "the vending machine works with any item, as long as it fits through the slot."

### `impl IntoView` --- the universal Leptos return type

Every Leptos component returns `impl IntoView`:

```rust
#[component]
pub fn SectionScoreCard(state: SectionScoreState, focused: bool) -> impl IntoView {
    // ... can return view!, ().into_view(), String, etc.
}
```

`impl IntoView` means "some type that implements the `IntoView` trait." The caller does not know the concrete type --- it could be an HTML div, a fragment, a string, or any other renderable element. The compiler figures out the concrete type and optimizes accordingly.

This is different from JavaScript's `React.ReactNode`, which is a runtime union type. In Rust, `impl IntoView` is resolved at compile time --- zero runtime cost for the abstraction.

### `.into_any()` --- when branches return different types

You have seen this pattern before:

```rust
if condition {
    view! { <div>"A"</div> }.into_any()
} else {
    view! { <span>"B"</span> }.into_any()
}
```

The two branches return different concrete types (`HtmlDiv` vs `HtmlSpan`). In JavaScript, both branches just return "JSX" and that is fine. In Rust, the `if/else` expression must return the same type from both branches. `.into_any()` erases the concrete type and returns a common `AnyView` type. Think of it as putting different-shaped items into the same type of box.

### Callback props: `impl Fn() + Copy + 'static`

When a parent passes an event handler to a child:

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

Let us decode `impl Fn() + Copy + 'static`:

- **`Fn()`** --- it is a callable function that takes no arguments (like clicking a button)
- **`Copy`** --- it can be duplicated cheaply (signals are `Copy`, so closures over signals are too)
- **`'static`** --- it does not borrow any short-lived data (it owns everything it needs)

In GrindIt, most callback props are closures that capture `RwSignal`s. Signals are `Copy` (they are just lightweight handles), so closures over them are also `Copy`. This is why Leptos works so well with closures.

### `Resource::new()` --- generic async data loading

```rust
let wods = Resource::new(
    move || (selected_date.get(), create_action.version().get()),
    |(date, _)| list_wods_for_date(date),
);
```

`Resource::new` is generic over three things:
1. The **source** type (the tuple of signals that trigger refetching)
2. The **future** type (the async function that loads data)
3. The **result** type (what the future returns)

The compiler infers all three from how you use them. You never write `Resource::<(String, usize), _, Vec<Wod>>::new(...)` --- the types flow from the closure bodies. This is type inference at work.

> **Programming Concept: What is Serialization?**
>
> Serialization is converting data into a format that can be sent somewhere else. Think of packing a suitcase for travel:
>
> - **Serialize** = pack your clothes neatly into the suitcase (convert struct to JSON)
> - **Deserialize** = unpack the suitcase at your destination (convert JSON back to struct)
>
> When you click "Log Score" in GrindIt:
> 1. The client **serializes** your scores into JSON text
> 2. The JSON travels over HTTP to the server
> 3. The server **deserializes** the JSON back into typed Rust structs
> 4. The server processes the data and saves it to the database
>
> Without serialization, the structured data in your browser (structs with typed fields) could not cross the network to the server. The network only understands bytes --- serialization is the translation layer.

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
> The biggest difference: TypeScript generics are erased at runtime --- they only exist during type checking. Rust generics are **monomorphized** --- the compiler generates specialized code for each concrete type. `Resource<(String, usize), Vec<Wod>>` and `Resource<String, AuthUser>` become two different types with two different compiled code paths. Zero runtime overhead.

---

## Exercise 1: Define the Scoring Structs

**Goal:** Create the database tables and Rust structs for workout logs, section scores, and movement logs.

### Step 1: The workout_logs table

This table already exists from Chapter 5 (basic logging), but we extend it:

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

Each `section_log` records one athlete's result for one section. The scoring fields are polymorphic --- different section types use different fields:

| Section Type | Primary Score Field | Secondary Fields |
|-------------|-------------------|-----------------|
| For Time | `finish_time_seconds` | `is_rx` |
| AMRAP | `rounds_completed` | `extra_reps`, `is_rx` |
| EMOM | `rounds_completed` | `extra_reps` |
| Strength | `weight_kg` | |
| Static | (none) | `notes`, `skipped` |

The `score_value` column stores a **precomputed integer** for leaderboard ranking. For Time scores store seconds. AMRAP scores store `rounds * 1000 + extra_reps`. This avoids recomputing the comparison logic every time the leaderboard loads.

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

And for set-by-set tracking (e.g., "Set 1: 5 reps at 100kg, Set 2: 5 reps at 105kg"):

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

This is the deepest level of the scoring hierarchy:

```
workout_log          (one per WOD attempt)
  +-- section_log    (one per section)
      +-- movement_log    (one per movement)
          +-- movement_log_set (one per set of that movement)
```

A single "Log Score" button press creates records across all four tables.

### Step 4: The Rust input structs

```rust
// src/db.rs --- structs for submitting scores

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

The `#[serde(default)]` attribute on `movement_logs` and `set_details` means: if the field is missing in the JSON, use the default value (`Vec::new()` for `Vec`). This is important because simpler submissions might not include movement-level data.

Notice that these are **input** structs --- they carry data from the client to the server. The **output** structs (for reading back from the database) are different:

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

The input struct does not have `id`, `workout_log_id`, or `score_value` --- those are generated by the server. The output struct does not have `movement_logs` --- those are loaded separately. This separation of input/output types is a common pattern in Rust web applications.

<details>
<summary>Hint: If serde deserialization fails with "missing field movement_logs"</summary>

Add `#[serde(default)]` to the `movement_logs` and `set_details` fields. Without this attribute, serde requires the field to be present in the JSON.

</details>

---

## Exercise 2: Build the WodScoreForm

**Goal:** A form that loads a WOD's sections and renders a scoring card for each one, with a submit button that sends all scores to the server.

> **Programming Concept: What is a Server Action?**
>
> A server action is a form submission that calls a server function and handles the loading, error, and success states for you. Think of it like ordering food at a restaurant:
>
> 1. You place your order (dispatch the action)
> 2. You see a "preparing" indicator (the action is pending)
> 3. Your food arrives (the action succeeds) or the waiter tells you it is unavailable (the action fails)
>
> In Leptos, `ServerAction` wraps a `#[server]` function and gives you:
> - `.pending()` --- is the request in flight?
> - `.value()` --- what did the server return?
> - `.version()` --- how many times has this action completed?
>
> You can use `.pending()` to show "Submitting..." on a button, and `.value()` to check for errors after the response arrives.

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

The page reads query parameters to determine which WOD to score. Navigating from a WOD card (`/log?wod_id=abc-123`) pre-selects the WOD. The two `if` branches return different types, so `.into_any()` is needed on both.

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
1. **WOD loaded** --- show the scoring form
2. **No WOD selected, today has WODs** --- show a picker
3. **No WOD selected, no WODs today** --- show empty state

Each state returns a different view, so all paths need `.into_any()`.

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

The `SectionScoreState` struct holds all reactive state for scoring one section. It mixes non-reactive data (`section_id`, `section_type`, `phase`) with reactive data (`is_rx`, `minutes`, `seconds`). The non-reactive fields identify the section; the reactive fields capture user input.

<details>
<summary>Hint: If sections appear empty with no movement tracking</summary>

The movement states are loaded lazily --- the `SectionScoreCard` component fetches movements for its section when it mounts. Check the browser console for server function errors.

Also verify that movements were added to the sections in Chapter 8. A section without movements is valid (e.g., a "Static" cool-down), but most scoring sections should have movements.

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

The `MovementLogState` adapts to the exercise's scoring type. A weightlifting movement shows reps/weight inputs. A conditioning movement (like rowing) shows calories or distance. The `scoring_type` field (from the exercise library) determines which inputs appear.

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

The `match` on `section_type.as_str()` is the key pattern here. Each section type has its own scoring UI:

- **For Time** shows minutes:seconds inputs
- **AMRAP/EMOM** shows rounds + extra reps
- **Strength** shows weight in kg
- **Static** (the `_` catch-all) shows nothing --- just skip/notes

This is the **strategy pattern** implemented with a `match` expression. In an object-oriented language, you would create separate classes for each scoring strategy. In Rust, a `match` achieves the same thing more concisely.

The movement states are loaded **lazily** via a `Resource` and initialized in an `Effect`. The section card appears immediately with the scoring inputs, and the movement tracking rows appear once the data arrives from the server.

<details>
<summary>Hint: If the RX toggle does not switch</summary>

The two buttons set explicit values: one sets `true`, the other sets `false`. They are NOT toggles. Check that `class:active` is bound to the correct condition: the RX button should be active when `is_rx.get()` is `true`, and the Scaled button when `is_rx.get()` is `false`.

</details>

---

## Exercise 4: Implement the Submit Server Function

**Goal:** A server function that receives the scoring data as JSON, validates it, and inserts records across multiple tables.

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

Let us trace through what happens when the user clicks "Log Score":

1. **Set loading state**: `submitting.set(true)` makes the button show "Submitting..."
2. **Collect all scores**: We iterate through every section state, reading each signal with `get_untracked()` (we do not want subscriptions here), and building `SectionScoreInput` structs
3. **Serialize to JSON**: The entire batch becomes a single JSON string
4. **Send to server**: `spawn_local` runs the async call without blocking the UI
5. **Handle result**: On success, show "Saved!" and navigate to history after 800ms. On error, show the error message.

The `filter_map` on movement logs is clever: it only includes movements where the user actually entered data. If you left a movement's fields blank, it is skipped entirely.

**`spawn_local`** runs an async block on the client without blocking the UI. This is Leptos's equivalent of a Promise --- the function returns immediately, and the result is handled in the `match` block when the server responds.

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

The server function:
1. **Requires authentication** --- you must be logged in to log scores
2. **Validates the date** --- prevents malformed date strings
3. **Deserializes the JSON** --- converts the string back into typed structs
4. **Handles idempotency** --- if you already logged this WOD today, it appends instead of duplicating

### Step 3: The database layer

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

The nested insert pattern mirrors the data hierarchy: each level creates a row and returns its ID, which becomes the foreign key for the next level down. Think of it as filling out a multi-page form: page 1 (workout log) gets a number, page 2 (section log) references page 1's number, page 3 (movement log) references page 2's number.

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
1. Is the user authenticated?
2. Is the date valid? (must be YYYY-MM-DD format)
3. Is the JSON valid? (check `scores_json` is not empty)

A common issue: if all section states have empty fields, `filter_map` may produce an empty vec, and the server inserts no section logs.

</details>

---

## Rust Gym

### Drill 1: Define a Simple Trait

Define a `Scorable` trait with a method `score_value(&self) -> Option<i32>`. Implement it for three structs: `ForTimeScore`, `AmrapScore`, and `StrengthScore`.

```rust
struct ForTimeScore { seconds: i32 }
struct AmrapScore { rounds: i32, extra_reps: i32 }
struct StrengthScore { weight_kg: f32 }

// Define the trait and implement it for each struct
```

<details>
<summary>Hint</summary>

The trait declaration looks like: `trait Scorable { fn score_value(&self) -> Option<i32>; }`. Then write `impl Scorable for ForTimeScore { ... }` for each struct.

</details>

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

The `&impl Scorable` syntax means "any reference to a type that implements Scorable." The compiler generates a specialized version of `display_score` for each concrete type --- no runtime dispatch.

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
<summary>Hint</summary>

Use `match` on the `Option<String>` title/rep_scheme field. Check if it is `Some` and not empty before including it.

</details>

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

The `&[impl Displayable]` parameter accepts a slice of any type that implements `Displayable`. You can pass `&sections` or `&movements` --- the same function works for both.

</details>

### Drill 3: Derive Macros --- What Compiles?

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
<summary>Hint</summary>

A derive macro generates `impl Trait for YourStruct` by requiring EVERY field's type to also implement that trait. If any field cannot implement it, the derive fails.

</details>

<details>
<summary>Solution</summary>

**A: Compiles.** `Vec<i32>` implements both `Clone` and `Serialize`.

**B: Does NOT compile.** `Mutex<String>` does not implement `Clone` (cloning a mutex makes no sense --- which thread owns the clone?). It also does not implement `Serialize` (what does it mean to serialize a lock?). Both derives fail.

**C: Compiles.** In both feature configurations:
- Without `ssr`: derives `Clone, Debug, Serialize, Deserialize` --- all implemented for `String` and `Option<String>`.
- With `ssr`: additionally derives `sqlx::FromRow` --- which requires field types to be extractable from a database row. `String` and `Option<String>` both work.

The rule: every field type must implement the trait being derived. If even one field does not, the derive fails.

</details>

---

## DSA in Context: The Strategy Pattern

Different section types use different scoring strategies. This is the **Strategy Pattern** --- defining a family of algorithms and making them interchangeable.

In GrindIt, we use a `match` expression instead of separate strategy classes:

```rust
let score = match section_type {
    "fortime" => input.finish_time_seconds,
    "amrap" => Some(rounds * 1000 + extra_reps),
    "strength" => input.weight_kg.map(|w| (w * 100.0) as i32),
    _ => None,
};
```

Why `match` instead of traits here? Because:
1. The section types are fixed (defined as a PostgreSQL enum)
2. Adding a new type requires database migration + scoring logic + UI changes --- all tightly coupled
3. The `match` is exhaustive --- the compiler catches missing types

A trait-based approach would be better if section types were extensible (e.g., gym owners could define custom scoring). For GrindIt's fixed set of types, `match` is simpler and equally safe.

---

## System Design Corner: Form State Management

### The problem

A workout score form has many inputs across multiple sections and movements. Each input needs a reactive signal, validation, aggregation into a payload, and feedback (errors, loading, success).

### GrindIt's approach: signals in structs

Each section gets a `SectionScoreState` with signals for every input. The submit handler reads all signals with `get_untracked()` and assembles the payload.

**Pros:** Each input is independently reactive (typing in one does not re-render others), state is colocated with the component.

**Cons:** Many signal allocations, manual aggregation.

### JSON serialization for the client-server bridge

GrindIt serializes the entire score payload as a JSON string:

```rust
// Instead of dozens of separate parameters:
submit_score(section_id: String, minutes: i32, seconds: i32, rounds: i32, ...)

// One JSON string carries everything:
submit_wod_scores(wod_id: String, date: String, notes: String, scores_json: String)
```

On the server, `serde_json::from_str` converts it back to typed structs. This "serialize on client, deserialize on server" pattern is common in Leptos when the data structure is complex.

---

## Design Insight: Complexity Budget

The workout scoring system is one of the most complex features in GrindIt:

- **4 database tables** with nested foreign keys
- **3 levels of input structs** (section -> movement -> set)
- **5 section types** with different scoring UIs
- **JSON serialization** for the client-server bridge
- **Idempotent submission** (append to existing log)

This complexity is **inherent** --- it comes from the problem domain, not from poor design. CrossFit scoring IS complex. The key architectural decision is WHERE to put the complexity:

1. **State structs** hold all reactive state, keeping components focused on rendering
2. **The submit handler** collects all state into a typed payload
3. **The database layer** handles nested inserts

Each layer has one job. The complexity is real but organized.

---

## What You Built

In this chapter, you:

1. **Created scoring tables** --- `section_logs` and `movement_logs` (and `movement_log_sets`) with the full scoring hierarchy and computed `score_value` for leaderboards
2. **Built the WodScoreForm** with a WOD picker, tab switcher, and per-section rendering
3. **Built the SectionScoreCard** with type-dependent inputs (time, rounds, weight), RX toggle, and per-movement tracking with lazy loading
4. **Implemented submit_wod_scores** --- client-side aggregation, JSON serialization, and server-side nested inserts across four tables
5. **Practiced traits and generics** --- `Serialize`/`Deserialize` derives, `impl IntoView`, `impl Fn()` callbacks, and `Resource::new()`

Athletes can now score WODs with detailed per-section and per-movement results. In Chapter 10, we will build the history page that displays these scores on a timeline, and the leaderboard that ranks athletes per section.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter has two data structure deep dives.

The first builds the Strategy pattern using traits — making scoring types extensible without editing existing code from scratch in Rust — no libraries, just std.

**→ [Strategy Scoring](../ds-narratives/ch09-strategy-scoring.md)**

The second builds a Queue (ring buffer) and Priority Queue for competition day score processing from scratch in Rust — no libraries, just std.

**→ [Queue Scoring](../ds-narratives/ch09-queue-scoring.md)**

When you write `Box<dyn ScoringStrategy>`, what does the compiler actually generate? This deep dive opens the vtable — fat pointers, function pointer tables, object safety rules, and when to choose static vs dynamic dispatch.

**→ [Trait Objects & Vtables — "The Workout Card and Its Instruction Sheet"](../ds-narratives/ch09-trait-objects-vtable.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| Scoring migrations | [`migrations/20260311120009_rework_workout_logs.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120009_rework_workout_logs.sql), [`..._010_create_section_logs.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120010_create_section_logs_rework_workout_exercises.sql), [`..._create_movement_logs.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260314120003_create_movement_logs_table.sql) |
| `src/pages/log_workout/mod.rs` | [`src/pages/log_workout/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/mod.rs) --- `LogWorkoutPage` with tabs and `WodScoreFlow` |
| `src/pages/log_workout/wod_score_form.rs` | [`src/pages/log_workout/wod_score_form.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/wod_score_form.rs) --- `WodScoreForm` with submission logic |
| `src/pages/log_workout/section_score_card.rs` | [`src/pages/log_workout/section_score_card.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/section_score_card.rs) --- `SectionScoreCard` with type-dependent inputs |
| `src/pages/log_workout/exercise_entry_card.rs` | [`src/pages/log_workout/exercise_entry_card.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/exercise_entry_card.rs) --- per-exercise card with sets |
| `src/pages/log_workout/server_fns.rs` | [`src/pages/log_workout/server_fns.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/log_workout/server_fns.rs) --- `submit_wod_scores`, `get_wod_for_scoring`, `get_todays_wods` |
| `src/db.rs` (scoring structs) | [`src/db.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/db.rs) --- `SectionScoreInput`, `MovementLogInput`, `MovementLogSetInput`, `SectionLog`, `MovementLog` |
