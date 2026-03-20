// Chapter 12 DSA Exercise: Bit Masking for Permission Systems
//
// Production permission systems use bitmasks for flexible, composable permissions.
// Each permission is a bit; roles are combinations of bits.
// Check permission with bitwise AND: (user_perms & required) == required

use std::fmt;

// ----------------------------------------------------------------
// Part 1: GrindIt's simple role hierarchy
// ----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum UserRole {
    Athlete,
    Coach,
    Admin,
}

impl UserRole {
    fn rank(&self) -> u8 {
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

fn require_role(user_role: UserRole, min_role: UserRole) -> Result<(), String> {
    if user_role.rank() >= min_role.rank() {
        Ok(())
    } else {
        Err(format!(
            "Insufficient permissions: {} (rank {}) < {} (rank {})",
            user_role,
            user_role.rank(),
            min_role,
            min_role.rank()
        ))
    }
}

// ----------------------------------------------------------------
// Part 2: Bitmask permission system (production pattern)
// ----------------------------------------------------------------

/// Permission flags — each is a single bit
const PERM_READ: u32 = 0b0000_0001; // 1
const PERM_WRITE: u32 = 0b0000_0010; // 2
const PERM_DELETE: u32 = 0b0000_0100; // 4
const PERM_ADMIN: u32 = 0b0000_1000; // 8
const PERM_MANAGE_USERS: u32 = 0b0001_0000; // 16
const PERM_MANAGE_WODS: u32 = 0b0010_0000; // 32
const PERM_VIEW_ANALYTICS: u32 = 0b0100_0000; // 64

/// Role presets — combinations of permission bits
const ROLE_ATHLETE: u32 = PERM_READ | PERM_WRITE; // 0b0000_0011 = 3
const ROLE_COACH: u32 = PERM_READ | PERM_WRITE | PERM_DELETE | PERM_MANAGE_WODS | PERM_VIEW_ANALYTICS; // 103
const ROLE_ADMIN: u32 = PERM_READ | PERM_WRITE | PERM_DELETE | PERM_ADMIN | PERM_MANAGE_USERS | PERM_MANAGE_WODS | PERM_VIEW_ANALYTICS; // 127

/// Check if a user has a specific permission
fn has_permission(user_perms: u32, required: u32) -> bool {
    (user_perms & required) == required
}

/// Check if a user has ALL of multiple required permissions
fn has_all_permissions(user_perms: u32, required: &[u32]) -> bool {
    let combined: u32 = required.iter().fold(0, |acc, &p| acc | p);
    has_permission(user_perms, combined)
}

/// Check if a user has ANY of the given permissions
fn has_any_permission(user_perms: u32, options: &[u32]) -> bool {
    options.iter().any(|&p| has_permission(user_perms, p))
}

/// Grant a permission (set a bit)
fn grant_permission(user_perms: u32, perm: u32) -> u32 {
    user_perms | perm
}

/// Revoke a permission (clear a bit)
fn revoke_permission(user_perms: u32, perm: u32) -> u32 {
    user_perms & !perm
}

/// Toggle a permission (flip a bit)
fn toggle_permission(user_perms: u32, perm: u32) -> u32 {
    user_perms ^ perm
}

/// Display permission bits in human-readable form
fn permission_string(perms: u32) -> String {
    let mut flags = Vec::new();
    if perms & PERM_READ != 0 {
        flags.push("READ");
    }
    if perms & PERM_WRITE != 0 {
        flags.push("WRITE");
    }
    if perms & PERM_DELETE != 0 {
        flags.push("DELETE");
    }
    if perms & PERM_ADMIN != 0 {
        flags.push("ADMIN");
    }
    if perms & PERM_MANAGE_USERS != 0 {
        flags.push("MANAGE_USERS");
    }
    if perms & PERM_MANAGE_WODS != 0 {
        flags.push("MANAGE_WODS");
    }
    if perms & PERM_VIEW_ANALYTICS != 0 {
        flags.push("VIEW_ANALYTICS");
    }
    if flags.is_empty() {
        "NONE".to_string()
    } else {
        flags.join(" | ")
    }
}

// ----------------------------------------------------------------
// Part 3: Interview Problem — Single Number (XOR trick)
// Every element appears twice except one. Find it using XOR.
// ----------------------------------------------------------------

fn single_number(nums: &[i32]) -> i32 {
    nums.iter().fold(0, |acc, &x| acc ^ x)
}

// ----------------------------------------------------------------
// Part 4: Unix-style file permissions (classic bitmask example)
// ----------------------------------------------------------------

fn unix_permission_string(mode: u32) -> String {
    let mut s = String::new();
    // Owner
    s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o100 != 0 { 'x' } else { '-' });
    // Group
    s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o010 != 0 { 'x' } else { '-' });
    // Other
    s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    s.push(if mode & 0o001 != 0 { 'x' } else { '-' });
    s
}

fn main() {
    println!("=== Bit Masking: Permission Systems ===\n");

    // Part 1: Simple role hierarchy
    println!("--- Part 1: GrindIt Role Hierarchy ---");
    let roles = [UserRole::Athlete, UserRole::Coach, UserRole::Admin];
    let actions = [
        ("View exercises", UserRole::Athlete),
        ("Create WODs", UserRole::Coach),
        ("Manage users", UserRole::Admin),
    ];

    for user_role in &roles {
        println!("  {} (rank {}):", user_role, user_role.rank());
        for (action, min_role) in &actions {
            let result = require_role(*user_role, *min_role);
            let allowed = result.is_ok();
            println!(
                "    {} {}: {}",
                if allowed { "[OK]" } else { "[NO]" },
                action,
                if allowed {
                    "allowed".to_string()
                } else {
                    result.unwrap_err()
                }
            );
        }
    }

    // Part 2: Bitmask permissions
    println!("\n--- Part 2: Bitmask Permissions ---");
    println!("  Bit layout:");
    println!("    READ           = {:08b} ({})", PERM_READ, PERM_READ);
    println!("    WRITE          = {:08b} ({})", PERM_WRITE, PERM_WRITE);
    println!("    DELETE         = {:08b} ({})", PERM_DELETE, PERM_DELETE);
    println!("    ADMIN          = {:08b} ({})", PERM_ADMIN, PERM_ADMIN);
    println!("    MANAGE_USERS   = {:08b} ({})", PERM_MANAGE_USERS, PERM_MANAGE_USERS);
    println!("    MANAGE_WODS    = {:08b} ({})", PERM_MANAGE_WODS, PERM_MANAGE_WODS);
    println!("    VIEW_ANALYTICS = {:08b} ({})", PERM_VIEW_ANALYTICS, PERM_VIEW_ANALYTICS);

    println!("\n  Role presets:");
    println!("    ATHLETE = {:08b} => {}", ROLE_ATHLETE, permission_string(ROLE_ATHLETE));
    println!("    COACH   = {:08b} => {}", ROLE_COACH, permission_string(ROLE_COACH));
    println!("    ADMIN   = {:08b} => {}", ROLE_ADMIN, permission_string(ROLE_ADMIN));

    println!("\n  Permission checks:");
    let test_cases = vec![
        ("Athlete can read?", ROLE_ATHLETE, PERM_READ),
        ("Athlete can delete?", ROLE_ATHLETE, PERM_DELETE),
        ("Coach can manage WODs?", ROLE_COACH, PERM_MANAGE_WODS),
        ("Coach can manage users?", ROLE_COACH, PERM_MANAGE_USERS),
        ("Admin can manage users?", ROLE_ADMIN, PERM_MANAGE_USERS),
    ];
    for (label, perms, required) in &test_cases {
        println!(
            "    {} => {}",
            label,
            has_permission(*perms, *required)
        );
    }

    // Custom permission: coach + analytics but no delete
    println!("\n  Custom role (coach without delete + analytics):");
    let custom = ROLE_COACH & !PERM_DELETE;
    println!("    Permissions: {:08b} => {}", custom, permission_string(custom));
    println!(
        "    Can delete? {}",
        has_permission(custom, PERM_DELETE)
    );
    println!(
        "    Can view analytics? {}",
        has_permission(custom, PERM_VIEW_ANALYTICS)
    );

    // Grant and revoke
    println!("\n  Grant/revoke operations:");
    let mut perms = ROLE_ATHLETE;
    println!("    Start:   {:08b} => {}", perms, permission_string(perms));
    perms = grant_permission(perms, PERM_MANAGE_WODS);
    println!("    +WODS:   {:08b} => {}", perms, permission_string(perms));
    perms = grant_permission(perms, PERM_VIEW_ANALYTICS);
    println!("    +ANALYT: {:08b} => {}", perms, permission_string(perms));
    perms = revoke_permission(perms, PERM_WRITE);
    println!("    -WRITE:  {:08b} => {}", perms, permission_string(perms));
    perms = toggle_permission(perms, PERM_DELETE);
    println!("    ^DELETE: {:08b} => {}", perms, permission_string(perms));

    // Part 3: Single Number (XOR)
    println!("\n--- Part 3: Single Number (XOR trick) ---");
    let workout_ids = vec![5, 3, 7, 3, 5];
    println!(
        "  IDs {:?} => unique: {}",
        workout_ids,
        single_number(&workout_ids)
    );
    println!("  XOR cancels pairs: a ^ a = 0, 0 ^ b = b");

    // Part 4: Unix permissions
    println!("\n--- Part 4: Unix File Permissions ---");
    let modes = vec![
        (0o755, "rwxr-xr-x  (typical executable)"),
        (0o644, "rw-r--r--  (typical file)"),
        (0o600, "rw-------  (secrets file)"),
        (0o777, "rwxrwxrwx  (open to all)"),
    ];
    for (mode, description) in &modes {
        println!(
            "  {:03o} = {} {}",
            mode,
            unix_permission_string(*mode),
            description
        );
    }

    println!("\n=== Key Insights ===");
    println!("1. Bitmasks are more flexible than linear hierarchies (arbitrary combos)");
    println!("2. Check: (perms & required) == required");
    println!("3. Grant: perms | new_perm");
    println!("4. Revoke: perms & !old_perm");
    println!("5. Toggle: perms ^ perm");
    println!("6. Real-world: Unix permissions, AWS IAM, Discord roles, feature flags");
}
