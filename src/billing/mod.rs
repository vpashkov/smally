use anyhow::{anyhow, Result};
use chrono::{Datelike, NaiveDateTime, Utc};
use parking_lot::Mutex;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::info;

use crate::auth::TokenClaims;
use crate::config;
use crate::models::{APIKey, TierType, User};

// Usage record for batching
#[derive(Clone, Debug)]
struct UsageRecord {
    user_id: i64,
    api_key_id: i64,
    embeddings_count: i32,
    timestamp: NaiveDateTime,
}

// Buffer for batching usage updates
pub struct UsageBuffer {
    buffer: Arc<Mutex<Vec<UsageRecord>>>,
    pool: &'static PgPool,
}

// Global usage buffer instance
static USAGE_BUFFER: once_cell::sync::OnceCell<Arc<UsageBuffer>> = once_cell::sync::OnceCell::new();

// Global Redis connection for rate limiting
static REDIS_CONNECTION: once_cell::sync::OnceCell<ConnectionManager> =
    once_cell::sync::OnceCell::new();

impl UsageBuffer {
    pub fn new(pool: &'static PgPool) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            pool,
        }
    }

    // Record a usage event (non-blocking)
    pub fn record(&self, user_id: i64, api_key_id: i64) {
        let now = chrono::Local::now().naive_local();
        let record = UsageRecord {
            user_id,
            api_key_id,
            embeddings_count: 1,
            timestamp: now,
        };

        let mut buffer = self.buffer.lock();
        buffer.push(record);
    }

    // Flush buffered records to database (batch insert)
    pub async fn flush(&self) -> Result<usize> {
        // Swap buffer to minimize lock time
        let records = {
            let mut buffer = self.buffer.lock();
            std::mem::take(&mut *buffer)
        };

        if records.is_empty() {
            return Ok(0);
        }

        let count = records.len();
        info!("Flushing {} usage records to database", count);

        // Batch insert using QueryBuilder
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO usage (user_id, api_key_id, embeddings_count, timestamp) ",
        );

        query_builder.push_values(records, |mut b, record| {
            b.push_bind(record.user_id)
                .push_bind(record.api_key_id)
                .push_bind(record.embeddings_count)
                .push_bind(record.timestamp);
        });

        query_builder.build().execute(self.pool).await?;

        info!("Successfully flushed {} usage records", count);
        Ok(count)
    }

    // Start background flush task (every 5 seconds)
    pub fn start_flush_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                if let Err(e) = self.flush().await {
                    tracing::error!("Failed to flush usage buffer: {}", e);
                }
            }
        });
    }
}

// Initialize global usage buffer
pub fn init_usage_buffer(pool: &'static PgPool) -> Result<()> {
    let buffer = Arc::new(UsageBuffer::new(pool));
    buffer.clone().start_flush_task();
    USAGE_BUFFER
        .set(buffer)
        .map_err(|_| anyhow::anyhow!("Usage buffer already initialized"))?;
    info!("Usage buffer initialized with 5-second flush interval");
    Ok(())
}

// Get global usage buffer
pub fn get_usage_buffer() -> &'static Arc<UsageBuffer> {
    USAGE_BUFFER.get().expect("Usage buffer not initialized")
}

// Initialize global Redis connection for rate limiting
pub async fn init_redis() -> Result<()> {
    let settings = config::get_settings();
    let redis_client = redis::Client::open(settings.redis_url.as_str())?;
    let conn = ConnectionManager::new(redis_client).await?;
    REDIS_CONNECTION
        .set(conn)
        .map_err(|_| anyhow::anyhow!("Redis connection already initialized"))?;
    info!("Redis connection for billing initialized");
    Ok(())
}

// Get global Redis connection
fn get_redis_connection() -> &'static ConnectionManager {
    REDIS_CONNECTION
        .get()
        .expect("Redis connection not initialized")
}

// Check rate limit for free tier users (paid tiers are unlimited)
pub async fn check_rate_limit(
    pool: &PgPool,
    user: &User,
    api_key: &APIKey,
) -> Result<(bool, HashMap<String, String>)> {
    // Skip rate limiting for paid tiers (Pro, Scale)
    // They use pure pay-as-you-go billing with no monthly quotas
    match user.tier {
        TierType::Pro | TierType::Scale => {
            info!("Skipping rate limit check for paid tier: {:?}", user.tier);
            Ok((true, HashMap::new()))
        }
        TierType::Free => {
            // Free tier: enforce monthly quota to prevent abuse
            info!("Checking rate limit for free tier user {}", user.id);
            // Try Redis first, fall back to DB if needed
            match check_rate_limit_redis(user, api_key).await {
                Ok(result) => Ok(result),
                Err(e) => {
                    info!("Redis rate limit check failed, falling back to DB: {}", e);
                    check_rate_limit_db(pool, user, api_key).await
                }
            }
        }
    }
}

// Redis-based rate limiting (fast path)
async fn check_rate_limit_redis(
    user: &User,
    api_key: &APIKey,
) -> Result<(bool, HashMap<String, String>)> {
    let settings = config::get_settings();

    // Use global Redis connection (reused across requests)
    let mut conn = get_redis_connection().clone();

    // Get current month for key
    let now = Utc::now();
    let month_key = format!(
        "ratelimit:{}:{}:{}",
        user.id,
        api_key.id,
        now.format("%Y-%m")
    );

    // Get current count from Redis
    let count: i64 = conn.get(&month_key).await.unwrap_or(0);

    info!(
        "Redis rate limit check: user {} api_key {} count {}",
        user.id, api_key.id, count
    );

    // Calculate month end for reset_at
    let year = now.year();
    let month = now.month();
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    let month_end = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .ok_or_else(|| anyhow!("Invalid date"))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Invalid time"))?;

    // Get tier limit
    let limit = get_tier_limit(&user.tier, settings);

    // Check if exceeded
    let is_allowed = count < limit as i64;
    let remaining = (limit as i64 - count).max(0);

    let mut rate_limit_info = HashMap::new();
    rate_limit_info.insert("limit".to_string(), limit.to_string());
    rate_limit_info.insert("remaining".to_string(), remaining.to_string());
    rate_limit_info.insert(
        "reset_at".to_string(),
        month_end.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    );
    rate_limit_info.insert("current_usage".to_string(), count.to_string());

    Ok((is_allowed, rate_limit_info))
}

// Database-based rate limiting (fallback)
async fn check_rate_limit_db(
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

    info!(
        "Checking rate limit (DB fallback) for user {} api_key {} from {} to {}",
        user.id, api_key.id, month_start, month_end
    );

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
    info!("Current embeddings count (DB): {}", embeddings_count);

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

// Increment usage - records usage for billing and analytics
// Only increments Redis counter for free tier (they have quotas)
pub async fn increment_usage(user: &User, api_key: &APIKey) -> Result<()> {
    let user_id = user.id;
    let api_key_id = api_key.id;
    let tier = user.tier.clone();

    // Only increment Redis counter for free tier (they have quotas)
    // Paid tiers use pure pay-as-you-go billing, no quota tracking needed
    match tier {
        TierType::Free => {
            tokio::spawn(async move {
                if let Err(e) = increment_redis_counter(user_id, api_key_id).await {
                    info!("Failed to increment Redis counter for free tier: {}", e);
                }
            });
        }
        TierType::Pro | TierType::Scale => {
            // Skip Redis counter for paid tiers
            info!("Skipping Redis counter for paid tier: {:?}", tier);
        }
    }

    // Buffer the usage record for ALL tiers (needed for billing/analytics)
    // The background task will flush it to the database
    get_usage_buffer().record(user.id, api_key.id);

    Ok(())
}

// Increment Redis counter for rate limiting
async fn increment_redis_counter(user_id: i64, api_key_id: i64) -> Result<()> {
    // Use global Redis connection (reused across requests)
    let mut conn = get_redis_connection().clone();

    // Get current month for key
    let now = Utc::now();
    let month_key = format!(
        "ratelimit:{}:{}:{}",
        user_id,
        api_key_id,
        now.format("%Y-%m")
    );

    // Atomically increment counter and set expiration
    let _: () = redis::pipe()
        .atomic()
        .incr(&month_key, 1)
        .expire(&month_key, 60 * 60 * 24 * 32) // 32 days
        .query_async(&mut conn)
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

// ====== New PASETO-based functions ======

/// Check rate limit using token claims (no DB required)
pub async fn check_rate_limit_from_claims(
    claims: &TokenClaims,
) -> Result<(bool, HashMap<String, String>)> {
    // Skip rate limiting for paid tiers (they use pay-as-you-go)
    match claims.tier {
        TierType::Pro | TierType::Scale => {
            info!("Skipping rate limit check for paid tier: {:?}", claims.tier);
            Ok((true, HashMap::new()))
        }
        TierType::Free => {
            // Free tier: check Redis quota
            info!("Checking rate limit for free tier user {}", claims.user_id);
            check_rate_limit_redis_from_claims(claims).await
        }
    }
}

/// Redis-based rate limiting using token claims
async fn check_rate_limit_redis_from_claims(
    claims: &TokenClaims,
) -> Result<(bool, HashMap<String, String>)> {
    // Use global Redis connection
    let mut conn = get_redis_connection().clone();

    // Get current month for key
    let now = Utc::now();
    let month_key = format!("ratelimit:{}:{}", claims.user_id, now.format("%Y-%m"));

    // Get current count from Redis
    let count: i64 = conn.get(&month_key).await.unwrap_or(0);

    info!(
        "Redis rate limit check: user {} count {}",
        claims.user_id, count
    );

    // Calculate month end for reset_at
    let year = now.year();
    let month = now.month();
    let next_month = if month == 12 { 1 } else { month + 1 };
    let next_year = if month == 12 { year + 1 } else { year };
    let month_end = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .ok_or_else(|| anyhow!("Invalid date"))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Invalid time"))?;

    // Get limit from token (embedded in token, no config needed!)
    let limit = claims.monthly_quota as i64;

    // Check if exceeded
    let is_allowed = count < limit;
    let remaining = (limit - count).max(0);

    let mut rate_limit_info = HashMap::new();
    rate_limit_info.insert("limit".to_string(), limit.to_string());
    rate_limit_info.insert("remaining".to_string(), remaining.to_string());
    rate_limit_info.insert(
        "reset_at".to_string(),
        month_end.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    );
    rate_limit_info.insert("current_usage".to_string(), count.to_string());

    Ok((is_allowed, rate_limit_info))
}

/// Increment usage using token claims (no DB needed for lookup)
pub async fn increment_usage_from_claims(claims: &TokenClaims) -> Result<()> {
    let user_id = claims.user_id;
    // For API key ID, we'll hash the key_id from the token
    let api_key_id = hash_key_id(&claims.key_id);
    let tier = claims.tier.clone();

    // Only increment Redis counter for free tier (they have quotas)
    match tier {
        TierType::Free => {
            tokio::spawn(async move {
                if let Err(e) = increment_redis_counter_simple(user_id).await {
                    info!("Failed to increment Redis counter for free tier: {}", e);
                }
            });
        }
        TierType::Pro | TierType::Scale => {
            info!("Skipping Redis counter for paid tier: {:?}", tier);
        }
    }

    // Buffer the usage record (for billing/analytics)
    get_usage_buffer().record(user_id, api_key_id);

    Ok(())
}

/// Increment Redis counter (simplified - no API key ID)
async fn increment_redis_counter_simple(user_id: i64) -> Result<()> {
    let mut conn = get_redis_connection().clone();

    // Get current month for key
    let now = Utc::now();
    let month_key = format!("ratelimit:{}:{}", user_id, now.format("%Y-%m"));

    // Atomically increment counter and set expiration
    let _: () = redis::pipe()
        .atomic()
        .incr(&month_key, 1)
        .expire(&month_key, 60 * 60 * 24 * 32) // 32 days
        .query_async(&mut conn)
        .await?;

    Ok(())
}

/// Hash key_id to get a deterministic API key ID
fn hash_key_id(key_id: &str) -> i64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    key_id.hash(&mut hasher);
    hasher.finish() as i64
}
