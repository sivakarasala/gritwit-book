// Chapter 4: Exercise CRUD
// Spotlight: Error Handling (Result, Option, ?)
//
// Server functions with validation, Option for edit mode, Result for errors.

use leptos::prelude::*;

/// Clean server function error messages for display
pub fn clean_error(e: &ServerFnError) -> String {
    let msg = e.to_string();
    // Strip "ServerFnError: " or "error running server fn: " prefixes
    msg.split(": ").last().unwrap_or(&msg).to_string()
}

#[server]
pub async fn create_exercise(
    name: String,
    category: String,
    scoring_type: String,
) -> Result<(), ServerFnError> {
    // Validation: name must not be empty
    if name.trim().is_empty() {
        return Err(ServerFnError::new("Exercise name cannot be empty"));
    }

    // Validation: category must be one of the allowed values
    let valid_categories = ["Weightlifting", "Gymnastics", "Monostructural"];
    if !valid_categories.contains(&category.as_str()) {
        return Err(ServerFnError::new(format!(
            "Invalid category: {}. Must be one of: {:?}",
            category, valid_categories
        )));
    }

    // In Ch 5, this becomes a real database insert
    println!("Created exercise: {} [{}]", name, category);
    Ok(())
}

#[server]
pub async fn update_exercise(
    id: i32,
    name: String,
    category: String,
    scoring_type: String,
) -> Result<(), ServerFnError> {
    if name.trim().is_empty() {
        return Err(ServerFnError::new("Exercise name cannot be empty"));
    }

    // Option pattern: check if exercise exists
    let exercise: Option<String> = None; // Placeholder — real DB lookup in Ch 5
    match exercise {
        Some(_) => {
            println!("Updated exercise {}: {} [{}]", id, name, category);
            Ok(())
        }
        None => Err(ServerFnError::new("Exercise not found")),
    }
}

#[server]
pub async fn delete_exercise(id: i32) -> Result<(), ServerFnError> {
    // Soft delete: set deleted_at timestamp instead of removing the row
    // Ownership check would verify current user created this exercise
    println!("Soft-deleted exercise {}", id);
    Ok(())
}
