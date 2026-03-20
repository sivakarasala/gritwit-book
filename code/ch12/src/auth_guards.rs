// Chapter 12: Profile & Admin
// Spotlight: Authorization & Role-Based Access Control
//
// Guard functions with early return, role hierarchy.

use crate::auth::UserRole;

pub struct AuthenticatedUser {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub role: UserRole,
}

/// Require any authenticated user. Returns the user or a ServerFnError.
pub async fn require_auth() -> Result<AuthenticatedUser, leptos::prelude::ServerFnError> {
    // In production: extract session from tower-sessions, look up user
    // Here showing the pattern:
    Err(leptos::prelude::ServerFnError::new("Not authenticated"))
}

/// Require a minimum role level. Admin > Coach > Athlete.
pub async fn require_role(
    min_role: UserRole,
) -> Result<AuthenticatedUser, leptos::prelude::ServerFnError> {
    let user = require_auth().await?;

    if user.role.rank() < min_role.rank() {
        return Err(leptos::prelude::ServerFnError::new(format!(
            "Requires {} or higher",
            min_role
        )));
    }

    Ok(user)
}

/// Check if a user can modify a resource (creator or admin)
pub fn can_modify(user: &AuthenticatedUser, created_by: Option<i32>) -> bool {
    match user.role {
        UserRole::Admin => true, // Admin can modify anything
        _ => created_by == Some(user.id), // Others can only modify their own
    }
}
