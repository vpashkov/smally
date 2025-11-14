use anyhow::Result;
use once_cell::sync::OnceCell;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::info;

use crate::config;

static DB_POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn init_db() -> Result<()> {
    // If already initialized, return early
    if DB_POOL.get().is_some() {
        return Ok(());
    }

    let settings = config::get_settings();

    // In test mode, use smaller pool with shorter timeouts to fail fast
    #[cfg(test)]
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect(&settings.database_url)
        .await?;

    // In production, use larger pool with default timeout
    #[cfg(not(test))]
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .connect(&settings.database_url)
        .await?;

    // Run migrations only in non-test mode
    #[cfg(not(test))]
    {
        info!("Running database migrations...");
        sqlx::migrate!("./migrations").run(&pool).await?;
        info!("Database migrations completed");
    }

    // Test the connection
    sqlx::query("SELECT 1").execute(&pool).await?;

    DB_POOL.set(pool).ok(); // Ignore error if already set

    info!("Database connection pool initialized");
    Ok(())
}

pub fn get_db() -> &'static PgPool {
    DB_POOL.get().expect("Database pool not initialized")
}
