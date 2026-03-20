// Chapter 9: Workout Logging & Scoring
// Spotlight: Traits & Generics
//
// Scoring types, serde traits, callback prop patterns.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkoutLog {
    pub id: i32,
    pub user_id: i32,
    pub wod_id: i32,
    pub wod_date: chrono::NaiveDate,
    pub rx: bool,
    pub notes: Option<String>,
    pub sections: Vec<SectionScore>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SectionScore {
    pub section_id: i32,
    pub section_type: String,
    pub score_value: Option<String>,    // "12:34" for time, "5" for rounds, "225" for weight
    pub movements: Vec<MovementLog>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MovementLog {
    pub exercise_name: String,
    pub reps_completed: Option<i32>,
    pub weight_used: Option<String>,
}

/// Format a score for display based on section type (Strategy pattern)
pub fn format_score(section_type: &str, score_value: &str) -> String {
    match section_type {
        "ForTime" => {
            // Parse seconds into MM:SS
            if let Ok(seconds) = score_value.parse::<i32>() {
                format!("{}:{:02}", seconds / 60, seconds % 60)
            } else {
                score_value.to_string()
            }
        }
        "AMRAP" => format!("{} rounds", score_value),
        "Strength" => format!("{} lbs", score_value),
        "EMOM" => format!("{} completed", score_value),
        _ => score_value.to_string(),
    }
}

/// Compare two scores (higher is better for AMRAP/Strength, lower for ForTime)
pub fn is_better_score(section_type: &str, a: &str, b: &str) -> bool {
    match section_type {
        "ForTime" => {
            // Lower time is better
            let a_val: i32 = a.parse().unwrap_or(i32::MAX);
            let b_val: i32 = b.parse().unwrap_or(i32::MAX);
            a_val < b_val
        }
        _ => {
            // Higher value is better (AMRAP rounds, Strength weight)
            let a_val: i32 = a.parse().unwrap_or(0);
            let b_val: i32 = b.parse().unwrap_or(0);
            a_val > b_val
        }
    }
}
