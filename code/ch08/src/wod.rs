// Chapter 8: WOD Programming
// Spotlight: Complex Data Structures & Relationships
//
// Nested Wod → WodSection → WodMovement tree structure.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Wod {
    pub id: i32,
    pub title: String,
    pub wod_date: chrono::NaiveDate,
    pub sections: Vec<WodSection>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WodSection {
    pub id: i32,
    pub wod_id: i32,
    pub section_type: String,       // "ForTime", "AMRAP", "Strength", "EMOM"
    pub time_cap_seconds: Option<i32>,
    pub rounds: Option<i32>,
    pub order_index: i32,
    pub movements: Vec<WodMovement>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct WodMovement {
    pub id: i32,
    pub section_id: i32,
    pub exercise_name: String,
    pub reps: Option<i32>,
    pub weight: Option<String>,     // "135 lbs", "bodyweight", "70%"
    pub distance: Option<String>,   // "400m", "100ft"
    pub order_index: i32,
}

impl Wod {
    /// Total number of movements across all sections (N-ary tree traversal)
    pub fn total_movements(&self) -> usize {
        self.sections.iter().map(|s| s.movements.len()).sum()
    }

    /// Flat list of all exercise names (DFS through the tree)
    pub fn all_exercise_names(&self) -> Vec<&str> {
        self.sections
            .iter()
            .flat_map(|section| section.movements.iter().map(|m| m.exercise_name.as_str()))
            .collect()
    }
}

impl WodSection {
    pub fn display_type(&self) -> &str {
        match self.section_type.as_str() {
            "ForTime" => "For Time",
            "AMRAP" => "AMRAP",
            "Strength" => "Strength",
            "EMOM" => "EMOM",
            other => other,
        }
    }
}
