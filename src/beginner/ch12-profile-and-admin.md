# Chapter 12: Profile & Admin

Users need to see who they are and change what they can. Admins need to see everyone and change what they should. This chapter builds the profile page (avatar, stats, edit form, password management) and the admin panel (user list, role management). Both pages are gated by authorization checks --- the profile requires any authenticated user, the admin page requires the Admin role.

The spotlight concept is **authorization and role-based access control (RBAC)** --- the pattern of defining roles with ordered permissions and enforcing them with guard functions. You will implement `require_auth()` and `require_role(min_role)` as async functions that short-circuit server functions before they reach any business logic. You will see how Rust enums model role hierarchies, how the first-user-is-admin pattern bootstraps the system, and how ownership checks ("only the creator or an admin can delete") compose with role checks.

By the end of this chapter, you will have:

- `require_auth()` and `require_role(min_role)` guard functions in `auth/session.rs`
- A `UserRole` enum with `Athlete < Coach < Admin` hierarchy and a `rank()` method
- `default_role_for_new_user()` that makes the first user an Admin
- A profile page with avatar (initials or Google photo), stats, edit form for name/email/phone/gender, and password management with Argon2
- An admin page with a user list, role badges, and promote/demote buttons
- Conditional UI rendering based on the current user's role

---

## Spotlight: Authorization & Role-Based Access Control

Before we write any code, let us understand the three core concepts that power this chapter.

> **Programming Concept: What is Authorization?**
>
> Authorization is deciding what a logged-in user is allowed to do. It is different from authentication (which is proving who you are).
>
> Think of it like entering a building:
> - **Authentication** = showing your ID at the front door. "Are you who you say you are?"
> - **Authorization** = checking your badge at each room. "Are you allowed in this room?"
>
> You authenticate once (logging in), but authorization happens on every action. A logged-in Athlete can log workouts but cannot delete other users. A logged-in Admin can do both. Same building, different access levels.
>
> In GrindIt, authentication was handled in Chapter 7 (login, session, cookies). This chapter handles authorization --- given that we know who the user is, which operations can they perform?

> **Programming Concept: What is a Guard?**
>
> A guard is a check at the door --- "are you allowed in?" If not, you are turned away before entering.
>
> ```rust
> // Without a guard: business logic runs regardless of permissions
> async fn delete_all_users() -> Result<(), Error> {
>     // Oops! Anyone can call this!
>     database.delete_all_users().await
> }
>
> // With a guard: permission check happens first
> async fn delete_all_users() -> Result<(), Error> {
>     require_role(Admin).await?;  // <-- the guard
>     // Only runs if the user is an Admin
>     database.delete_all_users().await
> }
> ```
>
> The `?` operator is the key. If the guard fails (returns an error), the `?` immediately returns that error to the caller. The business logic below the guard never executes. It is like a bouncer at a club --- if you do not have the right credentials, you never get past the door.
>
> Guards compose naturally. You can stack them:
>
> ```rust
> require_auth().await?;           // Must be logged in
> require_role(Coach).await?;      // Must be at least a Coach
> require_owner(resource).await?;  // Must own this resource
> ```
>
> Each guard checks one thing. If any fails, execution stops immediately.

> **Programming Concept: What is a Role Hierarchy?**
>
> A role hierarchy is levels of permission, like military ranks. Each rank can do everything the ranks below can do, plus more.
>
> ```
> Admin    (rank 2) --- can manage users, delete anything, see everything
>   |
> Coach    (rank 1) --- can create exercises and WODs, plus everything Athletes can do
>   |
> Athlete  (rank 0) --- can log workouts, view exercises, see own history
> ```
>
> The check is simple: "is your rank high enough?" If a page requires Coach level (rank 1), then:
> - Athlete (rank 0): denied
> - Coach (rank 1): allowed
> - Admin (rank 2): allowed
>
> This is called a **total ordering** --- every role is comparable to every other role. Admin always beats Coach, Coach always beats Athlete.

### The UserRole enum

Roles are modeled as a Rust enum with a `rank()` method:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Athlete,
    Coach,
    Admin,
}

impl UserRole {
    pub fn rank(&self) -> u8 {
        match self {
            UserRole::Athlete => 0,
            UserRole::Coach => 1,
            UserRole::Admin => 2,
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Athlete => write!(f, "athlete"),
            UserRole::Coach => write!(f, "coach"),
            UserRole::Admin => write!(f, "admin"),
        }
    }
}
```

Why use `rank()` instead of deriving `Ord` (which would let you compare roles directly with `<` and `>`)? Because `Ord` depends on the order you declare the enum variants. If someone reorders the variants, the permission hierarchy would silently change. The explicit `rank()` method makes the ordering intentional and documented.

### The AuthUser struct

The authenticated user carries their identity and role:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub gender: Option<String>,
}

impl AuthUser {
    pub fn initials(&self) -> String {
        self.display_name
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }

    pub fn identifier(&self) -> &str {
        self.email.as_deref()
            .or(self.phone.as_deref())
            .unwrap_or("")
    }
}
```

Two helper methods worth understanding:

**`initials()`** extracts up to two initials from the display name for the avatar circle. Let us trace through it with "John Doe":
1. `split_whitespace()` gives `["John", "Doe"]`
2. `filter_map(|w| w.chars().next())` takes the first character of each word: `['J', 'D']`
3. `take(2)` limits to two characters (handles names with three or more words)
4. `collect::<String>()` combines into `"JD"`
5. `to_uppercase()` ensures consistency

**`identifier()`** returns the best available identifier. `as_deref()` converts `Option<String>` to `Option<&str>` (borrowing instead of cloning). `.or()` chains options: try email first, fall back to phone, default to empty string.

### The guard functions

Guard functions are async functions that extract the current user from the session and validate their permissions:

```rust
pub async fn require_auth() -> Result<AuthUser, ServerFnError> {
    get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))
}

pub async fn require_role(min_role: UserRole) -> Result<AuthUser, ServerFnError> {
    let user = require_auth().await?;
    if user.role.rank() >= min_role.rank() {
        Ok(user)
    } else {
        Err(ServerFnError::new("Insufficient permissions"))
    }
}
```

Let us read these carefully:

**`require_auth()`** calls `get_current_user()`, which looks up the session in the database. It returns `Result<Option<AuthUser>>` --- a `Result` wrapping an `Option`:
- `Ok(Some(user))` --- session exists, user found
- `Ok(None)` --- no session (not logged in)
- `Err(...)` --- something went wrong (database error)

The `?` propagates database errors. Then `ok_or_else` converts `None` (no user) into an error.

**`require_role(min_role)`** calls `require_auth()` first. If the user is not logged in, they see "Unauthorized" (not "Insufficient permissions"). This is important --- unauthenticated users should not know what permissions they need. Only after confirming the user is logged in do we check their role rank.

This two-layer approach means every protected server function starts with a single line:

```rust
#[server]
async fn some_admin_action() -> Result<(), ServerFnError> {
    crate::auth::session::require_role(UserRole::Admin).await?;
    // ... only runs if the user is an admin
}

#[server]
async fn some_coach_action() -> Result<(), ServerFnError> {
    crate::auth::session::require_role(UserRole::Coach).await?;
    // ... runs for coaches AND admins (rank >= 1)
}

#[server]
async fn some_athlete_action() -> Result<(), ServerFnError> {
    let user = crate::auth::session::require_auth().await?;
    // ... runs for any authenticated user
}
```

### The auth_context helper

Many server functions need the user, the database pool, and the parsed UUID. The `auth_context()` helper bundles all three:

```rust
pub async fn auth_context() -> Result<(AuthUser, sqlx::PgPool, uuid::Uuid), ServerFnError> {
    let user = require_auth().await?;
    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = user.id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
    Ok((user, pool, user_uuid))
}
```

This eliminates three lines of boilerplate from every server function:

```rust
// Before: repetitive
let user = crate::auth::session::require_auth().await?;
let pool = crate::db::db().await?;
let user_uuid: uuid::Uuid = user.id.parse()
    .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

// After: one line
let (user, pool, user_uuid) = crate::auth::session::auth_context().await?;
```

This is called the **DRY principle** (Don't Repeat Yourself). When you see the same three lines in every server function, extract them into a helper.

### First user is Admin

When the app is first deployed, there are no admins to create other admins. This is the **bootstrap problem**. The solution is simple --- the first user to sign up becomes the Admin:

```rust
pub async fn default_role_for_new_user(
    pool: &sqlx::PgPool,
) -> Result<&'static str, ServerFnError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(if count.0 == 0 { "admin" } else { "athlete" })
}
```

If the users table is empty, the new user gets "admin". Otherwise, they get "athlete". This function is called during every registration flow --- email signup, OTP login, and Google OAuth.

Notice the return type is `&'static str` rather than `UserRole`. The string is interpolated directly into SQL. The literals `"admin"` and `"athlete"` have `'static` lifetime --- they are baked into the program binary and live forever.

### DSA connection: Bitmask permissions

Production permission systems often use **bitmasks** instead of role hierarchies. Each permission is a bit:

```rust
const READ: u32    = 0b0001;  // 1
const WRITE: u32   = 0b0010;  // 2
const DELETE: u32  = 0b0100;  // 4
const ADMIN: u32   = 0b1000;  // 8

// Athlete: read + write
const ATHLETE_PERMS: u32 = READ | WRITE;        // 0b0011 = 3

// Coach: read + write + delete
const COACH_PERMS: u32 = READ | WRITE | DELETE;  // 0b0111 = 7

// Check permission
fn has_permission(user_perms: u32, required: u32) -> bool {
    (user_perms & required) == required
}
```

What is happening with the `|` and `&` operators? These are **bitwise** operations:
- `|` (OR) combines permissions: `READ | WRITE` turns on both the read and write bits
- `&` (AND) checks if specific bits are set: `user_perms & required` keeps only the bits that are in both

Bitmasks are more flexible than a linear hierarchy --- they allow arbitrary permission combinations (a user who can delete but not write, for example). GrindIt's three-role hierarchy is simpler and sufficient, but the bitmask pattern appears in Unix file permissions (`chmod 755`), AWS IAM policies, and Discord role systems.

---

## Building the Profile Page

### Profile data aggregation

The profile page shows the user's info, stats, and edit controls. A single server function loads everything:

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ProfileData {
    pub user: AuthUser,
    pub workouts: i64,
    pub streak: i64,
    pub has_password: bool,
}

#[server]
async fn get_profile() -> Result<ProfileData, ServerFnError> {
    let (user, pool, user_uuid) = crate::auth::session::auth_context().await?;

    let workouts = crate::db::count_workouts_db(&pool, user_uuid)
        .await.unwrap_or(0);
    let streak = crate::db::streak_days_db(&pool, user_uuid)
        .await.unwrap_or(0);

    let has_password: (bool,) =
        sqlx::query_as("SELECT password_hash IS NOT NULL FROM users WHERE id = $1")
            .bind(user_uuid)
            .fetch_one(&pool)
            .await
            .unwrap_or((false,));

    Ok(ProfileData {
        user, workouts, streak,
        has_password: has_password.0,
    })
}
```

The `has_password` check determines whether to show "Set Password" (for users who signed up via Google OAuth or OTP) or "Update Password" (for users who already have one). The SQL `password_hash IS NOT NULL` returns a boolean directly --- no Rust logic needed.

### The avatar display

The profile avatar shows the user's Google photo if available, otherwise falls back to initials:

```rust
<div class="profile-avatar">
    {if let Some(ref url) = data.user.avatar_url {
        view! {
            <img src={url.clone()} class="profile-avatar-img"
                 referrerpolicy="no-referrer"/>
        }.into_any()
    } else {
        view! {
            <span class="profile-avatar-initials">{data.user.initials()}</span>
        }.into_any()
    }}
</div>
```

The `referrerpolicy="no-referrer"` attribute prevents the browser from sending a Referer header when loading the Google avatar URL --- a privacy best practice that prevents Google from knowing which page the user is on.

Notice `if let Some(ref url)` instead of `if let Some(url)`. The `ref` keyword borrows the URL instead of moving it out of `data.user.avatar_url`. Without `ref`, we would take ownership and `data.user` would be partially invalid.

### The edit form with SingleSelect

The profile form uses signals for each editable field and the `SingleSelect` component for gender:

```rust
let name = RwSignal::new(data.user.display_name.clone());
let email = RwSignal::new(data.user.email.clone().unwrap_or_default());
let phone = RwSignal::new(data.user.phone.clone().unwrap_or_default());
let gender = RwSignal::new(data.user.gender.clone().unwrap_or_default());

// Gender dropdown using SingleSelect
<SingleSelect
    options=vec![
        SelectOption { value: "".to_string(), label: "Not set".to_string() },
        SelectOption { value: "male".to_string(), label: "Male".to_string() },
        SelectOption { value: "female".to_string(), label: "Female".to_string() },
    ]
    selected=gender
    placeholder="Not set"
/>
```

Each signal is initialized from the loaded data. When the user types in a field, the signal updates. When they click Save, we read the signal values and send them to the server.

The form does not use a `<form>` tag with `on:submit`. Instead, the save button dispatches a manual async call using `spawn_local`:

```rust
let on_save_profile = move |_| {
    profile_saving.set(true);
    profile_saved.set(false);
    profile_error.set(None);
    let n = name.get_untracked();
    let e = email.get_untracked();
    let p = phone.get_untracked();
    let g = gender.get_untracked();
    leptos::task::spawn_local(async move {
        match update_profile(n, e, p, g).await {
            Ok(_) => {
                profile_saving.set(false);
                profile_saved.set(true);
                set_timeout(
                    move || profile_saved.set(false),
                    std::time::Duration::from_secs(2),
                );
            }
            Err(e) => {
                profile_saving.set(false);
                profile_error.set(Some(clean_error(&e)));
            }
        }
    });
};
```

Three states managed with signals:
- `profile_saving` --- disables the button and shows "Saving..."
- `profile_saved` --- shows a checkmark for 2 seconds after success
- `profile_error` --- displays the error message below the form

The `set_timeout` after success auto-clears the "Saved!" message after 2 seconds --- a nice UX touch. The `clean_error` function strips the `"error running server function: "` prefix from `ServerFnError` messages to show user-friendly text.

### Server-side validation

The `update_profile` server function validates inputs before touching the database:

```rust
#[server]
async fn update_profile(
    display_name: String,
    email: String,
    phone: String,
    gender: String,
) -> Result<(), ServerFnError> {
    let (_user, pool, user_uuid) = crate::auth::session::auth_context().await?;

    let name = crate::auth::validate_name(&display_name)?;
    let email_val = crate::auth::validate_email(&email)?;
    let email_opt = email_val.as_deref();

    let phone_val = phone.trim().to_string();
    let phone_opt = if phone_val.is_empty() { None } else { Some(phone_val.as_str()) };
    let gender_opt = if gender.is_empty() { None } else { Some(gender.as_str()) };

    crate::db::update_user_profile_db(
        &pool, user_uuid, &name, email_opt, phone_opt, gender_opt
    ).await.map_err(|e| {
        let msg = e.to_string();
        if msg.contains("unique") || msg.contains("duplicate") {
            if msg.contains("email") {
                ServerFnError::new("This email is already linked to another account")
            } else if msg.contains("phone") {
                ServerFnError::new("This phone number is already linked to another account")
            } else {
                ServerFnError::new("This value is already in use by another account")
            }
        } else {
            ServerFnError::new("Failed to update profile")
        }
    })?;

    Ok(())
}
```

Look at the error mapping at the bottom. When the database rejects a duplicate email or phone (a unique constraint violation), the raw error message is something like `"duplicate key violates unique constraint users_email_key"`. That is not user-friendly. The error mapper checks for keywords like "unique" and "email" and returns a clear message like "This email is already linked to another account."

The validation functions follow an important pattern:

```rust
pub fn validate_email(email: &str) -> Result<Option<String>, ServerFnError> {
    let email = email.trim().to_lowercase();
    if email.is_empty() {
        return Ok(None);  // empty email is valid (user might use phone)
    }
    if !email.contains('@') || !email.contains('.') {
        return Err(ServerFnError::new("Invalid email address"));
    }
    Ok(Some(email))
}
```

Notice the return type: `Result<Option<String>>`. This is a three-state return:
- `Ok(None)` --- no email provided (valid, user uses phone instead)
- `Ok(Some(email))` --- valid email, cleaned and lowercased
- `Err(...)` --- invalid email

This three-state pattern is common in Rust for optional fields that need validation. The `Result` handles errors, the `Option` handles "not provided."

### Password management with Argon2

The password section lets users set or change their password:

```rust
#[server]
async fn set_password(password: String) -> Result<(), ServerFnError> {
    let (user, pool, user_uuid) = crate::auth::session::auth_context().await?;

    crate::auth::validate_password(&password)?;

    if user.email.is_none() {
        return Err(ServerFnError::new(
            "Please add your email in profile first, then set a password",
        ));
    }

    let hash = crate::auth::hash_password(&password)?;

    sqlx::query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2")
        .bind(&hash)
        .bind(user_uuid)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
```

The `hash_password` function uses Argon2 --- the current best practice for password hashing:

```rust
pub fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

Why not just store the password directly? Because if the database is ever compromised, all passwords would be exposed. Hashing is a one-way transformation:
- `"mypassword123"` becomes `"$argon2id$v=19$m=19456,t=2,p=1$abc123...xyz789"`
- You cannot reverse the hash to get the original password
- To verify a login, you hash the attempt and compare hashes

Argon2 is deliberately slow --- it takes ~100ms to hash a password. This is a feature, not a bug: it makes brute-force attacks impractical. An attacker trying millions of passwords would need years instead of seconds.

The **salt** ensures that identical passwords produce different hashes. Without a salt, all users with password "password123" would have the same hash, and an attacker could crack all of them at once.

The client-side form validates that the password and confirmation match before sending:

```rust
let on_set_password = move |_| {
    let pw = new_password.get_untracked();
    let confirm = confirm_password.get_untracked();
    if pw != confirm {
        pw_result.set(Some(Err("Passwords do not match".to_string())));
        return;
    }
    pw_saving.set(true);
    pw_result.set(None);
    leptos::task::spawn_local(async move {
        match set_password(pw).await {
            Ok(_) => {
                pw_saving.set(false);
                pw_result.set(Some(Ok(())));
                new_password.set(String::new());
                confirm_password.set(String::new());
                has_password.set(true);
                set_timeout(move || pw_result.set(None),
                    std::time::Duration::from_secs(3));
            }
            Err(e) => {
                pw_saving.set(false);
                pw_result.set(Some(Err(clean_error(&e))));
            }
        }
    });
};
```

The `pw_result` signal is `RwSignal<Option<Result<(), String>>>` --- a three-state signal:
- `None` --- no result yet (initial state, or cleared after timeout)
- `Some(Ok(()))` --- success
- `Some(Err(msg))` --- failure with an error message

This pattern avoids the need for separate `success` and `error` signals. The `matches!()` macro checks the nested enum state cleanly:

```rust
class:btn--success=move || matches!(pw_result.get(), Some(Ok(())))
```

---

## Building the Admin Page

### The admin server functions

The admin page requires the Admin role. If a non-admin somehow navigates to the admin URL, the server function rejects the request:

```rust
#[server]
async fn list_all_users() -> Result<Vec<AuthUser>, ServerFnError> {
    crate::auth::session::require_role(UserRole::Admin).await?;
    let pool = crate::db::db().await?;
    crate::db::list_users_db(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn change_user_role(
    user_id: String,
    new_role: String,
) -> Result<(), ServerFnError> {
    crate::auth::session::require_role(UserRole::Admin).await?;
    let pool = crate::db::db().await?;
    let uid: uuid::Uuid = user_id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
    if !["athlete", "coach", "admin"].contains(&new_role.as_str()) {
        return Err(ServerFnError::new("Invalid role"));
    }
    crate::db::update_user_role_db(&pool, uid, &new_role)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

The `change_user_role` function validates the role string against a whitelist: `["athlete", "coach", "admin"]`. Even though the UI only offers valid options, server-side validation is mandatory. A malicious user could use browser developer tools or a custom HTTP client to send any value. This is the **never trust the client** principle --- always validate on the server.

### The AdminPage component

The admin page loads users and renders them in a list:

```rust
#[derive(Clone, Copy, PartialEq)]
enum AdminTab {
    Users,
    Exercises,
}

#[component]
pub fn AdminPage() -> impl IntoView {
    let change_action = ServerAction::<ChangeUserRole>::new();
    let users = Resource::new(
        move || change_action.version().get(),
        |_| list_all_users(),
    );

    view! {
        <div class="admin-page">
            <AdminNav tab=AdminTab::Users/>
            <Suspense fallback=|| view! { <p class="loading">"Loading users..."</p> }>
                {move || {
                    users.get().map(|result| {
                        match result {
                            Ok(list) => view! {
                                <div class="users-list">
                                    {list.into_iter().map(|user| {
                                        view! {
                                            <UserRow user=user change_action=change_action/>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any(),
                            Err(e) => view! {
                                <p class="error">{format!("Error: {}", e)}</p>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
```

The `Resource` depends on `change_action.version()` --- every time a role change completes, the version number increments, and the user list refetches. This is the same pattern used in the history page for delete-then-refresh.

### The UserRow component

Each user row shows their avatar, name, role badge, and promote/demote buttons:

```rust
#[component]
pub fn UserRow(
    user: AuthUser,
    change_action: ServerAction<ChangeUserRole>,
) -> impl IntoView {
    let uid = user.id.clone();
    let role_str = user.role.to_string();
    let user_ident = user.identifier().to_string();

    view! {
        <div class="user-row">
            <div class="user-avatar">{user.initials()}</div>
            <div class="user-info">
                <span class="user-name">{user.display_name}</span>
                <span class="user-email">{user_ident}</span>
            </div>
            <div class="user-role-controls">
                <span class={format!("role-badge role-badge--{}", role_str)}>
                    {role_str.to_uppercase()}
                </span>
                {(user.role != UserRole::Admin).then(|| {
                    let uid_promote = uid.clone();
                    let uid_demote = uid.clone();
                    let is_coach = user.role == UserRole::Coach;
                    view! {
                        <div class="role-actions">
                            {(!is_coach).then(|| {
                                view! {
                                    <button class="role-btn role-btn--promote"
                                        disabled=move || change_action.pending().get()
                                        on:click=move |_| {
                                            change_action.dispatch(ChangeUserRole {
                                                user_id: uid_promote.clone(),
                                                new_role: "coach".to_string(),
                                            });
                                        }
                                    >"Make Coach"</button>
                                }
                            })}
                            {is_coach.then(|| {
                                view! {
                                    <button class="role-btn role-btn--demote"
                                        disabled=move || change_action.pending().get()
                                        on:click=move |_| {
                                            change_action.dispatch(ChangeUserRole {
                                                user_id: uid_demote.clone(),
                                                new_role: "athlete".to_string(),
                                            });
                                        }
                                    >"Demote"</button>
                                }
                            })}
                        </div>
                    }
                })}
            </div>
        </div>
    }
}
```

The conditional logic is layered --- let us read it carefully:

1. **Admin users get no buttons** --- `(user.role != UserRole::Admin).then(...)` hides the controls for admins entirely. You cannot demote yourself.
2. **Athletes get "Make Coach"** --- `(!is_coach).then(...)` shows the promote button for athletes (not coaches).
3. **Coaches get "Demote"** --- `is_coach.then(...)` shows the demote button for coaches.

The `uid` string is cloned twice: `uid_promote` and `uid_demote`. Each clone is moved into a different button's closure. This is the clone-before-move pattern from Chapter 11 --- two closures each need their own copy of the user ID.

The `change_action` prop is `ServerAction<ChangeUserRole>`, which is `Copy` internally (it is a signal). Both button closures capture it without cloning.

The `disabled=move || change_action.pending().get()` disables both buttons while a role change is in progress, preventing double-clicks.

---

## Rust Gym

### Guard patterns with early return

```rust
// The guard pattern: validate, then proceed
async fn guarded_operation() -> Result<String, ServerFnError> {
    let user = require_auth().await?;           // guard 1: must be logged in
    require_role(UserRole::Coach).await?;        // guard 2: must be coach+
    let pool = crate::db::db().await?;           // guard 3: db must be available

    // Business logic only runs if all guards pass
    Ok("success".to_string())
}
```

Each `?` is a potential early exit. If any guard fails, the function returns immediately with the error. The business logic at the bottom is only reached if every guard passes. Think of it like a series of checkpoints: ID check, security scan, badge verification --- only then do you enter the building.

<details>
<summary>Exercise: implement a guard that checks ownership</summary>

Write a function `require_owner_or_admin` that checks if the current user either created the resource or is an admin.

```rust
pub async fn require_owner_or_admin(
    resource_creator_id: &str,
) -> Result<AuthUser, ServerFnError> {
    let user = require_auth().await?;
    if user.role == UserRole::Admin || user.id == resource_creator_id {
        Ok(user)
    } else {
        Err(ServerFnError::new("You don't have permission to modify this resource"))
    }
}
```

Usage:

```rust
#[server]
async fn delete_exercise(id: String) -> Result<(), ServerFnError> {
    let pool = crate::db::db().await?;
    let exercise = get_exercise_by_id(&pool, &id).await?;
    let creator = exercise.created_by.as_deref().unwrap_or("");
    let _user = require_owner_or_admin(creator).await?;
    // ... proceed with deletion
    Ok(())
}
```

This composes the role check (Admin?) with the ownership check (creator?) using `||` (OR). Either condition is sufficient.
</details>

### Role hierarchy as enum ordering

<details>
<summary>Exercise: extend the role system with a new "Manager" role</summary>

Add a `Manager` role between Coach and Admin. Update the `rank()` method and the admin UI.

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Athlete,
    Coach,
    Manager,
    Admin,
}

impl UserRole {
    pub fn rank(&self) -> u8 {
        match self {
            UserRole::Athlete => 0,
            UserRole::Coach => 1,
            UserRole::Manager => 2,
            UserRole::Admin => 3,
        }
    }
}
```

Because `require_role` uses `>=` comparison on ranks, all existing checks still work:
- `require_role(Coach)` (rank 1): allows Coach (1), Manager (2), and Admin (3)
- `require_role(Admin)` (rank 3): only allows Admin (3)
- A new `require_role(Manager)` (rank 2): allows Manager (2) and Admin (3)

This is the advantage of the numeric rank approach --- new roles slot in without changing existing guard calls.
</details>

### Validation chain pattern

<details>
<summary>Exercise: write a validation function that returns a cleaned value or an error</summary>

```rust
fn validate_phone(phone: &str) -> Result<Option<String>, ServerFnError> {
    let phone = phone.trim().to_string();
    if phone.is_empty() {
        return Ok(None);  // empty is fine --- user might not have a phone
    }
    // Must start with + and contain only digits after that
    if !phone.starts_with('+') {
        return Err(ServerFnError::new("Phone must start with country code (e.g., +91)"));
    }
    if phone.len() < 10 {
        return Err(ServerFnError::new("Phone number is too short"));
    }
    if !phone[1..].chars().all(|c| c.is_ascii_digit()) {
        return Err(ServerFnError::new("Phone must contain only digits after the +"));
    }
    Ok(Some(phone))
}
```

This follows the same `Result<Option<String>>` pattern as `validate_email`:
- `Ok(None)` --- empty input, that is fine
- `Ok(Some(phone))` --- valid phone, cleaned and returned
- `Err(...)` --- invalid phone, rejected with a clear message

Notice how each validation check returns early with `Err` if it fails. Only if all checks pass does execution reach the final `Ok(Some(phone))`. This is the guard pattern applied to data validation.
</details>

---

## Exercises

### Exercise 1: Implement require_auth() and require_role(min_role)

Implement the two guard functions in `auth/session.rs`. `require_auth()` should extract the current user from the session and return `Err("Unauthorized")` if no session exists. `require_role(min_role)` should call `require_auth()` first, then compare the user's role rank against the minimum required rank.

<details>
<summary>Hints</summary>

- `get_current_user()` returns `Result<Option<AuthUser>>` --- use `?` to propagate the outer Result, then `ok_or_else` to convert None to an error
- `require_role` calls `require_auth` first (so unauthenticated users see "Unauthorized", not "Insufficient permissions")
- Use `user.role.rank() >= min_role.rank()` for the comparison
- Also implement `auth_context()` that bundles user + pool + UUID parsing
</details>

<details>
<summary>Solution</summary>

```rust
use super::{AuthUser, UserRole};
use leptos::prelude::*;
use tower_sessions::Session;

const USER_ID_KEY: &str = "user_id";

pub async fn get_session() -> Result<Session, ServerFnError> {
    let session: Session = leptos_axum::extract().await
        .map_err(|e| ServerFnError::new(format!("Session extraction failed: {}", e)))?;
    Ok(session)
}

pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    let session = get_session().await?;
    let user_id: Option<String> = session.get(USER_ID_KEY).await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let Some(uid) = user_id else { return Ok(None); };

    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = uid.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;

    match crate::db::get_user_by_id(&pool, user_uuid).await {
        Ok(u) => Ok(Some(u)),
        Err(_) => Ok(None),
    }
}

pub async fn require_auth() -> Result<AuthUser, ServerFnError> {
    get_current_user().await?
        .ok_or_else(|| ServerFnError::new("Unauthorized"))
}

pub async fn require_role(min_role: UserRole) -> Result<AuthUser, ServerFnError> {
    let user = require_auth().await?;
    if user.role.rank() >= min_role.rank() {
        Ok(user)
    } else {
        Err(ServerFnError::new("Insufficient permissions"))
    }
}

pub async fn auth_context() -> Result<(AuthUser, sqlx::PgPool, uuid::Uuid), ServerFnError> {
    let user = require_auth().await?;
    let pool = crate::db::db().await?;
    let user_uuid: uuid::Uuid = user.id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
    Ok((user, pool, user_uuid))
}
```

The layered design: `get_current_user` is the low-level function that queries the session and database. `require_auth` wraps it to turn `None` into an error. `require_role` wraps `require_auth` to add the rank check. Each layer adds one validation step, and each builds on the layer below.
</details>

### Exercise 2: Build the profile page with avatar, stats, and edit form

Build the `ProfilePage` component. It should load the user's data, display an avatar (Google photo or initials), show workout count and streak stats, and provide an edit form for name, email, phone, and gender (using `SingleSelect`).

<details>
<summary>Hints</summary>

- Use `Resource::new(|| (), |_| get_profile())` to load profile data on mount
- Split into `ProfilePage` (handles loading) and `ProfileContent` (handles display)
- Initialize signal values from the loaded data: `RwSignal::new(data.user.display_name.clone())`
- Use `spawn_local` for the save action, with saving/saved/error states
- Use `set_timeout` to auto-clear the success state after 2 seconds
- Use `clean_error` to strip the ServerFnError prefix for user-friendly messages
</details>

<details>
<summary>Solution</summary>

The full implementation is in `src/pages/profile/mod.rs`. The key patterns:

1. **Two-component split**: `ProfilePage` handles the `Suspense` and error states; `ProfileContent` receives the loaded data and renders the form.
2. **Signal initialization from data**: each editable field gets a `RwSignal` initialized from the loaded `ProfileData`.
3. **Manual async dispatch**: `spawn_local(async move { ... })` replaces `ServerAction` for finer control over loading/success/error states.
4. **Error mapping**: the `update_profile` server function maps database constraint violations to user-friendly messages ("This email is already linked to another account").

```rust
// Pattern: three-state save button
<button
    class="profile-save-btn"
    class:btn--loading=move || profile_saving.get()
    class:btn--success=move || profile_saved.get()
    disabled=move || profile_saving.get()
    on:click=on_save_profile
>
    {move || if profile_saved.get() {
        "Saved!".to_string()
    } else if profile_saving.get() {
        "Saving...".to_string()
    } else {
        "Save Profile".to_string()
    }}
</button>
```
</details>

### Exercise 3: Add password change with Argon2 validation

Add a password management section to the profile page. Users without a password see "Set Password"; users with one see "Update Password". Validate that the password and confirmation match on the client side, then hash with Argon2 on the server.

<details>
<summary>Hints</summary>

- Track `has_password: RwSignal<bool>` initialized from `data.has_password`
- Use `pw_result: RwSignal<Option<Result<(), String>>>` for the three-state result
- Client-side: compare `new_password` and `confirm_password` before dispatching
- Server-side: call `validate_password` (min 8 chars, max 128), then `hash_password`
- Require email before allowing password set (for password reset flows)
- Clear the password fields and update `has_password` on success
</details>

<details>
<summary>Solution</summary>

```rust
#[server]
async fn set_password(password: String) -> Result<(), ServerFnError> {
    let (user, pool, user_uuid) = crate::auth::session::auth_context().await?;
    crate::auth::validate_password(&password)?;

    if user.email.is_none() {
        return Err(ServerFnError::new(
            "Please add your email in profile first, then set a password",
        ));
    }

    let hash = crate::auth::hash_password(&password)?;
    sqlx::query("UPDATE users SET password_hash = $1, updated_at = now() WHERE id = $2")
        .bind(&hash)
        .bind(user_uuid)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
```

The Argon2 hash function:

```rust
pub fn hash_password(password: &str) -> Result<String, ServerFnError> {
    use argon2::{
        password_hash::{rand_core::OsRng, SaltString},
        Argon2, PasswordHasher,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

The client-side validation check prevents unnecessary server round-trips when passwords do not match. The server still validates independently --- never trust the client.
</details>

### Exercise 4: Build the admin page with user list and role management

Build the `AdminPage` component that lists all users (requires Admin role). Each user row shows their avatar, name, email, role badge, and promote/demote buttons. The role change should be guarded by `require_role(Admin)` on the server.

<details>
<summary>Hints</summary>

- Use `ServerAction::<ChangeUserRole>::new()` for the role change action
- The `Resource` depends on `change_action.version()` to refetch after changes
- Validate the role string on the server: `["athlete", "coach", "admin"].contains(&new_role.as_str())`
- Hide promote/demote buttons for Admin users (you cannot change your own role)
- Athletes get "Make Coach", Coaches get "Demote"
- Disable buttons while the action is pending: `disabled=move || change_action.pending().get()`
</details>

<details>
<summary>Solution</summary>

The `AdminPage` uses `list_all_users()` guarded by `require_role(Admin)`. The `UserRow` component conditionally renders buttons based on the user's current role.

```rust
#[server]
async fn list_all_users() -> Result<Vec<AuthUser>, ServerFnError> {
    crate::auth::session::require_role(UserRole::Admin).await?;
    let pool = crate::db::db().await?;
    crate::db::list_users_db(&pool).await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
pub async fn change_user_role(
    user_id: String,
    new_role: String,
) -> Result<(), ServerFnError> {
    crate::auth::session::require_role(UserRole::Admin).await?;
    let pool = crate::db::db().await?;
    let uid: uuid::Uuid = user_id.parse()
        .map_err(|e: uuid::Error| ServerFnError::new(e.to_string()))?;
    if !["athlete", "coach", "admin"].contains(&new_role.as_str()) {
        return Err(ServerFnError::new("Invalid role"));
    }
    crate::db::update_user_role_db(&pool, uid, &new_role).await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
```

The `UserRow` component uses nested `bool::then()` calls for the conditional button rendering. The pattern `(user.role != UserRole::Admin).then(|| { ... })` hides all controls for admin users. Inside, `(!is_coach).then()` and `is_coach.then()` select between promote and demote buttons.

See the full implementations in `src/pages/admin/mod.rs` and `src/pages/admin/user_row.rs`.
</details>

---

### 🧬 DS Deep Dive

50 request threads all reaching for the same data. This deep dive builds Mutex and RwLock from scratch with atomics, shows the RAII guard pattern, and demonstrates deadlock with a real GrindIt scenario.

**→ [Mutex & RwLock — "The Single-Occupancy Bathroom vs The Gym Floor"](../ds-narratives/ch12-locks-mutex-rwlock.md)**

---

## Summary

This chapter introduced role-based access control as a composition of simple building blocks:

- **Authorization** decides what a logged-in user can do (different from authentication, which proves who they are)
- **Guards** are checks at the door that reject unauthorized access before business logic runs
- **Role hierarchies** use ordered ranks so each level inherits everything below it
- **`UserRole` enum with `rank()`** --- a total ordering on roles that enables `>=` comparison
- **`require_auth()` and `require_role(min_role)`** --- layered guard functions that short-circuit with `?`
- **`default_role_for_new_user()`** --- the first user bootstraps as Admin
- **`auth_context()`** --- a helper that eliminates repetitive boilerplate
- **Ownership checks** --- "only the creator or an admin can delete" composes role guards with equality checks

On the UI side, you saw the profile page's three-state save button (idle/saving/saved), the `Result<Option<String>>` pattern for input validation with optional fields, Argon2 password hashing for security, and the admin page's conditional rendering with `bool::then()`.

The next chapter tackles video uploads --- uploading files to object storage and displaying them in the exercise library.
