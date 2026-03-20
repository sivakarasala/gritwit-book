// Chapter 7: User Authentication
// Spotlight: Enums & Pattern Matching
//
// UserRole enum, match expressions, Display trait, auth patterns.

use std::fmt;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UserRole {
    Athlete,
    Coach,
    Admin,
}

impl UserRole {
    pub fn rank(&self) -> i32 {
        match self {
            UserRole::Athlete => 0,
            UserRole::Coach => 1,
            UserRole::Admin => 2,
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRole::Athlete => write!(f, "Athlete"),
            UserRole::Coach => write!(f, "Coach"),
            UserRole::Admin => write!(f, "Admin"),
        }
    }
}

/// First user registered becomes Admin; all others are Athletes.
pub fn default_role_for_new_user(user_count: i64) -> UserRole {
    if user_count == 0 {
        UserRole::Admin
    } else {
        UserRole::Athlete
    }
}

/// Auth method — demonstrates enum with different data per variant
pub enum AuthMethod {
    Google { access_token: String },
    Password { email: String, password_hash: String },
    Otp { phone: String, code: String },
}

impl AuthMethod {
    pub fn provider_name(&self) -> &str {
        match self {
            AuthMethod::Google { .. } => "Google",
            AuthMethod::Password { .. } => "Email/Password",
            AuthMethod::Otp { .. } => "Phone OTP",
        }
    }
}
