use anyhow::{anyhow, Result};
use dashmap::DashMap;
use sha2::{Sha256, Digest};
use sqlx::PgPool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::config;
use crate::models::{APIKey, TierType, User};

// Cache entry with stale-while-revalidate support
#[derive(Clone)]
struct CachedApiKey {
    user: User,
    api_key: APIKey,
    cached_at: Instant,
    fresh_until: Instant,      // Soft TTL - serve without refresh
    valid_until: Instant,      // Hard TTL - max staleness
    refreshing: Arc<AtomicBool>, // Prevent duplicate background refreshes
}

// In-memory API key cache with stale-while-revalidate
pub struct ApiKeyCache {
    cache: Arc<DashMap<String, CachedApiKey>>,
    fresh_ttl: Duration,  // How long data is considered fresh (no refresh needed)
    stale_ttl: Duration,  // How long stale data can be served (with background refresh)
}

impl ApiKeyCache {
    pub fn new(fresh_ttl_seconds: u64, stale_ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            fresh_ttl: Duration::from_secs(fresh_ttl_seconds),
            stale_ttl: Duration::from_secs(stale_ttl_seconds),
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

        // Check cache with stale-while-revalidate logic
        if let Some(entry) = self.cache.get(&hash) {
            let now = Instant::now();

            // Case 1: Fresh - serve immediately
            if now < entry.fresh_until {
                info!("Cache HIT (fresh) for API key");
                return Ok((entry.user.clone(), entry.api_key.clone()));
            }

            // Case 2: Stale but valid - serve stale + refresh in background
            if now < entry.valid_until {
                info!("Cache HIT (stale) for API key - serving stale while revalidating");
                let result = Ok((entry.user.clone(), entry.api_key.clone()));

                // Trigger background refresh (only if not already refreshing)
                if !entry.refreshing.swap(true, Ordering::Relaxed) {
                    let cache_clone = self.cache.clone();
                    let pool_clone = pool.clone();
                    let hash_clone = hash.clone();
                    let fresh_ttl = self.fresh_ttl;
                    let stale_ttl = self.stale_ttl;

                    tokio::spawn(async move {
                        info!("Background refresh for API key: {}", &hash_clone[..10]);
                        if let Err(e) = Self::refresh_cache_entry(
                            &cache_clone,
                            &pool_clone,
                            &hash_clone,
                            fresh_ttl,
                            stale_ttl,
                        ).await {
                            warn!("Background refresh failed: {}", e);
                        }
                    });
                }

                return result;
            }

            // Case 3: Expired - must validate synchronously
            info!("Cache entry EXPIRED (hard TTL exceeded) - forcing validation");
            drop(entry);
            self.cache.remove(&hash);
        }

        info!("Cache MISS - querying database");

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

        // Cache the result with fresh and stale TTLs
        let now = Instant::now();
        self.cache.insert(
            hash,
            CachedApiKey {
                user: user.clone(),
                api_key: api_key_obj.clone(),
                cached_at: now,
                fresh_until: now + self.fresh_ttl,
                valid_until: now + self.stale_ttl,
                refreshing: Arc::new(AtomicBool::new(false)),
            },
        );

        Ok((user, api_key_obj))
    }

    // Background refresh helper
    async fn refresh_cache_entry(
        cache: &DashMap<String, CachedApiKey>,
        pool: &PgPool,
        hash: &str,
        fresh_ttl: Duration,
        stale_ttl: Duration,
    ) -> Result<()> {
        // Query database
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

        let row = result.ok_or_else(|| anyhow!("API key no longer valid"))?;

        // Reconstruct user and key
        let user = User {
            id: row.user_id,
            email: row.email,
            tier: row.tier,
            is_active: row.user_active,
            created_at: row.user_created,
            updated_at: row.user_updated,
        };

        let api_key = APIKey {
            id: row.key_id,
            user_id: row.key_user_id,
            key_hash: row.key_hash,
            key_prefix: row.key_prefix,
            name: row.name,
            is_active: row.key_active,
            created_at: row.key_created,
            last_used_at: row.last_used_at,
        };

        // Update cache
        let now = Instant::now();
        cache.insert(
            hash.to_string(),
            CachedApiKey {
                user,
                api_key,
                cached_at: now,
                fresh_until: now + fresh_ttl,
                valid_until: now + stale_ttl,
                refreshing: Arc::new(AtomicBool::new(false)),
            },
        );

        info!("Background refresh completed for API key: {}", &hash[..10]);
        Ok(())
    }

    // Periodically clean up expired entries (hard TTL exceeded)
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.cache.retain(|_, entry| now < entry.valid_until);
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

// Legacy function for backwards compatibility - now uses stale-while-revalidate cache
pub async fn validate_api_key(pool: &PgPool, api_key: &str) -> Result<(User, APIKey)> {
    // Create cache with:
    // - 5 minute fresh TTL (no refresh needed)
    // - 60 minute stale TTL (serve stale data, refresh in background)
    // This means: fresh for 5m, stale-but-valid for 55m, expired after 60m
    let cache = ApiKeyCache::new(300, 3600);
    cache.validate(pool, api_key).await
}
