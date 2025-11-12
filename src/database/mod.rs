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

pub async fn create_tables() -> Result<()> {
    let pool = get_db();

    let queries = vec![
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            email VARCHAR(255) UNIQUE NOT NULL,
            tier VARCHAR(50) NOT NULL DEFAULT 'free',
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)",
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            key_hash VARCHAR(255) UNIQUE NOT NULL,
            key_prefix VARCHAR(50) NOT NULL,
            name VARCHAR(255),
            is_active BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            last_used_at TIMESTAMP
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash)",
        "CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id)",
        r#"
        CREATE TABLE IF NOT EXISTS usage (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            api_key_id INTEGER NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
            month VARCHAR(10) NOT NULL,
            embeddings_count BIGINT NOT NULL DEFAULT 0,
            cache_hits BIGINT NOT NULL DEFAULT 0,
            cache_misses BIGINT NOT NULL DEFAULT 0,
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP NOT NULL DEFAULT NOW()
        )
        "#,
        "CREATE INDEX IF NOT EXISTS idx_usage_user_month ON usage(user_id, month)",
        "CREATE INDEX IF NOT EXISTS idx_usage_api_key_month ON usage(api_key_id, month)",
    ];

    for query in queries {
        sqlx::query(query).execute(pool).await?;
    }

    info!("Database tables created successfully");
    Ok(())
}
