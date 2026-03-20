# Error Handling Ecosystem --- "The Incident Report Form"

Your CRUD operations work. `Result<Exercise, String>` gets the job done. But then you add database errors, validation errors, auth errors, and serialization errors --- and suddenly you're writing `.map_err(|e| e.to_string())` on every line. Your error messages are useless ("something went wrong"), your error types don't compose, and you can't tell a 404 from a 500 from a user typo.

It's time to build a real error system --- one that knows *what* went wrong, *why*, and what to tell the user vs what to log for debugging.

---

## The gym analogy

Think about how a real gym handles incidents.

**String errors are like yelling "SOMETHING BROKE!" across the gym floor.** Everyone hears it, nobody knows what happened. Was it the cable machine? Did someone get hurt? Is the building on fire? You have no idea, and neither does anyone else. All you have is a string of panic.

**Typed errors are the incident report form.** Category (equipment failure, injury, access violation). Severity (minor, major, critical). What happened. Who's affected. What to do next. The gym manager reads one section, the athlete reads another, the insurance company reads a third. Same incident, structured for every audience.

**Error conversion --- the `From` trait --- is translating between departments.** The maintenance team reports "cable machine motor fault code E-47." The front desk translates that to "the cable machine is out of service." The app shows the athlete "choose a different exercise." Same underlying failure, three different representations for three different audiences. That's what `From` implementations do in Rust.

---

## 1. The Problem with String Errors

Here's where most projects start:

```rust
struct Exercise {
    id: i32,
    name: String,
    category: String,
}

fn create_exercise(name: &str) -> Result<Exercise, String> {
    if name.is_empty() {
        return Err("Name is required".to_string());
    }
    if name.len() > 100 {
        return Err("Name is too long".to_string());
    }
    // Imagine this calls a database:
    // save_to_db(&exercise).map_err(|e| e.to_string())?;
    Ok(Exercise {
        id: 1,
        name: name.to_string(),
        category: "Weightlifting".to_string(),
    })
}

fn main() {
    match create_exercise("") {
        Ok(ex) => println!("Created: {}", ex.name),
        Err(e) => println!("Error: {}", e),
    }
}
```

This compiles and runs. So what's wrong?

**You can't pattern match on error types.** Is `"Name is required"` a validation error or a database error? You'd have to do string comparison --- fragile, untranslatable, and guaranteed to break when someone changes the wording.

**You're losing context.** When you call `.map_err(|e| e.to_string())`, the original error type is gone forever. You had a `sqlx::Error` with connection details, query text, and a Postgres error code. Now you have the string `"error returned from database: connection refused"`. Good luck programmatically deciding what to do with that.

**You can't decide HTTP status codes.** A validation error should be 400 (Bad Request). A missing exercise should be 404. A database failure should be 500. With `String`, they're all just... strings. Your API handler has no way to distinguish them.

**There's no error chain.** "Connection refused" --- to *what*? At what point? During which operation? A string gives you one layer. Real failures have causes.

This is yelling "SOMETHING BROKE!" across the gym. Time to fill out the incident report.

---

## 2. Custom Error Types --- The Enum Approach

Every well-structured Rust application defines its own error type. For GrindIt:

```rust
use std::fmt;

#[derive(Debug)]
enum GrindItError {
    Validation(String),     // 400 --- user did something wrong
    NotFound(String),       // 404 --- resource doesn't exist
    Unauthorized(String),   // 401 --- not logged in
    Forbidden(String),      // 403 --- wrong role
    Database(String),       // 500 --- internal DB failure
    Internal(String),       // 500 --- catch-all
}

impl fmt::Display for GrindItError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrindItError::Validation(msg) => write!(f, "Validation error: {}", msg),
            GrindItError::NotFound(msg) => write!(f, "Not found: {}", msg),
            GrindItError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            GrindItError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            GrindItError::Database(msg) => write!(f, "Database error: {}", msg),
            GrindItError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for GrindItError {}

fn main() {
    let errors: Vec<GrindItError> = vec![
        GrindItError::Validation("Name is required".into()),
        GrindItError::NotFound("Exercise #42".into()),
        GrindItError::Database("connection refused".into()),
    ];

    for err in &errors {
        let status = match err {
            GrindItError::Validation(_) => 400,
            GrindItError::Unauthorized(_) => 401,
            GrindItError::Forbidden(_) => 403,
            GrindItError::NotFound(_) => 404,
            GrindItError::Database(_) | GrindItError::Internal(_) => 500,
        };
        println!("[{}] {}", status, err);
    }
}
```

Now your API handler can pattern match:

```rust
# use std::fmt;
# #[derive(Debug)]
# enum GrindItError {
#     Validation(String),
#     NotFound(String),
#     Unauthorized(String),
#     Forbidden(String),
#     Database(String),
#     Internal(String),
# }
# impl fmt::Display for GrindItError {
#     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
#         write!(f, "{:?}", self)
#     }
# }
# impl std::error::Error for GrindItError {}
fn handle_error(err: &GrindItError) -> (u16, String) {
    match err {
        GrindItError::Validation(msg) => (400, msg.clone()),
        GrindItError::NotFound(msg) => (404, msg.clone()),
        // User sees a friendly message; logs get the real error
        GrindItError::Database(detail) => {
            eprintln!("DB error (logged, not shown to user): {}", detail);
            (500, "Something went wrong. Please try again.".into())
        }
        GrindItError::Unauthorized(_) => (401, "Please log in to continue".into()),
        GrindItError::Forbidden(_) => (403, "You don't have permission".into()),
        GrindItError::Internal(detail) => {
            eprintln!("Internal error (logged): {}", detail);
            (500, "Something went wrong. Please try again.".into())
        }
    }
}

fn main() {
    let err = GrindItError::Database("connection refused to 10.0.0.5:5432".into());
    let (status, message) = handle_error(&err);
    println!("HTTP {} — User sees: {}", status, message);
    // HTTP 500 — User sees: Something went wrong. Please try again.
    // (The real error went to stderr for developers.)
}
```

This is the incident report form. The gym manager (your API handler) reads the category field and decides what to do. The athlete (your user) gets a friendly message. The maintenance log (stderr) gets the full technical detail.

---

## 3. The From Trait --- Automatic Error Conversion

Writing `map_err(|e| GrindItError::Validation(...))` everywhere gets old. The `?` operator can do this automatically --- if you implement the `From` trait.

```rust
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
enum GrindItError {
    Validation(String),
    NotFound(String),
    Internal(String),
}

impl fmt::Display for GrindItError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrindItError::Validation(msg) => write!(f, "Validation: {}", msg),
            GrindItError::NotFound(msg) => write!(f, "Not found: {}", msg),
            GrindItError::Internal(msg) => write!(f, "Internal: {}", msg),
        }
    }
}

impl std::error::Error for GrindItError {}

impl From<ParseIntError> for GrindItError {
    fn from(err: ParseIntError) -> Self {
        GrindItError::Validation(format!("Invalid integer: {}", err))
    }
}

impl From<ParseFloatError> for GrindItError {
    fn from(err: ParseFloatError) -> Self {
        GrindItError::Validation(format!("Invalid number: {}", err))
    }
}

impl From<std::io::Error> for GrindItError {
    fn from(err: std::io::Error) -> Self {
        GrindItError::Internal(format!("IO error: {}", err))
    }
}

fn parse_weight(input: &str) -> Result<f64, GrindItError> {
    let weight: f64 = input.parse()?; // ParseFloatError -> GrindItError::Validation
    if weight <= 0.0 {
        return Err(GrindItError::Validation("Weight must be positive".into()));
    }
    Ok(weight)
}

fn main() {
    match parse_weight("not_a_number") {
        Ok(w) => println!("Weight: {}", w),
        Err(e) => println!("{}", e),
        // Prints: Validation: Invalid number: invalid float literal
    }

    match parse_weight("-5") {
        Ok(w) => println!("Weight: {}", w),
        Err(e) => println!("{}", e),
        // Prints: Validation: Weight must be positive
    }
}
```

This is the department translation at work. The maintenance team (the standard library) reports `ParseFloatError`. Your `From` impl translates it into the front desk's language (`GrindItError::Validation`). The athlete sees "Invalid number" instead of a raw Rust error type name.

---

## 4. The ? Operator --- Desugar the Magic

The `?` operator is the most-used piece of syntax in Rust error handling. Here's what it actually does:

```rust
# use std::fmt;
# use std::num::{ParseFloatError, ParseIntError};
# #[derive(Debug)]
# enum GrindItError {
#     Validation(String),
#     NotFound(String),
#     Internal(String),
# }
# impl fmt::Display for GrindItError {
#     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
#         write!(f, "{:?}", self)
#     }
# }
# impl std::error::Error for GrindItError {}
# impl From<ParseIntError> for GrindItError {
#     fn from(err: ParseIntError) -> Self {
#         GrindItError::Validation(format!("Invalid integer: {}", err))
#     }
# }
# impl From<ParseFloatError> for GrindItError {
#     fn from(err: ParseFloatError) -> Self {
#         GrindItError::Validation(format!("Invalid number: {}", err))
#     }
# }
# #[derive(Debug)]
# struct Exercise { id: i32, name: String, weight: f64 }

// This line:
//   let id: i32 = raw_id.parse()?;
//
// Is syntactic sugar for:
//   let id: i32 = match raw_id.parse() {
//       Ok(val) => val,
//       Err(err) => return Err(From::from(err)),
//   };

// Three things happen:
// 1. Unwrap the Ok value
// 2. Convert the error type via From::from()
// 3. Early-return the Err

fn validate_name(name: &str) -> Result<(), GrindItError> {
    if name.is_empty() {
        return Err(GrindItError::Validation("Name is required".into()));
    }
    Ok(())
}

fn find_exercise(id: i32) -> Result<Exercise, GrindItError> {
    if id == 42 {
        Ok(Exercise { id: 42, name: "Back Squat".into(), weight: 100.0 })
    } else {
        Err(GrindItError::NotFound(format!("Exercise #{}", id)))
    }
}

fn save_exercise(exercise: &Exercise) -> Result<(), GrindItError> {
    // Pretend this talks to a database
    println!("Saved: {:?}", exercise);
    Ok(())
}

fn update_exercise(
    raw_id: &str,
    name: &str,
    raw_weight: &str,
) -> Result<Exercise, GrindItError> {
    let id: i32 = raw_id.parse()?;          // ParseIntError   -> Validation
    validate_name(name)?;                     // GrindItError    -> GrindItError (no conversion needed)
    let weight: f64 = raw_weight.parse()?;   // ParseFloatError -> Validation
    let mut exercise = find_exercise(id)?;    // GrindItError    -> GrindItError (NotFound passes through)
    exercise.name = name.to_string();
    exercise.weight = weight;
    save_exercise(&exercise)?;                // GrindItError    -> GrindItError (Database would pass through)
    Ok(exercise)
}

fn main() {
    // Each ? in update_exercise can fail with a different underlying type,
    // but they all convert to GrindItError automatically.
    println!("{:?}", update_exercise("abc", "Deadlift", "225"));
    // Err(Validation("Invalid integer: invalid digit found in string"))

    println!("{:?}", update_exercise("42", "", "225"));
    // Err(Validation("Name is required"))

    println!("{:?}", update_exercise("99", "Deadlift", "225"));
    // Err(NotFound("Exercise #99"))

    println!("{:?}", update_exercise("42", "Deadlift", "225"));
    // Ok(Exercise { id: 42, name: "Deadlift", weight: 225.0 })
}
```

Five `?` operators in one function, each potentially converting a different error type. The function reads like the happy path --- no nested matches, no manual conversions. But every failure is properly typed and categorized.

---

## 5. Error Context --- What Caused This?

Raw errors lose context. "Connection refused" --- to *what*? The `std::error::Error` trait has a `source()` method that builds a chain of causes:

```rust
use std::fmt;
use std::error::Error;

#[derive(Debug)]
struct ConnectionError {
    address: String,
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "connection refused to {}", self.address)
    }
}

impl Error for ConnectionError {}

#[derive(Debug)]
struct DatabaseError {
    message: String,
    source: Box<dyn Error>,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Database error: {}", self.message)
    }
}

impl Error for DatabaseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[derive(Debug)]
struct CreateExerciseError {
    exercise_name: String,
    source: Box<dyn Error>,
}

impl fmt::Display for CreateExerciseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to create exercise \"{}\"", self.exercise_name)
    }
}

impl Error for CreateExerciseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

fn print_error_chain(err: &dyn Error) {
    println!("Error: {}", err);
    let mut source = err.source();
    let mut depth = 1;
    while let Some(cause) = source {
        println!("{}Caused by: {}", "  ".repeat(depth), cause);
        source = cause.source();
        depth += 1;
    }
}

fn main() {
    // Build a three-level error chain:
    let conn_err = ConnectionError {
        address: "10.0.0.5:5432".into(),
    };
    let db_err = DatabaseError {
        message: "insert into exercises failed".into(),
        source: Box::new(conn_err),
    };
    let app_err = CreateExerciseError {
        exercise_name: "Back Squat".into(),
        source: Box::new(db_err),
    };

    print_error_chain(&app_err);
    // Error: Failed to create exercise "Back Squat"
    //   Caused by: Database error: insert into exercises failed
    //     Caused by: connection refused to 10.0.0.5:5432
}
```

Three departments, three levels of detail. The athlete sees "Failed to create exercise." The DBA sees "insert failed." The ops team sees "connection refused to 10.0.0.5:5432." Each layer adds context without losing the original cause.

---

## 6. thiserror vs anyhow --- The Ecosystem Split

Every serious Rust project uses one or both of two crates. They solve opposite sides of the same problem.

### thiserror --- for library code (defining error types)

`thiserror` is a derive macro that auto-generates `Display`, `Error`, and `From` implementations. Everything we wrote by hand in sections 2--3, thiserror writes for you:

```rust,ignore
use thiserror::Error;

#[derive(Error, Debug)]
enum GrindItError {
    #[error("Validation: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Database error")]
    Database(#[from] sqlx::Error),  // auto-generates From<sqlx::Error>!

    #[error("IO error")]
    Io(#[from] std::io::Error),     // auto-generates From<std::io::Error>!

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}
```

That `#[from]` attribute generates the entire `From` impl --- the same boilerplate we wrote by hand earlier. The `#[error("...")]` attribute generates the `Display` impl. Five lines replace fifty.

### anyhow --- for application code (handling errors)

`anyhow` takes the opposite approach. Instead of defining precise error types, it wraps *any* error into `anyhow::Error` and lets you add context:

```rust,ignore
use anyhow::{Context, Result};

fn load_exercises() -> Result<Vec<Exercise>> {
    let config = read_config()
        .context("Failed to read configuration")?;

    let exercises = fetch_from_db(&config.db_url)
        .context("Failed to fetch exercises from database")?;

    Ok(exercises)
}
```

Each `.context()` call adds a layer to the error chain --- exactly like the manual `CreateExerciseError` wrapping we built in section 5, but in one method call.

### When to use which

The rule is simple: **thiserror for libraries and APIs, anyhow for binaries and handlers.**

If you're writing code that other code calls (a library, a shared module, an API boundary), use `thiserror`. Your callers need to pattern match on your errors to decide what to do. `GrindItError::Validation` should become HTTP 400. `GrindItError::Database` should become HTTP 500. That requires typed errors.

If you're writing the final handler --- the `main()` function, a CLI tool, a one-off script --- use `anyhow`. You don't need to match on error types. You just need to print them, log them, or exit with an error code. `anyhow` makes that effortless.

In GrindIt: the domain layer uses `thiserror` to define `GrindItError`. The Axum handlers use those typed errors to decide HTTP status codes. A CLI migration tool might use `anyhow` because it just needs to print what went wrong and exit.

---

## 7. Error Design Philosophy

Every error has three audiences. Designing a good error system means serving all three.

**The user** needs a friendly, actionable message. "Exercise name is required" --- they can fix that. "UNIQUE constraint failed: exercises.name" --- they cannot.

**The developer** needs debugging information. The file, the line, the function, the input that triggered the failure, the full error chain. This goes in logs, not in API responses.

**The monitoring system** needs structured, categorized data. Error type, endpoint, severity, count. This drives alerts and dashboards, not human reading.

```rust
use std::fmt;

#[derive(Debug)]
enum GrindItError {
    Validation(String),
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    Database(String),
    Internal(String),
}

impl fmt::Display for GrindItError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl GrindItError {
    /// What to show the user
    fn user_message(&self) -> &str {
        match self {
            Self::Validation(msg) => msg,
            Self::NotFound(_) => "The requested resource was not found",
            Self::Unauthorized(_) => "Please log in to continue",
            Self::Forbidden(_) => "You don't have permission for this action",
            Self::Database(_) | Self::Internal(_) => {
                "Something went wrong. Please try again."
            }
        }
    }

    /// HTTP status code for the API layer
    fn status_code(&self) -> u16 {
        match self {
            Self::Validation(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::Forbidden(_) => 403,
            Self::NotFound(_) => 404,
            Self::Database(_) | Self::Internal(_) => 500,
        }
    }

    /// Category tag for monitoring/metrics
    fn error_category(&self) -> &str {
        match self {
            Self::Validation(_) => "validation",
            Self::Unauthorized(_) => "auth",
            Self::Forbidden(_) => "auth",
            Self::NotFound(_) => "not_found",
            Self::Database(_) => "database",
            Self::Internal(_) => "internal",
        }
    }
}

fn main() {
    let errors = vec![
        GrindItError::Validation("Exercise name is required".into()),
        GrindItError::Database("UNIQUE constraint failed: exercises.name".into()),
        GrindItError::NotFound("Exercise #999".into()),
    ];

    for err in &errors {
        // What the user sees:
        println!("HTTP {} — {}", err.status_code(), err.user_message());
        // What the developer sees (in logs):
        println!("  [DEBUG] {:?}", err);
        // What the monitoring system sees:
        println!("  [METRIC] error_category={}", err.error_category());
        println!();
    }
}
```

Notice how the database error *hides* the ugly constraint message from the user ("Something went wrong") while the debug output preserves the full detail for developers. Three audiences, one error type, zero information leakage.

---

## 8. Option vs Result --- When to Use Each

These two types solve different problems. Confusing them leads to awkward code.

| Situation | Use | Example |
|-----------|-----|---------|
| Value might not exist, and that's **normal** | `Option<T>` | `exercises.iter().find(\|e\| e.name == "Back Squat")` returns `Option<&Exercise>` |
| Operation can **fail**, and the caller needs to know **why** | `Result<T, E>` | `create_exercise(data)` returns `Result<Exercise, GrindItError>` |
| Convert Option to Result | `.ok_or()` | `find(id).ok_or(GrindItError::NotFound(...))` |
| Convert Result to Option | `.ok()` | `parse().ok()` --- discard the error, keep the value |

```rust
use std::fmt;

#[derive(Debug)]
struct Exercise { id: i32, name: String }

#[derive(Debug)]
enum GrindItError {
    NotFound(String),
    Validation(String),
}

impl fmt::Display for GrindItError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for GrindItError {}

fn find_exercise(exercises: &[Exercise], id: i32) -> Option<&Exercise> {
    // Option: the exercise might not exist, and that's fine
    exercises.iter().find(|e| e.id == id)
}

fn get_exercise_or_error(
    exercises: &[Exercise],
    id: i32,
) -> Result<&Exercise, GrindItError> {
    // Result: we NEED this exercise, its absence is an error
    find_exercise(exercises, id)
        .ok_or_else(|| GrindItError::NotFound(format!("Exercise #{}", id)))
}

fn main() {
    let exercises = vec![
        Exercise { id: 1, name: "Back Squat".into() },
        Exercise { id: 2, name: "Deadlift".into() },
    ];

    // Option usage — "do we have it?"
    match find_exercise(&exercises, 3) {
        Some(ex) => println!("Found: {}", ex.name),
        None => println!("Not in the library (that's okay)"),
    }

    // Result usage — "we need it, failure is an error"
    match get_exercise_or_error(&exercises, 3) {
        Ok(ex) => println!("Found: {}", ex.name),
        Err(e) => println!("Error: {}", e),
    }
}
```

The bridge between them --- `.ok_or()` and `.ok()` --- is one of the most common patterns in Rust. You'll use it constantly in GrindIt: look something up with `Option`, convert to `Result` when its absence is an error.

---

## 9. Try It Yourself

**Exercise 1: Build a typed error system.** Define a `GrindItError` enum with at least four variants (Validation, NotFound, Database, Internal). Implement `Display`, `std::error::Error`, `From<ParseIntError>`, and `From<std::io::Error>`. Write a function that parses a string to an integer and reads a file, using `?` for both --- the error types should convert automatically.

**Exercise 2: Build an error chain.** Create three error types: `ConnectionError`, `QueryError` (which wraps `ConnectionError` via `source()`), and `AppError` (which wraps `QueryError`). Implement the `source()` method on each. Build a three-level chain and write a loop that walks `source()` to print all three levels.

**Exercise 3: Split user-facing from developer-facing messages.** Add a `user_message()` method to your error type that returns friendly strings for Validation and NotFound variants, but hides the real message behind "Something went wrong" for Database and Internal variants. Write a test function that creates one of each variant and asserts that Database errors never leak internal details to the user message.
