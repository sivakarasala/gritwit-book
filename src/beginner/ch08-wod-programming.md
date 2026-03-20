# Chapter 8: WOD Programming

A workout-of-the-day (WOD) is more than a title and a description. A real CrossFit WOD has structure: a warm-up section with mobility drills, a strength block with barbell movements, a conditioning piece with a time cap, and maybe a cool-down. Each section has its own type (AMRAP, For Time, EMOM, Strength), and each section contains specific movements with prescribed reps and weights.

This chapter builds the data model and UI for this nested structure. You will create database tables that reference each other, Rust structs that mirror those tables, a weekly calendar that works without any date library, and a WOD creation form that only coaches can use.

The spotlight concept is **complex data structures and relationships** --- nested structs, multi-table database schemas with foreign keys, and the patterns for managing tree-like data in both Rust and SQL.

By the end of this chapter, you will have:

- Database migrations for `wods`, `wod_sections`, and `wod_movements` tables with correct foreign keys and cascading deletes
- Rust structs (`Wod`, `WodSection`, `WodMovement`) with serde serialization and conditional `sqlx::FromRow` derives
- A weekly calendar component with date arithmetic using integer-only calculations (no `chrono` on the client)
- A WOD creation form with workout type selection and date picker
- A WOD card that expands to show sections and movements, with edit/delete for owners
- Server functions with `require_role(Coach)` guards for all mutations

---

## Spotlight: Complex Data Structures & Relationships

### From flat to nested

In Chapters 2--6, our data was flat: an `Exercise` has a name, category, and scoring type. One struct, one table. WODs are different --- they form a tree:

```
Wod
+-- WodSection (Warm-Up, Strength)
|   +-- WodMovement (Back Squat 5x5 @ 80%)
|   +-- WodMovement (Romanian Deadlift 3x10)
+-- WodSection (Conditioning, AMRAP 12min)
|   +-- WodMovement (Thrusters 15 reps @ 43kg)
|   +-- WodMovement (Pull-ups 12 reps)
|   +-- WodMovement (Box Jumps 9 reps)
+-- WodSection (Cool Down, Static)
    +-- WodMovement (Foam Roll)
```

> **Programming Concept: What are Nested Data Structures?**
>
> Nested data structures are data inside data, like Russian nesting dolls. Think about how a school is organized:
>
> - A **school** has departments
> - Each **department** has classes
> - Each **class** has students
>
> You cannot describe a student without knowing which class they are in, and you cannot describe a class without knowing which department it belongs to. The data has levels.
>
> Our WOD works the same way:
> - A **WOD** has sections
> - Each **section** has movements
> - Each **movement** references an exercise from the library
>
> In JavaScript, you would model this as nested objects (objects inside arrays inside objects). In Rust, you model it as separate structs connected by ID fields. The database stores them as separate tables connected by foreign keys.

In JavaScript, you would write this as nested objects:

```javascript
const wod = {
  title: "Monday Grind",
  sections: [
    {
      phase: "warmup",
      movements: [
        { exercise: "Back Squat", reps: "5x5", weight: 80 },
      ]
    }
  ]
};
```

This works but has no guarantees. Nothing prevents `sections` from being a string, or `movements` from containing a number, or `phase` from being `"warmup"` in one place and `"warm-up"` in another.

### Nested structs in Rust

Rust enforces the structure at compile time:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Wod {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub workout_type: String,
    pub time_cap_minutes: Option<i32>,
    pub programmed_date: String,
    pub created_by: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct WodSection {
    pub id: String,
    pub wod_id: String,
    pub phase: String,
    pub title: Option<String>,
    pub section_type: String,
    pub time_cap_minutes: Option<i32>,
    pub rounds: Option<i32>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct WodMovement {
    pub id: String,
    pub section_id: String,
    pub exercise_id: String,
    pub exercise_name: String,
    pub rep_scheme: Option<String>,
    pub weight_kg_male: Option<f32>,
    pub weight_kg_female: Option<f32>,
    pub notes: Option<String>,
    pub sort_order: i32,
    pub scoring_type: String,
}
```

Let us walk through the key observations:

- **`Option<T>` for nullable fields.** A WOD's description might be empty. A section's time cap might not apply (strength sections do not have time caps). Rust forces you to handle both cases wherever you use these fields. You cannot accidentally do `wod.description.len()` --- the compiler says "that is an `Option`, not a `String`."
- **`#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]`** --- the `FromRow` derive only applies on the server, where sqlx queries run. On the client (WASM), sqlx does not exist, so this derive is skipped.
- **IDs as `String`, not `Uuid`.** Across the client-server boundary, UUIDs are serialized as strings anyway. Using `String` avoids needing the `uuid` crate in the WASM bundle.
- **Foreign keys as fields.** `WodSection.wod_id` links a section to its parent WOD. `WodMovement.section_id` links a movement to its parent section. The database enforces that these references are valid; the Rust structs express the relationship in code.

> **Programming Concept: What is a Foreign Key?**
>
> A foreign key is a link between database tables. Think of a student ID card: the card has a student number printed on it, and that number points back to the student's record in the school's main database.
>
> In our database:
> - Each `WodSection` has a `wod_id` field that contains the ID of the WOD it belongs to
> - Each `WodMovement` has a `section_id` field that contains the ID of the section it belongs to
>
> The database enforces these links. If you try to create a section with a `wod_id` that does not exist, the database will reject it. If you try to delete a WOD that still has sections, the database will either reject the delete or automatically remove the sections (depending on the CASCADE setting).
>
> This is referential integrity --- the guarantee that every reference actually points to something real. In JavaScript, nothing stops you from setting `{ wodId: "nonexistent-id" }`. In a database with foreign keys, that is an error.

### The relationship between structs and tables

To assemble the full tree (a WOD with all its sections and movements), you run multiple queries:

```rust
// Pseudocode for loading a full WOD
let wod = get_wod_by_id(&pool, wod_id).await?;
let sections = list_wod_sections(&pool, wod_id).await?;
let movements = get_all_wod_movements(&pool, wod_id).await?;
```

This is **normalization** --- the data lives in three tables, joined at query time. The alternative (denormalization) would store sections and movements as JSON arrays inside the wods table. But then you lose the ability to query sections independently, enforce foreign key constraints, or update a single movement without rewriting the entire WOD.

> **Coming from JS?**
>
> | Concept | JavaScript/JSON | Rust structs + SQL |
> |---------|----------------|-------------------|
> | Nested data | Inline objects `{ sections: [...] }` | Separate structs linked by ID fields |
> | Optional fields | `undefined` or missing key | `Option<T>` --- compiler-enforced |
> | Type safety | None (runtime shape) | Compile-time (wrong field type = error) |
> | Schema changes | Nothing enforces the new shape | Add/remove a struct field = compiler finds every call site |
> | Database mapping | ORM magic or manual | `#[derive(sqlx::FromRow)]` --- 1:1 column-to-field |
>
> The biggest difference: in JavaScript, you can pass `{ titel: "Fran" }` (typo) and nothing catches it until the UI shows undefined. In Rust, `Wod { titel: ... }` is a compile error --- the field does not exist.

---

## Exercise 1: Create the WOD Database Migrations

**Goal:** Build the three-table schema for WODs with correct foreign keys, enum types, and cascading deletes.

### Step 1: The wods table

```sql
-- migrations/XXXXXX_create_wods_table.sql
CREATE TABLE wods (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title            TEXT NOT NULL,
    description      TEXT,
    workout_type     TEXT NOT NULL DEFAULT 'fortime',
    time_cap_minutes INTEGER,
    programmed_date  DATE NOT NULL,
    created_by       UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Design decisions:

- **`workout_type` as TEXT**, not a PostgreSQL enum. We use text for flexibility --- adding a new type (like "tabata") does not require a migration. The validation happens in Rust code instead.
- **`created_by` with ON DELETE SET NULL.** If a user is deleted, their WODs remain but lose the creator reference. This is better than CASCADE (which would delete all WODs the user created) or RESTRICT (which would prevent user deletion entirely).
- **`programmed_date` as DATE**, not TIMESTAMPTZ. A WOD is programmed for a specific day, not a specific moment.

### Step 2: The wod_sections table

```sql
-- migrations/XXXXXX_add_enums_and_wod_sections.sql
CREATE TYPE wod_phase AS ENUM (
    'warmup', 'strength', 'conditioning', 'cooldown', 'optional', 'personal'
);
CREATE TYPE section_type AS ENUM (
    'fortime', 'amrap', 'emom', 'strength', 'static'
);

CREATE TABLE wod_sections (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wod_id           UUID NOT NULL REFERENCES wods(id) ON DELETE CASCADE,
    phase            wod_phase NOT NULL,
    title            TEXT,
    section_type     section_type NOT NULL DEFAULT 'static',
    time_cap_minutes INTEGER,
    rounds           INTEGER,
    notes            TEXT,
    sort_order       INTEGER NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_wod_sections_wod ON wod_sections (wod_id);
```

> **Programming Concept: What is a Cascade Delete?**
>
> Imagine you cancel a school field trip. What happens to all the permission slips that parents signed for that trip? They become meaningless --- there is no trip to give permission for.
>
> A cascade delete works the same way: when you delete a parent record, all its children are automatically deleted too.
>
> In our schema, `ON DELETE CASCADE` on `wod_id` means: when a WOD is deleted, all its sections are automatically deleted. And because sections have their own CASCADE relationship with movements, deleting a WOD automatically cleans up sections AND movements. No orphaned records, no manual cleanup.
>
> The alternative is `ON DELETE RESTRICT`, which would prevent you from deleting a WOD that still has sections. You would have to delete all sections first, then the WOD. This is safer but more tedious.

Here we *do* use PostgreSQL enums for `phase` and `section_type`. These are structural --- a section's phase determines how it is displayed (warm-up vs. conditioning), and the section type determines how it is scored (time vs. rounds vs. weight). Adding new values here requires a migration, which is intentional --- it forces us to update the scoring logic too.

### Step 3: The wod_movements table

```sql
-- migrations/XXXXXX_create_wod_movements.sql
CREATE TABLE wod_movements (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    section_id       UUID NOT NULL REFERENCES wod_sections(id) ON DELETE CASCADE,
    exercise_id      UUID NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    rep_scheme       TEXT,
    weight_kg_male   REAL,
    weight_kg_female REAL,
    notes            TEXT,
    sort_order       INTEGER NOT NULL DEFAULT 0,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_wod_movements_section ON wod_movements (section_id);
```

Key design choices:

- **`section_id`, not `wod_id`.** Movements belong to sections, not directly to WODs. This is the normalized tree structure. To find all movements in a WOD, you first find the WOD's sections, then find each section's movements.
- **`exercise_id` references the exercises table.** This links prescribed movements to the exercise library, enabling queries like "show me all WODs that include back squats."
- **`weight_kg_male` and `weight_kg_female`.** Prescribed weights differ by gender. The UI shows the appropriate weight based on the athlete's gender setting.
- **`rep_scheme` as TEXT**, not INTEGER. A rep scheme can be "21-15-9", "5x5", "AMRAP", or "Max effort" --- strings capture the variety that integers cannot.

### Step 4: Run the migrations

```bash
sqlx migrate run
```

<details>
<summary>Hint: If you see "relation exercises does not exist"</summary>

The `wod_movements` table references `exercises(id)`. If you have not run the exercises migration yet (from Chapter 5), the foreign key constraint fails. Ensure all earlier migrations are applied first.

Migration order is determined by the timestamp prefix in the filename. Use `sqlx migrate info` to see which migrations have been applied and which are pending.

</details>

---

## Exercise 2: Build the Weekly Calendar Component

**Goal:** A horizontal date picker showing the current week (Sun-Sat), with navigation arrows and swipe support. All date arithmetic uses integer-only calculations --- no `chrono` dependency on the client.

> **Programming Concept: What is Date Arithmetic?**
>
> Date arithmetic is calculating with dates: adding days, finding the day of the week, or determining the difference between two dates.
>
> It sounds simple, but dates are surprisingly tricky. How many days does February have? Depends on whether it is a leap year. What about February 2100? Not a leap year (centuries are not leap years unless divisible by 400). When does Monday fall if today is Thursday? You need to know the day-of-week formula.
>
> GrindIt uses an ancient mathematical trick called **Julian Day Numbers** --- a continuous count of days since January 1, 4713 BC. Converting between regular dates and JDN uses pure integer math: no floating point, no date library, and no allocation. Shifting a date by 7 days is just adding 7 to the JDN and converting back.

### Step 1: Date arithmetic with Julian Day Numbers

GrindIt avoids pulling `chrono` into the WASM bundle. Instead, it uses JDN:

```rust
pub(crate) fn ymd_to_jdn(y: i64, m: i64, d: i64) -> i64 {
    (1461 * (y + 4800 + (m - 14) / 12)) / 4
        + (367 * (m - 2 - 12 * ((m - 14) / 12))) / 12
        - (3 * ((y + 4900 + (m - 14) / 12) / 100)) / 4
        + d
        - 32075
}

pub(crate) fn jdn_to_ymd(jdn: i64) -> (i64, i64, i64) {
    let l = jdn + 68569;
    let n = (4 * l) / 146097;
    let l = l - (146097 * n + 3) / 4;
    let i = (4000 * (l + 1)) / 1461001;
    let l = l - (1461 * i) / 4 + 31;
    let j = (80 * l) / 2447;
    let d = l - (2447 * j) / 80;
    let l = j / 11;
    let m = j + 2 - 12 * l;
    let y = 100 * (n - 49) + i + l;
    (y, m, d)
}
```

These formulas date back to the 16th century. They look like magic, but they handle month boundaries, leap years, and century transitions perfectly. The beauty is that they use only integer arithmetic --- no floating point, no external libraries.

With JDN, common operations become trivial:

- **Add 7 days:** `jdn_to_ymd(ymd_to_jdn(y, m, d) + 7)`
- **Find day of week:** `(jdn + 1) % 7` gives 0=Sunday, 1=Monday, ..., 6=Saturday
- **Difference between dates:** `jdn1 - jdn2` gives the number of days

### Step 2: Computing the week

```rust
pub(crate) fn compute_week_dates(anchor: &str) -> (String, Vec<String>) {
    let today = today_iso();
    let (y, m, d) = if anchor.is_empty() {
        parse_ymd(&today)
    } else {
        parse_ymd(anchor)
    };
    let jdn = ymd_to_jdn(y, m, d);
    let dow = (jdn + 1) % 7; // 0=Sun, 1=Mon, ..., 6=Sat
    let sunday_jdn = jdn - dow;
    let week: Vec<String> = (0..7)
        .map(|i| {
            let (ny, nm, nd) = jdn_to_ymd(sunday_jdn + i);
            format!("{:04}-{:02}-{:02}", ny, nm, nd)
        })
        .collect();
    (today, week)
}
```

Let us trace through this step by step:

1. We start with an `anchor` date --- the date the calendar is centered on. If empty, we use today.
2. We convert the anchor to a Julian Day Number.
3. We find the day of the week (0 for Sunday, 6 for Saturday).
4. We subtract the day-of-week from the JDN to get Sunday's JDN.
5. We generate 7 dates starting from Sunday, giving us Sun--Sat.

To navigate to the previous week, set the anchor to 7 days before. To navigate to the next week, set the anchor to 1 day after Saturday.

### Step 3: The today_iso function

```rust
pub(crate) fn today_iso() -> String {
    #[cfg(feature = "hydrate")]
    {
        let d = js_sys::Date::new_0();
        format!(
            "{:04}-{:02}-{:02}",
            d.get_full_year(),
            d.get_month() + 1,  // JS months are 0-indexed
            d.get_date()
        )
    }
    #[cfg(feature = "ssr")]
    {
        chrono::Local::now().date_naive().to_string()
    }
}
```

This function has two implementations: on the client (WASM), it uses JavaScript's `Date` API through `js_sys`. On the server, it uses `chrono`. The compiler picks the right one based on which feature is active.

Note the `d.get_month() + 1` --- JavaScript months are 0-indexed (January is 0, February is 1), so we add 1 to get the normal 1--12 range. This is one of those classic JavaScript gotchas.

### Step 4: The WeeklyCalendar component

```rust
const DAY_LABELS: [&str; 7] = ["S", "M", "T", "W", "T", "F", "S"];

#[component]
pub fn WeeklyCalendar(
    selected_date: RwSignal<String>,
    #[prop(optional)] anchor: Option<RwSignal<String>>,
) -> impl IntoView {
    let anchor = anchor.unwrap_or_else(|| RwSignal::new(String::new()));

    let week = Memo::new(move |_| compute_week_dates(&anchor.get()));

    view! {
        <div class="week-calendar">
            {move || {
                let (today, dates) = week.get();
                let first = dates.first().cloned().unwrap_or_default();
                let last = dates.last().cloned().unwrap_or_default();
                let month_label = week_month_label(&first, &last);
                view! {
                    <div class="week-cal-month">{month_label}</div>
                    <div class="week-cal-row">
                        <button
                            class="week-cal-nav"
                            on:click=move |_| anchor.set(shift_date(&first, -7))
                        >"\u{2039}"</button>

                        <div class="week-cal-days">
                            {dates.into_iter().enumerate().map(|(i, date)| {
                                let day_num = date_day_num(&date);
                                let is_today = date == today;
                                let d = date.clone();
                                view! {
                                    <button
                                        class="week-cal-day"
                                        class:selected=move || selected_date.get() == d
                                        class:week-cal-day--today=is_today
                                        on:click={
                                            let d2 = date.clone();
                                            move |_| selected_date.set(d2.clone())
                                        }
                                    >
                                        <span class="week-cal-label">{DAY_LABELS[i]}</span>
                                        <span class="week-cal-num"
                                            class:today=is_today
                                        >{day_num}</span>
                                    </button>
                                }
                            }).collect_view()}
                        </div>

                        <button
                            class="week-cal-nav"
                            on:click=move |_| anchor.set(shift_date(&last, 1))
                        >"\u{203A}"</button>
                    </div>
                }
            }}
        </div>
    }
}
```

Let us unpack the key patterns:

**`Memo::new`** caches the week computation. It only recalculates when `anchor` changes. This is more efficient than computing inside the `view!` closure (which runs on every render).

**`#[prop(optional)]`** makes `anchor` an optional prop. If the parent does not provide one, the component creates its own internal signal. This makes the component reusable --- the WOD page can control the anchor externally, while a standalone calendar manages its own state.

**The `let d = date.clone()` and `let d2 = date.clone()` lines** deserve explanation. Inside a loop, the `date` variable changes with each iteration. But closures capture variables by reference. If the `on:click` closure captured `date` directly, all closures would point to the same variable, and every day button would set the date to the last day. Cloning before the closure gives each closure its own copy.

<details>
<summary>Hint: If the selected date does not update when clicking a day</summary>

Verify that `selected_date` is an `RwSignal`, not a derived memo or read-only signal. The `on:click` handler calls `selected_date.set(...)`, which requires write access.

Also check that the closure captures `date.clone()` before the loop moves on. Without cloning, all closures would capture the same reference.

</details>

---

## Exercise 3: Build the WOD Creation Form

**Goal:** A form that creates a new WOD with title, description, workout type, time cap, and date. Coaches and admins see the form; athletes do not.

### Step 1: The server function

```rust
// src/pages/wod/server_fns.rs
use crate::db::{Wod, WodSection, WodMovement};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::auth::UserRole;

#[server]
pub async fn create_wod(
    title: String,
    description: String,
    workout_type: String,
    time_cap_minutes: String,
    programmed_date: String,
) -> Result<String, ServerFnError> {
    let user = crate::auth::session::require_role(UserRole::Coach).await?;
    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = user.id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    let time_cap = if time_cap_minutes.is_empty() {
        None
    } else {
        time_cap_minutes.parse::<i32>().ok()
    };
    let desc = if description.is_empty() { None } else { Some(description.as_str()) };

    crate::db::create_wod_db(
        &pool, &title, desc, &workout_type, time_cap, &programmed_date, Some(user_uuid),
    )
    .await
    .map(|id| id.to_string())
    .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn list_wods_for_date(date: String) -> Result<Vec<Wod>, ServerFnError> {
    let pool = crate::db::db().await?;
    crate::db::list_wods_for_date_db(&pool, &date)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

Notice that `create_wod` calls `require_role(UserRole::Coach)` --- only coaches and admins can create WODs. The `list_wods_for_date` function does NOT require auth --- anyone can view the day's programming.

Also notice the pattern for optional string fields: check if empty, then wrap in `Option`. This bridges HTML form inputs (which always produce strings) and Rust types (which distinguish between "no value" and "empty value").

### Step 2: The WodForm component

```rust
// src/pages/wod/wod_form.rs
use leptos::prelude::*;
use super::CreateWod;

#[component]
pub fn WodForm(
    create_action: ServerAction<CreateWod>,
    show_form: RwSignal<bool>,
    title_input: RwSignal<String>,
    desc_input: RwSignal<String>,
    type_input: RwSignal<String>,
    cap_input: RwSignal<String>,
    date_input: RwSignal<String>,
) -> impl IntoView {
    view! {
        <form
            class="wod-form"
            on:submit=move |ev| {
                ev.prevent_default();
                let t = title_input.get_untracked();
                if t.is_empty() { return; }
                create_action.dispatch(CreateWod {
                    title: t,
                    description: desc_input.get_untracked(),
                    workout_type: type_input.get_untracked(),
                    time_cap_minutes: cap_input.get_untracked(),
                    programmed_date: date_input.get_untracked(),
                });
                title_input.set(String::new());
                desc_input.set(String::new());
                cap_input.set(String::new());
                show_form.set(false);
            }
        >
            <div class="form-row">
                <input
                    type="date"
                    prop:value=move || date_input.get()
                    on:input=move |ev| date_input.set(event_target_value(&ev))
                />
                <select
                    prop:value=move || type_input.get()
                    on:change=move |ev| type_input.set(event_target_value(&ev))
                >
                    <option value="fortime">"For Time"</option>
                    <option value="amrap">"AMRAP"</option>
                    <option value="emom">"EMOM"</option>
                    <option value="tabata">"Tabata"</option>
                    <option value="strength">"Strength"</option>
                    <option value="custom">"Custom"</option>
                </select>
            </div>
            <input
                type="text"
                placeholder="WOD title (e.g. Fran)"
                prop:value=move || title_input.get()
                on:input=move |ev| title_input.set(event_target_value(&ev))
            />
            <input
                type="text"
                placeholder="Description (optional)"
                prop:value=move || desc_input.get()
                on:input=move |ev| desc_input.set(event_target_value(&ev))
            />
            <input
                type="number"
                placeholder="Time cap (minutes)"
                prop:value=move || cap_input.get()
                on:input=move |ev| cap_input.set(event_target_value(&ev))
            />
            <button type="submit" class="form-submit"
                disabled=move || create_action.pending().get()
            >
                {move || if create_action.pending().get() {
                    "Creating..."
                } else {
                    "Create WOD"
                }}
            </button>
        </form>
    }
}
```

The form props are all `RwSignal<String>` --- the parent (WodPage) owns the state. This is Leptos's equivalent of "lifting state up" in React. The parent can clear the form, pre-fill values for editing, or read the values from elsewhere.

**`get_untracked()`** reads a signal's current value without subscribing to future changes. In the submit handler, we want the current values but do not want the closure to re-run when the values change. Using `.get()` would create a subscription, causing the closure to regenerate on every keystroke.

### Step 3: The WodPage with role-based FAB

```rust
// src/pages/wod/mod.rs (simplified)
#[component]
pub fn WodPage() -> impl IntoView {
    let auth_user = use_context::<AuthUser>();
    let is_coach = auth_user
        .as_ref()
        .map(|u| matches!(u.role, UserRole::Coach | UserRole::Admin))
        .unwrap_or(false);

    let create_action = ServerAction::<CreateWod>::new();
    let selected_date = RwSignal::new(week_calendar::today_iso());

    let wods = Resource::new(
        move || (selected_date.get(), create_action.version().get()),
        |(date, _)| list_wods_for_date(date),
    );

    let show_form = RwSignal::new(false);
    // ... signal declarations for form inputs ...

    let fab_view = if is_coach {
        view! {
            <button
                class={move || if show_form.get() { "fab fab--active" } else { "fab" }}
                on:click=move |_| show_form.update(|v| *v = !*v)
            >
                <span class="fab-icon"></span>
            </button>
        }.into_any()
    } else {
        ().into_view().into_any()
    };

    view! {
        <div class="wod-page">
            {fab_view}
            // ... form and list views ...
            <WeeklyCalendar selected_date=selected_date />
            <Suspense fallback=|| view! { <p class="loading">"Loading WODs..."</p> }>
                // ... WOD list rendering ...
            </Suspense>
        </div>
    }
}
```

The `is_coach` check uses `matches!` --- a macro that returns `true` if the value matches the pattern. `matches!(u.role, UserRole::Coach | UserRole::Admin)` checks two variants at once. The floating action button (FAB) for creating WODs only appears for coaches and admins.

**Resource versioning:** The `Resource::new` call includes `create_action.version().get()` in its source. Every time a WOD is created (the action completes), the version increments, triggering a refetch of the WOD list. This is how Leptos implements "refetch on mutation" without manual cache invalidation.

<details>
<summary>Hint: If the FAB appears for all users</summary>

Verify that `AuthUser` is being provided to the context. In your `app.rs`, after loading the user via `get_me()`, you need to call `provide_context(user)`. If the context is not provided, `use_context()` returns `None`, and `is_coach` defaults to `false` (correct). But if you are providing a default `AuthUser` with `Coach` role for testing, that explains the issue.

</details>

---

## Exercise 4: Build the WOD Card with Sections

**Goal:** A card that displays a WOD with its metadata, sections, and movements. Owners see edit/delete buttons.

### Step 1: Helper functions for display

```rust
// src/pages/wod/helpers.rs
pub fn wod_type_label(t: &str) -> &'static str {
    match t {
        "amrap" => "AMRAP",
        "fortime" => "FOR TIME",
        "emom" => "EMOM",
        "tabata" => "TABATA",
        "strength" => "STRENGTH",
        _ => "CUSTOM",
    }
}

pub fn phase_label(p: &str) -> &'static str {
    match p {
        "warmup" => "Warm-Up",
        "strength" => "Strength",
        "conditioning" => "Conditioning",
        "cooldown" => "Cool Down",
        "optional" => "Optional",
        "personal" => "Personal",
        _ => "Section",
    }
}

pub fn section_derived_label(
    section_type: &str,
    time_cap: Option<i32>,
    rounds: Option<i32>,
    title: Option<&str>,
) -> String {
    if let Some(t) = title.filter(|s| !s.is_empty()) {
        return t.to_string();
    }
    match section_type {
        "fortime" => {
            let mut s = String::new();
            if let Some(r) = rounds { s.push_str(&format!("{} Rounds ", r)); }
            s.push_str("For Time");
            if let Some(cap) = time_cap { s.push_str(&format!(" . {} min cap", cap)); }
            s
        }
        "amrap" => time_cap.map(|c| format!("{} min AMRAP", c))
            .unwrap_or_else(|| "AMRAP".to_string()),
        "emom" => time_cap.map(|c| format!("{} min EMOM", c))
            .unwrap_or_else(|| "EMOM".to_string()),
        "strength" => "Strength".to_string(),
        other => section_type_label(other).to_string(),
    }
}
```

The `section_derived_label` function shows a common Rust pattern: **derive labels from structured data**. Rather than storing a display string in the database, we store structured fields (`section_type`, `time_cap`, `rounds`, `title`) and build the human-readable label at render time. This keeps the data clean and makes it easy to change the display format without migrating the database.

### Step 2: The WodCard component

```rust
// src/pages/wod/wod_card.rs (simplified)
#[component]
pub fn WodCard(
    wod: Wod,
    can_edit: bool,
    editing_wod: RwSignal<Option<String>>,
    update_action: ServerAction<UpdateWod>,
    pending_delete_wod_id: RwSignal<String>,
    show_delete_wod: RwSignal<bool>,
) -> impl IntoView {
    let expanded = RwSignal::new(true);
    let wid_panel = StoredValue::new(wod.id.clone());
    let wid_del = wod.id.clone();

    let type_label = wod_type_label(&wod.workout_type);
    let type_cls = format!("wod-badge {}", wod_type_class(&wod.workout_type));
    let title = wod.title.clone();
    let desc = wod.description.clone();

    view! {
        <div class="wod-card" on:click=move |_| {
            if editing_wod.get().is_some() { return; }
            expanded.update(|v| *v = !*v);
        }>
            <div class="wod-card-top">
                <div class="wod-card-meta">
                    <span class={type_cls}>{type_label}</span>
                    <span class="wod-date">{wod.programmed_date.clone()}</span>
                </div>
                <div class="wod-card-actions" on:click=move |ev| ev.stop_propagation()>
                    {can_edit.then(move || view! {
                        <button class="wod-edit-btn"
                            on:click=move |_| editing_wod.set(Some(wid_del.clone()))
                        >"Edit"</button>
                        <button class="wod-delete"
                            on:click=move |_| {
                                pending_delete_wod_id.set(wid_del.clone());
                                show_delete_wod.set(true);
                            }
                        >"x"</button>
                    })}
                </div>
            </div>
            <h2 class="wod-title">{title}</h2>
            {desc.map(|d| view! { <p class="wod-desc">{d}</p> })}
            {move || expanded.get().then(|| view! {
                <WodSectionsPanel wod_id=wid_panel.get_value() can_edit=can_edit />
            })}
        </div>
    }
}
```

**Ownership check.** The `can_edit` boolean is computed by the parent:

```rust
let can_edit = is_coach && (
    is_admin || wod.created_by.as_deref() == current_user_id.as_deref()
);
```

Coaches can edit their own WODs. Admins can edit any WOD. This check happens in the UI for showing/hiding buttons AND on the server in the update/delete functions. **Never trust the client** --- the server must verify permissions independently.

**`StoredValue`** vs `RwSignal`. The `wid_panel` uses `StoredValue::new(wod.id.clone())`. Unlike `RwSignal`, `StoredValue` is not reactive --- it does not notify subscribers when changed. We use it for values that are set once and never change, avoiding unnecessary reactivity overhead.

**`ev.stop_propagation()`** on the actions div prevents the card's click handler (which toggles expand/collapse) from firing when the user clicks edit or delete. Without this, clicking "Delete" would also toggle the card.

<details>
<summary>Hint: If clicking the WodCard always collapses it, even when clicking edit</summary>

Ensure that `ev.stop_propagation()` is on the actions container, not on individual buttons. The propagation stop prevents clicks from bubbling up to the card's click handler.

Also check that the `on:click` on the card checks `editing_wod.get().is_some()` and returns early when in edit mode.

</details>

---

## Rust Gym

### Drill 1: Navigate a Nested Structure

Given these structs, write a function that returns the name of the first movement in the first section. Return `"No movements"` if any level is empty.

```rust
struct Wod { sections: Vec<WodSection> }
struct WodSection { movements: Vec<WodMovement> }
struct WodMovement { exercise_name: String }

fn first_movement_name(wod: &Wod) -> &str {
    // Your implementation
}
```

<details>
<summary>Hint</summary>

Use `.first()` to get the first element of a Vec (it returns `Option`). Chain `.and_then()` to go one level deeper. Use `.unwrap_or()` for the fallback.

</details>

<details>
<summary>Solution</summary>

```rust
fn first_movement_name(wod: &Wod) -> &str {
    wod.sections
        .first()                           // Option<&WodSection>
        .and_then(|s| s.movements.first()) // Option<&WodMovement>
        .map(|m| m.exercise_name.as_str()) // Option<&str>
        .unwrap_or("No movements")         // &str
}
```

This chains `Option` methods: `first()` returns `Option<&WodSection>`, `and_then` transforms the inner value while keeping the `Option` wrapper, `map` transforms the value inside `Some`, and `unwrap_or` provides the fallback. No `if` statements, no null checks.

</details>

### Drill 2: Sum with filter_map

Given a `Vec<WodSection>`, write a function that returns the total time cap across all sections that have one. Sections without a time cap should be ignored.

```rust
fn total_time_cap(sections: &[WodSection]) -> i32 {
    // Your implementation
}
```

<details>
<summary>Hint</summary>

`filter_map` combines filtering and mapping in one step. It applies a function to each element: if the function returns `Some(value)`, the value is kept. If it returns `None`, the element is skipped.

</details>

<details>
<summary>Solution</summary>

```rust
fn total_time_cap(sections: &[WodSection]) -> i32 {
    sections
        .iter()
        .filter_map(|s| s.time_cap_minutes)  // keeps Some values, skips None
        .sum()
}
```

`filter_map` is perfect here because `time_cap_minutes` is already `Option<i32>`. Sections with `None` are automatically filtered out, and sections with `Some(value)` yield the value.

</details>

### Drill 3: Count Across Tables

Write a function that counts movements belonging to the given sections, using separate vectors (as they come from the database):

```rust
fn count_movements(sections: &[WodSection], movements: &[WodMovement]) -> usize {
    // Your implementation
}
```

<details>
<summary>Hint</summary>

Build a `HashSet` of section IDs, then count how many movements have a `section_id` in that set. This is O(sections + movements) instead of O(sections * movements).

</details>

<details>
<summary>Solution</summary>

```rust
fn count_movements(sections: &[WodSection], movements: &[WodMovement]) -> usize {
    let section_ids: std::collections::HashSet<&str> = sections
        .iter()
        .map(|s| s.id.as_str())
        .collect();

    movements
        .iter()
        .filter(|m| section_ids.contains(m.section_id.as_str()))
        .count()
}
```

Because the data comes from separate queries (normalized tables), we join them in code. The `HashSet` gives O(1) lookup per movement, making the total cost O(sections + movements). This mirrors what the database does with an indexed JOIN.

</details>

---

## DSA in Context: N-ary Tree Traversal

A WOD is an **N-ary tree** --- a tree where each node can have any number of children:

```
         WOD (root)
       /     |      \
  Section  Section  Section
   / \       |        |
 Mvt Mvt   Mvt      Mvt
```

When the coach creates a WOD, they build this tree top-down: create the WOD, add sections, add movements to each section. When the athlete views the WOD, we traverse the tree to render it.

**DFS (depth-first)** visits one branch completely before moving to the next. This is natural for rendering --- you render Section 1's header, then all of Section 1's movements, then Section 2's header, then Section 2's movements. GrindIt's rendering is implicitly DFS:

```rust
sections.iter().map(|section| {
    view! {
        <SectionHeader section=section />
        {section_movements.iter().map(|mvt| {
            view! { <MovementRow mvt=mvt /> }
        }).collect_view()}
    }
}).collect_view()
```

**BFS (breadth-first)** visits all nodes at the same level before going deeper. This would render all section headers first, then all movements. Less useful for display, but useful for queries like "find all sections of type AMRAP."

In the database, the tree is stored as separate tables (normalized), and we reconstruct it with queries. This is more flexible than a literal tree structure --- you can query movements across all WODs without traversing any trees.

---

## System Design Corner: Normalization vs Denormalization

### Normalized (GrindIt's approach)

Three tables: `wods`, `wod_sections`, `wod_movements`. Data lives in one place. Updating a movement's rep scheme is a single UPDATE on one row.

**Pros:** No data duplication, easy to query individual entities, foreign key constraints prevent orphans, JOINs are efficient with indexes.

**Cons:** Multiple queries to load a full WOD, more complex INSERT logic (create parent first, then children with the parent's ID).

### Denormalized alternative

One table: `wods` with a `sections JSONB` column containing the entire tree.

**Pros:** Single query to load everything, simple INSERT (one row), natural mapping to nested JavaScript objects.

**Cons:** Updating one movement requires rewriting the entire JSON blob, cannot query individual movements across WODs, no referential integrity, limited indexing.

### When to choose which

Use **normalization** (separate tables) when:
- You query at multiple levels (e.g., "show all WODs containing back squats")
- Data integrity matters (foreign keys prevent invalid references)
- Individual entities are updated independently

Use **denormalization** (JSON blobs) when:
- The nested data is always loaded and saved as a unit
- You never query the inner structure
- Schema flexibility matters more than integrity

GrindIt's WOD system clearly fits normalization: we query movements by exercise, sections by type, and WODs by date.

---

## Design Insight: Deep Modules (Ousterhout)

The WOD system is a **deep module**. Its interface is simple:

- `create_wod(title, type, date)` returns a WOD ID
- `list_wods_for_date(date)` returns WODs
- `<WodCard wod=wod can_edit=can_edit />` renders a WOD

Behind this interface, the module manages three database tables, tree reconstruction, label derivation, ownership checks, and sort ordering. The caller only interacts with the top-level interface.

Ousterhout's principle: *the best modules are those whose interface is much simpler than their implementation.*

---

## What You Built

In this chapter, you:

1. **Created three database tables** --- `wods`, `wod_sections`, and `wod_movements` --- with foreign keys, enum types, cascading deletes, and indexes
2. **Built the WeeklyCalendar** component with integer-only date arithmetic (Julian Day Numbers), week navigation, touch gestures, and today highlighting
3. **Built the WodForm** for creating WODs with workout type selection, date picker, and coach-level authorization
4. **Built the WodCard** for displaying WODs with sections, movements, and ownership-based edit/delete buttons
5. **Practiced nested struct patterns** --- `Option` chaining, `filter_map`, tree traversal, and the normalized data model

The WOD system is now complete for coaches: they can program structured workouts, navigate by week, and manage sections and movements. Athletes can browse the day's WODs. In Chapter 9, athletes will log their scores against these WODs.

---

### 🧬 DS Deep Dive

Ready to go deeper? This chapter's data structure deep dive builds a generic N-ary tree and shows why your WOD is naturally a tree with DFS and BFS traversal from scratch in Rust — no libraries, just std.

**→ [N-ary Tree WOD](../ds-narratives/ch08-nary-tree-wod.md)**

---

### Reference implementation

The files you built in this chapter correspond to these files in the reference codebase:

| Your file | Reference |
|-----------|-----------|
| WOD migrations | [`migrations/20260311120006_create_wods_table.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120006_create_wods_table.sql), [`..._007_add_enums_gender_wod_sections.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120007_add_enums_gender_wod_sections.sql), [`..._008_rework_wod_movements.sql`](https://github.com/sivakarasala/gritwit/blob/main/migrations/20260311120008_rework_wod_movements.sql) |
| `src/pages/wod/mod.rs` | [`src/pages/wod/mod.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/wod/mod.rs) --- `WodPage` with role checks and calendar |
| `src/pages/wod/week_calendar.rs` | [`src/pages/wod/week_calendar.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/wod/week_calendar.rs) --- JDN date math, `WeeklyCalendar` component |
| `src/pages/wod/wod_form.rs` | [`src/pages/wod/wod_form.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/wod/wod_form.rs) --- WOD creation form |
| `src/pages/wod/wod_card.rs` | [`src/pages/wod/wod_card.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/wod/wod_card.rs) --- WOD display with expand/collapse and edit mode |
| `src/pages/wod/helpers.rs` | [`src/pages/wod/helpers.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/wod/helpers.rs) --- label and class helpers |
| `src/pages/wod/server_fns.rs` | [`src/pages/wod/server_fns.rs`](https://github.com/sivakarasala/gritwit/blob/main/src/pages/wod/server_fns.rs) --- WOD, section, and movement CRUD |
