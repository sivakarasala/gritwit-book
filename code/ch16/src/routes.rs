// Chapter 16: REST API Layer
// Spotlight: Axum Route Organization & API Design
//
// "Two doors, one database" — REST handlers call the same db.rs functions.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

/// Health check endpoint — returns 200 OK
pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

/// REST endpoint: list all exercises
/// Both this handler AND the Leptos server function call list_exercises_db()
pub async fn api_list_exercises() -> impl IntoResponse {
    match crate::db::list_exercises_db(crate::db::db()).await {
        Ok(exercises) => Json(exercises).into_response(),
        Err(e) => {
            tracing::error!("Failed to list exercises: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
        }
    }
}

/// Build the API router with versioned routes
pub fn api_routes() -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .nest(
            "/api/v1",
            Router::new()
                .route("/exercises", get(api_list_exercises))
                // .route("/exercises", post(api_create_exercise))
                // .route("/wods", get(api_list_wods))
                // .route("/workouts", get(api_list_workouts))
        )
}
