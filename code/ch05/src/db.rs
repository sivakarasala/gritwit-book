// Chapter 5: Database Persistence
// Spotlight: Async/Await & SQLx
//
// Global pool, async queries, compile-time verified SQL.

use sqlx::PgPool;
use std::sync::OnceLock;

static POOL: OnceLock<PgPool> = OnceLock::new();

pub fn init_pool(pool: PgPool) {
    POOL.set(pool).expect("Pool already initialized");
}

pub fn db() -> &'static PgPool {
    POOL.get().expect("Pool not initialized — call init_pool() first")
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
pub struct Exercise {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub scoring_type: String,
    pub created_by: Option<i32>,
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[cfg(feature = "ssr")]
pub async fn list_exercises_db(pool: &PgPool) -> Result<Vec<Exercise>, sqlx::Error> {
    sqlx::query_as!(
        Exercise,
        r#"SELECT id, name, category, scoring_type, created_by, deleted_at
           FROM exercises
           WHERE deleted_at IS NULL
           ORDER BY name"#
    )
    .fetch_all(pool)
    .await
}

#[cfg(feature = "ssr")]
pub async fn create_exercise_db(
    pool: &PgPool,
    name: &str,
    category: &str,
    scoring_type: &str,
    created_by: i32,
) -> Result<Exercise, sqlx::Error> {
    sqlx::query_as!(
        Exercise,
        r#"INSERT INTO exercises (name, category, scoring_type, created_by)
           VALUES ($1, $2, $3, $4)
           RETURNING id, name, category, scoring_type, created_by, deleted_at"#,
        name,
        category,
        scoring_type,
        created_by,
    )
    .fetch_one(pool)
    .await
}
