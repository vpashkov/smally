use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use rusty_paseto::core::{Key, PasetoAsymmetricPublicKey, Public, V4};
use rusty_paseto::prelude::PasetoParser;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::config;
use crate::models::TierType;

/// PASETO token claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    /// Subject - unique key identifier
    pub sub: String,
    /// User ID
    pub user_id: i64,
    /// API key ID (for revocation tracking)
    pub key_id: String,
    /// User tier
    pub tier: TierType,
    /// Expiration time
    pub exp: DateTime<Utc>,
    /// Issued at time
    pub iat: DateTime<Utc>,
    /// Token limits (embedded in token)
    pub limits: TokenLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenLimits {
    pub max_tokens: usize,
    pub monthly_quota: i32,
}

/// Revocation status cache entry
#[derive(Clone)]
struct RevocationStatus {
    is_revoked: bool,
    cached_at: Instant,
    fresh_until: Instant,
    valid_until: Instant,
    refreshing: Arc<AtomicBool>,
}

/// PASETO validator with stale-while-revalidate revocation checking
pub struct PasetoValidator {
    public_key: Vec<u8>,
    revocation_cache: Arc<DashMap<String, RevocationStatus>>,
    redis_client: ConnectionManager,
    fresh_ttl: Duration,
    stale_ttl: Duration,
}

impl PasetoValidator {
    /// Create a new PASETO validator
    pub async fn new(
        public_key_hex: &str,
        redis_client: ConnectionManager,
        fresh_ttl_seconds: u64,
        stale_ttl_seconds: u64,
    ) -> Result<Self> {
        let public_key = hex::decode(public_key_hex)?;

        Ok(Self {
            public_key,
            revocation_cache: Arc::new(DashMap::new()),
            redis_client,
            fresh_ttl: Duration::from_secs(fresh_ttl_seconds),
            stale_ttl: Duration::from_secs(stale_ttl_seconds),
        })
    }

    /// Validate a PASETO token with stale-while-revalidate revocation checking
    pub async fn validate(&self, token: &str) -> Result<TokenClaims> {
        // Step 1: Verify PASETO signature (~10μs, no network)
        let claims = self.verify_token(token)?;

        // Step 2: Check expiration
        if claims.exp < Utc::now() {
            return Err(anyhow!("Token expired"));
        }

        // Step 3: Check revocation with stale-while-revalidate
        let key_id = &claims.key_id;

        if let Some(status) = self.revocation_cache.get(key_id) {
            let now = Instant::now();

            // Case 1: Fresh - serve immediately
            if now < status.fresh_until {
                if status.is_revoked {
                    return Err(anyhow!("Token revoked"));
                }
                return Ok(claims);
            }

            // Case 2: Stale but valid - serve stale + refresh in background
            if now < status.valid_until {
                let result = if status.is_revoked {
                    Err(anyhow!("Token revoked"))
                } else {
                    Ok(claims.clone())
                };

                // Trigger background refresh (only if not already refreshing)
                if !status.refreshing.swap(true, Ordering::Relaxed) {
                    let cache = self.revocation_cache.clone();
                    let redis = self.redis_client.clone();
                    let key_id = key_id.clone();
                    let fresh_ttl = self.fresh_ttl;
                    let stale_ttl = self.stale_ttl;

                    tokio::spawn(async move {
                        if let Err(e) = Self::refresh_revocation_status(
                            &cache, &redis, &key_id, fresh_ttl, stale_ttl,
                        )
                        .await
                        {
                            warn!("Background revocation refresh failed: {}", e);
                        }
                    });
                }

                return result;
            }

            // Case 3: Expired - remove from cache, fall through to Redis check
            drop(status);
            self.revocation_cache.remove(key_id);
        }

        // Cache miss or expired - check Redis (blocking, but rare)
        let is_revoked = self.check_redis_revocation(key_id).await?;

        // Cache the result
        let now = Instant::now();
        self.revocation_cache.insert(
            key_id.clone(),
            RevocationStatus {
                is_revoked,
                cached_at: now,
                fresh_until: now + self.fresh_ttl,
                valid_until: now + self.stale_ttl,
                refreshing: Arc::new(AtomicBool::new(false)),
            },
        );

        if is_revoked {
            Err(anyhow!("Token revoked"))
        } else {
            Ok(claims)
        }
    }

    /// Verify PASETO token signature and decode claims
    fn verify_token(&self, token: &str) -> Result<TokenClaims> {
        // Convert bytes to array and create public key
        let key_bytes: [u8; 32] = self.public_key[..].try_into()?;
        let key = Key::<32>::try_from(&key_bytes[..])?;
        let public_key = PasetoAsymmetricPublicKey::<V4, Public>::from(&key);

        // Parse and verify token
        let verified_payload = PasetoParser::<V4, Public>::default()
            .parse(token, &public_key)?;

        // Convert JsonValue to TokenClaims
        let claims: TokenClaims = serde_json::from_value(verified_payload)?;

        Ok(claims)
    }

    /// Check if a key is revoked in Redis
    async fn check_redis_revocation(&self, key_id: &str) -> Result<bool> {
        let mut conn = self.redis_client.clone();
        let exists: bool = conn
            .exists(format!("revoked:{}", key_id))
            .await
            .unwrap_or(false);
        Ok(exists)
    }

    /// Background refresh of revocation status
    async fn refresh_revocation_status(
        cache: &DashMap<String, RevocationStatus>,
        redis: &ConnectionManager,
        key_id: &str,
        fresh_ttl: Duration,
        stale_ttl: Duration,
    ) -> Result<()> {
        let mut conn = redis.clone();
        let is_revoked: bool = conn
            .exists(format!("revoked:{}", key_id))
            .await
            .unwrap_or(false);

        let now = Instant::now();
        cache.insert(
            key_id.to_string(),
            RevocationStatus {
                is_revoked,
                cached_at: now,
                fresh_until: now + fresh_ttl,
                valid_until: now + stale_ttl,
                refreshing: Arc::new(AtomicBool::new(false)),
            },
        );

        info!(
            "Background revocation refresh: key {} revoked={}",
            key_id, is_revoked
        );
        Ok(())
    }

    /// Periodically clean up expired cache entries
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.revocation_cache
            .retain(|_, entry| now < entry.valid_until);
    }
}

/// Global PASETO validator instance
static PASETO_VALIDATOR: once_cell::sync::OnceCell<PasetoValidator> =
    once_cell::sync::OnceCell::new();

/// Initialize the global PASETO validator
pub async fn init_paseto_validator() -> Result<()> {
    let settings = config::get_settings();

    // Get Redis connection
    let redis_client = redis::Client::open(settings.redis_url.as_str())?;
    let conn = ConnectionManager::new(redis_client).await?;

    let validator = PasetoValidator::new(
        &settings.paseto_public_key,
        conn,
        300,  // 5 minutes fresh TTL
        3600, // 60 minutes stale TTL
    )
    .await?;

    PASETO_VALIDATOR
        .set(validator)
        .map_err(|_| anyhow::anyhow!("PASETO validator already initialized"))?;

    info!("PASETO validator initialized");
    Ok(())
}

/// Get the global PASETO validator
pub fn get_validator() -> &'static PasetoValidator {
    PASETO_VALIDATOR
        .get()
        .expect("PASETO validator not initialized")
}
