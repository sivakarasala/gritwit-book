// Chapter 4 DSA Exercise: Null Safety with Option<T>
//
// Rust eliminates null pointer errors by encoding absence in the type system.
// Option<T> forces handling of the "not present" case at compile time.

/// A GrindIt exercise — some fields are optional.
#[derive(Debug, Clone)]
struct Exercise {
    name: String,
    category: String,
    scoring_type: String,
    description: Option<String>,
    demo_video_url: Option<String>,
    created_by: Option<String>,
    deleted_at: Option<String>,
}

/// A user profile with optional fields.
#[derive(Debug)]
struct Profile {
    display_name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
}

// ----------------------------------------------------------------
// Interview Problem 1: Option combinators
// Given a Profile, return the best greeting string.
// Priority: display_name > email > phone > "athlete"
// ----------------------------------------------------------------
fn greeting(profile: &Profile) -> String {
    let name = profile
        .display_name
        .as_ref()
        .or(profile.email.as_ref())
        .or(profile.phone.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("athlete");
    format!("Hello, {}!", name)
}

// ----------------------------------------------------------------
// Interview Problem 2: Safe data extraction with Result + Option
// Parse a JSON-like key-value string and extract a typed field.
// Demonstrates the ? operator chain that replaces nested null checks.
// ----------------------------------------------------------------
fn extract_weight(data: &str) -> Result<f64, String> {
    // Simulate: find "weight:" in the data
    let weight_str = data
        .split(',')
        .find(|s| s.trim().starts_with("weight:"))
        .ok_or_else(|| "Field 'weight' not found".to_string())?;

    let value_str = weight_str
        .split(':')
        .nth(1)
        .ok_or_else(|| "Malformed weight field".to_string())?;

    value_str
        .trim()
        .parse::<f64>()
        .map_err(|e| format!("Parse error: {}", e))
}

// ----------------------------------------------------------------
// Interview Problem 3: Soft delete with ownership check
// Returns Ok(()) if deletion is allowed, Err with reason otherwise.
// Demonstrates Option pattern matching for authorization.
// ----------------------------------------------------------------
fn can_delete(exercise: &Exercise, current_user: &str) -> Result<(), String> {
    // Already deleted?
    if exercise.deleted_at.is_some() {
        return Err(format!("'{}' is already deleted", exercise.name));
    }

    // Check ownership
    match &exercise.created_by {
        Some(creator) if creator == current_user => Ok(()),
        Some(creator) => Err(format!(
            "Cannot delete '{}': owned by '{}', you are '{}'",
            exercise.name, creator, current_user
        )),
        None => Err(format!(
            "Cannot delete '{}': no owner recorded (seed data)",
            exercise.name
        )),
    }
}

// ----------------------------------------------------------------
// Interview Problem 4: Flatten nested Options
// Common in database joins where multiple levels can be NULL.
// ----------------------------------------------------------------
fn get_exercise_video_extension(exercise: &Exercise) -> Option<String> {
    exercise
        .demo_video_url
        .as_ref()
        .and_then(|url| url.rsplit('.').next().map(|ext| ext.to_lowercase()))
}

// ----------------------------------------------------------------
// Interview Problem 5: Convert between Option and Result
// Demonstrates ok_or, ok_or_else, and the ? chain.
// ----------------------------------------------------------------
fn validate_and_parse(
    name: Option<&str>,
    weight_input: Option<&str>,
) -> Result<(String, f64), String> {
    let name = name
        .filter(|n| !n.trim().is_empty())
        .ok_or("Name is required")?
        .trim()
        .to_string();

    let weight = weight_input
        .ok_or("Weight input is required")?
        .trim()
        .parse::<f64>()
        .map_err(|e| format!("Invalid weight: {}", e))?;

    if weight <= 0.0 {
        return Err("Weight must be positive".to_string());
    }

    Ok((name, weight))
}

fn main() {
    println!("=== Null Safety with Option<T> ===\n");

    // 1. Greeting with Option combinators
    println!("--- Greeting (Option chaining) ---");
    let profiles = vec![
        Profile {
            display_name: Some("Coach Mike".to_string()),
            email: Some("mike@grindit.app".to_string()),
            phone: None,
        },
        Profile {
            display_name: None,
            email: Some("jane@grindit.app".to_string()),
            phone: None,
        },
        Profile {
            display_name: None,
            email: None,
            phone: Some("+1-555-0123".to_string()),
        },
        Profile {
            display_name: None,
            email: None,
            phone: None,
        },
    ];
    for p in &profiles {
        println!("  {:?} => {}", p, greeting(p));
    }

    // 2. Extract weight with ? chain
    println!("\n--- Extract Weight (Result + Option + ?) ---");
    let test_data = vec![
        "name: Back Squat, weight: 135.5, reps: 5",
        "name: Pull-Up, reps: 10",
        "name: Bad Data, weight: abc",
        "name: Deadlift, weight: 225",
    ];
    for data in test_data {
        match extract_weight(data) {
            Ok(w) => println!("  '{}' => weight: {} lbs", data, w),
            Err(e) => println!("  '{}' => ERROR: {}", data, e),
        }
    }

    // 3. Soft delete ownership check
    println!("\n--- Soft Delete with Ownership ---");
    let exercises = vec![
        Exercise {
            name: "Back Squat".to_string(),
            category: "Weightlifting".to_string(),
            scoring_type: "weight_and_reps".to_string(),
            description: Some("Barbell back squat".to_string()),
            demo_video_url: Some("https://cdn.grindit.app/videos/back-squat.mp4".to_string()),
            created_by: Some("coach_mike".to_string()),
            deleted_at: None,
        },
        Exercise {
            name: "Custom Move".to_string(),
            category: "Other".to_string(),
            scoring_type: "reps".to_string(),
            description: None,
            demo_video_url: None,
            created_by: Some("athlete_jane".to_string()),
            deleted_at: None,
        },
        Exercise {
            name: "Deleted Exercise".to_string(),
            category: "Weightlifting".to_string(),
            scoring_type: "weight_and_reps".to_string(),
            description: None,
            demo_video_url: None,
            created_by: Some("coach_mike".to_string()),
            deleted_at: Some("2024-01-15T10:30:00Z".to_string()),
        },
        Exercise {
            name: "Seed Exercise".to_string(),
            category: "Gymnastics".to_string(),
            scoring_type: "reps".to_string(),
            description: None,
            demo_video_url: None,
            created_by: None,
            deleted_at: None,
        },
    ];
    let current_user = "coach_mike";
    for ex in &exercises {
        match can_delete(ex, current_user) {
            Ok(()) => println!("  '{}': DELETE ALLOWED", ex.name),
            Err(reason) => println!("  '{}': DENIED - {}", ex.name, reason),
        }
    }

    // 4. Flatten nested Options
    println!("\n--- Video Extension (nested Option) ---");
    for ex in &exercises {
        let ext = get_exercise_video_extension(ex);
        println!("  '{}': video ext = {:?}", ex.name, ext);
    }

    // 5. Validate and parse
    println!("\n--- Validate and Parse (Option -> Result) ---");
    let cases: Vec<(Option<&str>, Option<&str>)> = vec![
        (Some("Back Squat"), Some("135.5")),
        (Some(""), Some("100")),
        (None, Some("100")),
        (Some("Deadlift"), None),
        (Some("Bench Press"), Some("-50")),
        (Some("Thruster"), Some("abc")),
    ];
    for (name, weight) in cases {
        match validate_and_parse(name, weight) {
            Ok((n, w)) => println!("  ({:?}, {:?}) => OK: {} @ {} lbs", name, weight, n, w),
            Err(e) => println!("  ({:?}, {:?}) => ERROR: {}", name, weight, e),
        }
    }

    // Key insight
    println!("\n=== Key Insight ===");
    println!("In Java/JS: exercise.getDescription().length() => NullPointerException");
    println!("In Rust:    exercise.description.len() => COMPILE ERROR");
    println!("            exercise.description.map(|s| s.len()).unwrap_or(0) => safe");
    println!("\nThe type system prevents null bugs at compile time.");
    println!("Option<T> is not a wrapper around null — it's an enum with two variants.");
    println!("The compiler forces you to handle both Some and None.");
}
