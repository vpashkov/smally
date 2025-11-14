use anyhow::Result;
use once_cell::sync::OnceCell;
use sqlx::postgres::{PgPoolOptions, PgPool};
use tracing::info;

use crate::config;

static DB_POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn init_db() -> Result<()> {
    let settings = config::get_settings();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(&settings.database_url)
        .await?;

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;
    info!("Database migrations completed");

    // Test the connection
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await?;

    DB_POOL.set(pool).map_err(|_| anyhow::anyhow!("Database pool already initialized"))?;

    info!("Database connection pool initialized");
    Ok(())
}

pub fn get_db() -> &'static PgPool {
    DB_POOL.get().expect("Database pool not initialized")
}
