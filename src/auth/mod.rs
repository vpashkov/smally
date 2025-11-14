use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

use crate::config;
use crate::models::TierType;

/// CBOR-encoded token data (ultra-compact binary format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// Expiration time (Unix timestamp)
    pub e: i64,
    /// User ID
    pub u: i64,
    /// API key ID (for revocation tracking)
    pub k: String,
    /// User tier (serializes as 0=Free, 1=Pro, 2=Scale)
    pub t: TierType,
    /// Max tokens
    pub m: i32,
    /// Monthly quota
    pub q: i32,
}

/// PASETO token claims with CBOR-encoded data
#[derive(Debug, Clone)]
pub struct TokenClaims {
    /// Decoded token data (cached for efficiency)
    data: TokenData,
}

impl TokenClaims {
    /// Create TokenClaims from TokenData
    pub fn from_token_data(data: TokenData) -> Self {
        Self { data }
    }

    /// Get CBOR-encoded bytes
    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut cbor_bytes = Vec::new();
        ciborium::into_writer(&self.data, &mut cbor_bytes)?;
        Ok(cbor_bytes)
    }

    /// Decode from CBOR bytes
    pub fn from_cbor_bytes(cbor_bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let data: TokenData = ciborium::from_reader(cbor_bytes)?;
        Ok(Self { data })
    }

    /// Get user_id
    pub fn user_id(&self) -> i64 {
        self.data.u
    }

    /// Get key_id
    pub fn key_id(&self) -> &str {
        &self.data.k
    }

    /// Get tier
    pub fn tier(&self) -> Result<TierType, anyhow::Error> {
        Ok(self.data.t)
    }

    /// Get expiration
    pub fn exp(&self) -> Result<DateTime<Utc>, anyhow::Error> {
        Ok(DateTime::from_timestamp(self.data.e, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?)
    }

    /// Get max_tokens
    pub fn max_tokens(&self) -> usize {
        self.data.m as usize
    }

    /// Get monthly_quota
    pub fn monthly_quota(&self) -> i32 {
        self.data.q
    }
}

/// Sign token data with Ed25519 (direct approach, no PASETO overhead)
/// Format: base64(version(1 byte) + cbor_bytes + signature(64 bytes))
pub fn sign_token_direct(
    token_data: &TokenData,
    signing_key: &ed25519_dalek::SigningKey,
) -> Result<String, anyhow::Error> {
    // Encode to CBOR
    let mut cbor_bytes = Vec::new();
    ciborium::into_writer(token_data, &mut cbor_bytes)?;

    // Prepare data to sign: version + CBOR
    const VERSION: u8 = 1;
    let mut data_to_sign = vec![VERSION];
    data_to_sign.extend_from_slice(&cbor_bytes);

    // Sign with Ed25519
    use ed25519_dalek::Signer;
    let signature = signing_key.sign(&data_to_sign);

    // Build final token: version + cbor + signature
    let mut token_bytes = data_to_sign;
    token_bytes.extend_from_slice(&signature.to_bytes());

    // Base64 encode (no prefix needed - version is first byte after decoding)
    Ok(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &token_bytes))
}

/// Verify and decode directly signed token
pub fn verify_token_direct(
    token: &str,
    verifying_key: &ed25519_dalek::VerifyingKey,
) -> Result<TokenClaims, anyhow::Error> {
    // Decode base64 (no prefix - version is embedded as first byte)
    let token_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, token)?;

    // Minimum size: version(1) + cbor(min 10) + signature(64) = 75 bytes
    if token_bytes.len() < 75 {
        return Err(anyhow!("Token too short"));
    }

    // Extract components
    let version = token_bytes[0];
    if version != 1 {
        return Err(anyhow!("Unsupported token version: {}", version));
    }

    let signature_start = token_bytes.len() - 64;
    let data_bytes = &token_bytes[..signature_start]; // version + cbor
    let signature_bytes = &token_bytes[signature_start..];

    // Verify signature
    use ed25519_dalek::Verifier;
    let signature = ed25519_dalek::Signature::from_bytes(signature_bytes.try_into()?);
    verifying_key.verify(data_bytes, &signature)?;

    // Extract CBOR (skip version byte)
    let cbor_bytes = &data_bytes[1..];

    // Decode CBOR to TokenClaims
    TokenClaims::from_cbor_bytes(cbor_bytes)
}

// Keep TokenLimits for compatibility with billing module
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

    /// Validate a directly signed token with stale-while-revalidate revocation checking
    pub async fn validate(&self, token: &str) -> Result<TokenClaims> {
        // Step 1: Verify Ed25519 signature (~10μs, no network)
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(
            &self.public_key[..].try_into()
                .map_err(|_| anyhow!("Invalid public key length"))?
        )?;
        let claims = verify_token_direct(token, &verifying_key)?;

        // Step 2: Check expiration
        if claims.exp()? < Utc::now() {
            return Err(anyhow!("Token expired"));
        }

        // Step 3: Check revocation with stale-while-revalidate
        let key_id = claims.key_id().to_string();

        if let Some(status) = self.revocation_cache.get(&key_id) {
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
            self.revocation_cache.remove(&key_id);
        }

        // Cache miss or expired - check Redis (blocking, but rare)
        let is_revoked = self.check_redis_revocation(&key_id).await?;

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
