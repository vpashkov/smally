use anyhow::{anyhow, Result};
use dashmap::DashMap;
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

use crate::config;
use crate::models::{APIKey, TierType, User};

// Cache entry with TTL
#[derive(Clone)]
struct CachedApiKey {
    user: User,
    api_key: APIKey,
    cached_at: Instant,
}

// In-memory API key cache
pub struct ApiKeyCache {
    cache: Arc<DashMap<String, CachedApiKey>>,
    ttl: Duration,
}

impl ApiKeyCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub async fn validate(&self, pool: &PgPool, api_key: &str) -> Result<(User, APIKey)> {
        let settings = config::get_settings();

        info!("Validating API key: {}...", &api_key[..10.min(api_key.len())]);

        if !api_key.starts_with(&settings.api_key_prefix) {
            info!("API key does not start with prefix: {}", settings.api_key_prefix);
            return Err(anyhow!("Invalid API key format"));
        }

        // Compute hash once
        let hash = hash_api_key(api_key);

        // Check cache first
        if let Some(entry) = self.cache.get(&hash) {
            if entry.cached_at.elapsed() < self.ttl {
                info!("Cache hit for API key");
                return Ok((entry.user.clone(), entry.api_key.clone()));
            } else {
                info!("Cache entry expired, removing");
                drop(entry); // Release the read lock
                self.cache.remove(&hash);
            }
        }

        info!("Cache miss, querying database");

        // Cache miss - query database with JOIN
        // This is a single query that gets both user and API key
        let result = sqlx::query!(
            r#"
            SELECT
                u.id as user_id, u.email, u.tier as "tier: TierType", u.is_active as user_active,
                u.created_at as user_created, u.updated_at as user_updated,
                k.id as key_id, k.user_id as key_user_id, k.key_hash, k.key_prefix, k.name,
                k.is_active as key_active, k.created_at as key_created, k.last_used_at
            FROM api_keys k
            INNER JOIN users u ON k.user_id = u.id
            WHERE k.key_hash = $1 AND k.is_active = true AND u.is_active = true
            "#,
            hash
        )
        .fetch_optional(pool)
        .await?;

        let row = result.ok_or_else(|| anyhow!("Invalid or expired API key"))?;

        info!("User fetched: {} ({:?})", row.email, row.tier);

        // Construct User and APIKey from query result
        let user = User {
            id: row.user_id,
            email: row.email,
            tier: row.tier,
            is_active: row.user_active,
            created_at: row.user_created,
            updated_at: row.user_updated,
        };

        let api_key_obj = APIKey {
            id: row.key_id,
            user_id: row.key_user_id,
            key_hash: row.key_hash,
            key_prefix: row.key_prefix,
            name: row.name,
            is_active: row.key_active,
            created_at: row.key_created,
            last_used_at: row.last_used_at,
        };

        // Update last used timestamp (fire and forget, don't wait)
        let now = chrono::Local::now().naive_local();
        let key_id = api_key_obj.id;
        let pool_clone = pool.clone();
        tokio::spawn(async move {
            let _ = sqlx::query(
                r#"
                UPDATE api_keys
                SET last_used_at = $1
                WHERE id = $2
                "#,
            )
            .bind(now)
            .bind(key_id)
            .execute(&pool_clone)
            .await;
        });

        // Cache the result
        self.cache.insert(
            hash,
            CachedApiKey {
                user: user.clone(),
                api_key: api_key_obj.clone(),
                cached_at: Instant::now(),
            },
        );

        Ok((user, api_key_obj))
    }

    // Periodically clean up expired entries
    pub fn cleanup_expired(&self) {
        self.cache.retain(|_, entry| entry.cached_at.elapsed() < self.ttl);
    }
}

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

// Legacy function for backwards compatibility - now uses cache
pub async fn validate_api_key(pool: &PgPool, api_key: &str) -> Result<(User, APIKey)> {
    // For now, create a temporary cache with 5-minute TTL
    // In production, this should be passed from main.rs
    let cache = ApiKeyCache::new(300);
    cache.validate(pool, api_key).await
}
