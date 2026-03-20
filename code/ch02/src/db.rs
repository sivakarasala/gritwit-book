// Chapter 2: The Exercise Library
// Spotlight: Structs & impl Blocks
//
// Exercise struct with methods and hardcoded data.

#[derive(Clone, Debug)]
pub struct Exercise {
    pub name: String,
    pub category: String,
    pub scoring_type: String,
}

impl Exercise {
    pub fn new(name: &str, category: &str, scoring_type: &str) -> Self {
        Exercise {
            name: name.to_string(),
            category: category.to_string(),
            scoring_type: scoring_type.to_string(),
        }
    }

    pub fn summary(&self) -> String {
        format!("{} [{}] — {}", self.name, self.category, self.scoring_type)
    }

    pub fn is_weightlifting(&self) -> bool {
        self.category == "Weightlifting"
    }
}

pub fn get_exercises() -> Vec<Exercise> {
    vec![
        Exercise::new("Back Squat", "Weightlifting", "Weight"),
        Exercise::new("Deadlift", "Weightlifting", "Weight"),
        Exercise::new("Bench Press", "Weightlifting", "Weight"),
        Exercise::new("Clean and Jerk", "Weightlifting", "Weight"),
        Exercise::new("Snatch", "Weightlifting", "Weight"),
        Exercise::new("Pull-up", "Gymnastics", "Reps"),
        Exercise::new("Push-up", "Gymnastics", "Reps"),
        Exercise::new("Handstand Walk", "Gymnastics", "Distance"),
        Exercise::new("Muscle-up", "Gymnastics", "Reps"),
        Exercise::new("Running", "Monostructural", "Time"),
        Exercise::new("Rowing", "Monostructural", "Calories"),
        Exercise::new("Box Jump", "Monostructural", "Reps"),
    ]
}
