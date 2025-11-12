use anyhow::{anyhow, Result};
use chrono::{Datelike, Utc};
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use std::collections::HashMap;
use tracing::info;

use crate::config;
use crate::models::{APIKey, TierType, Usage, User};

pub fn generate_api_key() -> Result<String> {
    let settings = config::get_settings();
    let random_part = hex::encode(rand::random::<[u8; 32]>());
    Ok(format!("{}{}", settings.api_key_prefix, random_part))
}

pub fn hash_api_key(api_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn verify_api_key(plain_key: &str, hashed_key: &str) -> bool {
    hash_api_key(plain_key) == hashed_key
}

pub async fn validate_api_key(pool: &PgPool, api_key: &str) -> Result<(User, APIKey)> {
    let settings = config::get_settings();

    info!("Validating API key: {}...", &api_key[..10.min(api_key.len())]);

    if !api_key.starts_with(&settings.api_key_prefix) {
        info!("API key does not start with prefix: {}", settings.api_key_prefix);
        return Err(anyhow!("Invalid API key format"));
    }

    // Query all active API keys
    let keys = sqlx::query_as::<_, APIKey>(
        r#"
        SELECT id, user_id, key_hash, key_prefix, name, is_active, created_at, last_used_at
        FROM api_keys
        WHERE is_active = true
        "#,
    )
    .fetch_all(pool)
    .await?;

    info!("Found {} active API keys in database", keys.len());

    // Find matching key
    let mut matched_key: Option<APIKey> = None;
    for key in keys {
        let computed_hash = hash_api_key(api_key);
        info!("Comparing hash - Computed: {}, Stored: {}", computed_hash, key.key_hash);
        if verify_api_key(api_key, &key.key_hash) {
            info!("Found matching key!");
            matched_key = Some(key);
            break;
        }
    }

    let matched_key = matched_key.ok_or_else(|| anyhow!("Invalid or expired API key"))?;

    info!("Fetching user with ID: {}", matched_key.user_id);

    // Get user
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, email, tier, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(matched_key.user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        info!("Error fetching user: {}", e);
        anyhow!("Error fetching user: {}", e)
    })?;

    info!("User fetched: {} ({:?})", user.email, user.tier);

    if !user.is_active {
        return Err(anyhow!("User account is inactive"));
    }

    // Update last used timestamp
    let now = chrono::Local::now().naive_local();
    sqlx::query(
        r#"
        UPDATE api_keys
        SET last_used_at = $1
        WHERE id = $2
        "#,
    )
    .bind(now)
    .bind(matched_key.id)
    .execute(pool)
    .await
    .ok();

    Ok((user, matched_key))
}

pub async fn check_rate_limit(
    pool: &PgPool,
    user: &User,
    api_key: &APIKey,
) -> Result<(bool, HashMap<String, String>)> {
    let settings = config::get_settings();

    // Get current month start and end timestamps
    let now = Utc::now();
    let year = now.year();
    let month = now.month();

    // First day of current month at 00:00:00 UTC
    let month_start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| anyhow!("Invalid date"))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Invalid time"))?;

    // First day of next month at 00:00:00 UTC
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    let month_end = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .ok_or_else(|| anyhow!("Invalid date"))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Invalid time"))?;

    info!("Checking rate limit for user {} api_key {} from {} to {}",
          user.id, api_key.id, month_start, month_end);

    // Sum embeddings count for current month
    let total_count: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(embeddings_count), 0)
        FROM usage
        WHERE user_id = $1 AND api_key_id = $2 AND timestamp >= $3 AND timestamp < $4
        "#,
    )
    .bind(user.id)
    .bind(api_key.id)
    .bind(month_start)
    .bind(month_end)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        info!("Error fetching usage: {}", e);
        anyhow!("Error fetching usage: {}", e)
    })?;

    let embeddings_count = total_count.unwrap_or(0);
    info!("Current embeddings count: {}", embeddings_count);

    // Get tier limit
    let limit = get_tier_limit(&user.tier, settings);

    // Check if exceeded
    let is_allowed = embeddings_count < limit as i64;
    let remaining = (limit as i64 - embeddings_count).max(0);

    let mut rate_limit_info = HashMap::new();
    rate_limit_info.insert("limit".to_string(), limit.to_string());
    rate_limit_info.insert("remaining".to_string(), remaining.to_string());
    rate_limit_info.insert(
        "reset_at".to_string(),
        month_end.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    );
    rate_limit_info.insert("current_usage".to_string(), embeddings_count.to_string());

    Ok((is_allowed, rate_limit_info))
}

pub async fn increment_usage(
    pool: &PgPool,
    user: &User,
    api_key: &APIKey,
    _cached: bool,
) -> Result<()> {
    let now = chrono::Local::now().naive_local();

    // Insert a new usage record for each embedding request
    sqlx::query(
        r#"
        INSERT INTO usage (user_id, api_key_id, embeddings_count, timestamp)
        VALUES ($1, $2, 1, $3)
        "#,
    )
    .bind(user.id)
    .bind(api_key.id)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

fn get_tier_limit(tier: &TierType, settings: &config::Settings) -> i32 {
    match tier {
        TierType::Free => settings.free_tier_limit,
        TierType::Pro => settings.pro_tier_limit,
        TierType::Scale => settings.scale_tier_limit,
    }
}
