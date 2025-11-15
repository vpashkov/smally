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
use crate::models::TierType;

// Response update for batching
#[derive(Clone, Debug)]
struct ResponseUpdate {
    request_id: uuid::Uuid,
    tokens: i32,
    response_metadata: serde_json::Value,
    timestamp: NaiveDateTime,
}

// Usage event for batching
#[derive(Clone, Debug)]
struct UsageEvent {
    organization_id: uuid::Uuid,
    api_key_id: uuid::Uuid,
    product: String,
    event_type: String,
    tokens: i32,
    requests: i32,
    timestamp: NaiveDateTime,
}

// Buffer for batching usage updates
pub struct UsageBuffer {
    response_updates_buffer: Arc<Mutex<Vec<ResponseUpdate>>>,
    usage_events_buffer: Arc<Mutex<Vec<UsageEvent>>>,
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
            response_updates_buffer: Arc::new(Mutex::new(Vec::new())),
            usage_events_buffer: Arc::new(Mutex::new(Vec::new())),
            pool,
        }
    }

    /// Record incoming API request immediately (non-blocking insert to api_request_log)
    /// This creates an audit trail of ALL requests, even if they fail later
    pub fn record_request(
        &self,
        request_id: uuid::Uuid,
        organization_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
        product: String,
        endpoint: String,
        input_text: String,
        input_metadata: Option<serde_json::Value>,
    ) {
        let pool = self.pool;

        // Spawn non-blocking insert - don't wait for database
        tokio::spawn(async move {
            let result = sqlx::query(
                "INSERT INTO api_request_log
                 (request_id, organization_id, api_key_id, product, endpoint, input_text, input_metadata, request_timestamp, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), 'pending')",
            )
            .bind(request_id)
            .bind(organization_id)
            .bind(api_key_id)
            .bind(product)
            .bind(endpoint)
            .bind(input_text)
            .bind(input_metadata)
            .execute(pool)
            .await;

            if let Err(e) = result {
                tracing::error!("Failed to record request {}: {}", request_id, e);
            } else {
                tracing::debug!("Recorded request {} to api_request_log", request_id);
            }
        });
    }

    /// Record API response and usage (updates api_request_log, buffers usage_events)
    /// This is called when the response is ready with calculated tokens and metadata
    pub fn record_response(
        &self,
        request_id: uuid::Uuid,
        organization_id: uuid::Uuid,
        api_key_id: uuid::Uuid,
        product: &str,
        tokens: i32,
        response_metadata: serde_json::Value,
    ) {
        let now = chrono::Local::now().naive_local();

        // Buffer the response update for api_request_log
        let response_update = ResponseUpdate {
            request_id,
            tokens,
            response_metadata,
            timestamp: now,
        };
        self.response_updates_buffer.lock().push(response_update);

        // Buffer the usage event for billing
        let usage = UsageEvent {
            organization_id,
            api_key_id,
            product: product.to_string(),
            event_type: "inference".to_string(),
            tokens,
            requests: 1,
            timestamp: now,
        };
        self.usage_events_buffer.lock().push(usage);
    }

    // Flush buffered records to database (batch insert)
    pub async fn flush(&self) -> Result<(usize, usize)> {
        // 1. Flush response updates to api_request_log
        let response_updates = {
            let mut buffer = self.response_updates_buffer.lock();
            std::mem::take(&mut *buffer)
        };

        let response_count = if !response_updates.is_empty() {
            let count = response_updates.len();
            info!("Flushing {} response updates to api_request_log", count);

            // Batch update using individual queries (PostgreSQL doesn't support batch UPDATE well)
            for update in response_updates {
                sqlx::query(
                    "UPDATE api_request_log
                     SET tokens = $1,
                         response_metadata = $2,
                         response_timestamp = $3,
                         status = 'success',
                         updated_at = NOW()
                     WHERE request_id = $4",
                )
                .bind(update.tokens)
                .bind(update.response_metadata)
                .bind(update.timestamp)
                .bind(update.request_id)
                .execute(self.pool)
                .await?;
            }

            info!("Successfully flushed {} response updates", count);
            count
        } else {
            0
        };

        // 2. Flush usage events
        let usage_events = {
            let mut buffer = self.usage_events_buffer.lock();
            std::mem::take(&mut *buffer)
        };

        let usage_count = if !usage_events.is_empty() {
            let count = usage_events.len();
            info!("Flushing {} usage events", count);

            // Batch insert using QueryBuilder
            let mut query_builder = sqlx::QueryBuilder::new(
                "INSERT INTO usage_events (organization_id, api_key_id, product, event_type, tokens, requests, timestamp) ",
            );

            query_builder.push_values(usage_events, |mut b, event| {
                b.push_bind(event.organization_id)
                    .push_bind(event.api_key_id)
                    .push_bind(event.product)
                    .push_bind(event.event_type)
                    .push_bind(event.tokens)
                    .push_bind(event.requests)
                    .push_bind(event.timestamp);
            });

            query_builder.build().execute(self.pool).await?;

            info!("Successfully flushed {} usage events", count);
            count
        } else {
            0
        };

        Ok((response_count, usage_count))
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
    // If already initialized, return early
    if USAGE_BUFFER.get().is_some() {
        return Ok(());
    }

    let buffer = Arc::new(UsageBuffer::new(pool));
    buffer.clone().start_flush_task();
    USAGE_BUFFER.set(buffer).ok(); // Ignore error if already set
    info!("Usage buffer initialized with 5-second flush interval");
    Ok(())
}

// Get global usage buffer
pub fn get_usage_buffer() -> &'static Arc<UsageBuffer> {
    USAGE_BUFFER.get().expect("Usage buffer not initialized")
}

// Initialize global Redis connection for rate limiting
pub async fn init_redis() -> Result<()> {
    // If already initialized, return early
    if REDIS_CONNECTION.get().is_some() {
        return Ok(());
    }

    let settings = config::get_settings();
    let redis_client = redis::Client::open(settings.redis_url.as_str())?;
    let conn = ConnectionManager::new(redis_client).await?;
    REDIS_CONNECTION.set(conn).ok(); // Ignore error if already set
    info!("Redis connection for billing initialized");
    Ok(())
}

// Get global Redis connection
fn get_redis_connection() -> &'static ConnectionManager {
    REDIS_CONNECTION
        .get()
        .expect("Redis connection not initialized")
}

// ====== Token-based functions ======

/// Check rate limit using token claims (no DB required)
pub async fn check_rate_limit_from_claims(
    claims: &TokenClaims,
) -> Result<(bool, HashMap<String, String>)> {
    // Skip rate limiting for paid tiers (they use pay-as-you-go)
    let tier = claims.tier()?;
    match tier {
        TierType::Pro | TierType::Scale => {
            info!("Skipping rate limit check for paid tier: {:?}", tier);
            Ok((true, HashMap::new()))
        }
        TierType::Free => {
            // Free tier: check Redis quota
            info!("Checking rate limit for free tier org {}", claims.org_id());
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
    let month_key = format!("ratelimit:{}:{}", claims.org_id(), now.format("%Y-%m"));

    // Get current count from Redis
    let count: i64 = conn.get(&month_key).await.unwrap_or(0);

    info!(
        "Redis rate limit check: org {} count {}",
        claims.org_id(),
        count
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
    let limit = claims.monthly_quota() as i64;

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

/// Increment Redis counter for free tier rate limiting (async, non-blocking)
pub fn increment_free_tier_counter(org_id: uuid::Uuid) {
    tokio::spawn(async move {
        if let Err(e) = increment_redis_counter_simple(org_id).await {
            info!("Failed to increment Redis counter for free tier: {}", e);
        }
    });
}

/// Increment Redis counter (simplified - no API key ID)
async fn increment_redis_counter_simple(user_id: uuid::Uuid) -> Result<()> {
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
fn hash_key_id(key_id: uuid::Uuid) -> i64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    key_id.hash(&mut hasher);
    hasher.finish() as i64
}
