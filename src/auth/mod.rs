use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use coset::{
    cwt::{ClaimsSetBuilder, Timestamp},
    CborSerializable, CoseSign1Builder, HeaderBuilder, iana,
};
use dashmap::DashMap;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};
use uuid::Uuid;

use crate::config;
use crate::models::TierType;

/// CBOR-encoded token data (ultra-compact binary format with fixed-length fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    /// Expiration time (Unix timestamp)
    #[serde(rename = "e")]
    pub expiration: i64,
    /// User ID
    #[serde(rename = "u")]
    pub user_id: i64,
    /// API key ID (UUIDv7 - time-ordered, 16 bytes fixed)
    #[serde(rename = "k")]
    pub key_id: Uuid,
    /// User tier (serializes as 0=Free, 1=Pro, 2=Scale)
    #[serde(rename = "t")]
    pub tier: TierType,
    /// Max tokens
    #[serde(rename = "m")]
    pub max_tokens: i32,
    /// Monthly quota
    #[serde(rename = "q")]
    pub monthly_quota: i32,
}

/// Token claims with CBOR-encoded data
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
        self.data.user_id
    }

    /// Get key_id
    pub fn key_id(&self) -> Uuid {
        self.data.key_id
    }

    /// Get tier
    pub fn tier(&self) -> Result<TierType, anyhow::Error> {
        Ok(self.data.tier)
    }

    /// Get expiration
    pub fn exp(&self) -> Result<DateTime<Utc>, anyhow::Error> {
        Ok(DateTime::from_timestamp(self.data.expiration, 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid timestamp"))?)
    }

    /// Get max_tokens
    pub fn max_tokens(&self) -> usize {
        self.data.max_tokens as usize
    }

    /// Get monthly_quota
    pub fn monthly_quota(&self) -> i32 {
        self.data.monthly_quota
    }
}

/// Maximum allowed CBOR payload size (2KB - reasonable for CWT ClaimsSet)
const MAX_CBOR_SIZE: usize = 2048;

/// Sign token data with Ed25519 using COSET CWT (CBOR Web Token)
/// Format: base64(COSE_Sign1(CWT ClaimsSet))
/// Uses RFC 8392 (CWT) and RFC 8152 (COSE) standards
pub fn sign_token_direct(
    token_data: &TokenData,
    signing_key: &ed25519_dalek::SigningKey,
) -> Result<String, anyhow::Error> {
    // Validate timestamp is reasonable (not negative, not absurdly far in future)
    if token_data.expiration < 0 {
        return Err(anyhow!("Invalid expiration: timestamp cannot be negative"));
    }
    // Max year ~2100 (4102444800 = 2100-01-01 00:00:00 UTC)
    if token_data.expiration > 4102444800 {
        return Err(anyhow!("Invalid expiration: timestamp too far in future"));
    }

    // Build CWT ClaimsSet with standard and custom claims
    // Use text claims for compact encoding (single-letter keys)
    let claims = ClaimsSetBuilder::new()
        .subject(token_data.user_id.to_string())
        .expiration_time(Timestamp::WholeSeconds(token_data.expiration))
        .text_claim("k".to_string(), ciborium::value::Value::Text(token_data.key_id.to_string()))
        .text_claim("t".to_string(), ciborium::value::Value::Integer((token_data.tier as i64).into()))
        .text_claim("m".to_string(), ciborium::value::Value::Integer((token_data.max_tokens as i64).into()))
        .text_claim("q".to_string(), ciborium::value::Value::Integer((token_data.monthly_quota as i64).into()))
        .build();

    // Serialize ClaimsSet to CBOR
    let claims_bytes = claims.to_vec()
        .map_err(|e| anyhow!("Failed to serialize CWT ClaimsSet: {}", e))?;

    // Validate CBOR size is reasonable
    if claims_bytes.len() > MAX_CBOR_SIZE {
        return Err(anyhow!(
            "Token data too large: {} bytes exceeds maximum of {} bytes",
            claims_bytes.len(),
            MAX_CBOR_SIZE
        ));
    }

    // Create COSE protected header with EdDSA algorithm
    let protected = HeaderBuilder::new()
        .algorithm(iana::Algorithm::EdDSA)
        .build();

    // Create COSE_Sign1 structure with ClaimsSet as payload
    let mut sign1 = CoseSign1Builder::new()
        .protected(protected)
        .payload(claims_bytes)
        .build();

    // Sign with Ed25519 using COSE Sig_structure
    use ed25519_dalek::Signer;
    let tbs = sign1.tbs_data(b"Signature1");
    let signature = signing_key.sign(&tbs);
    sign1.signature = signature.to_bytes().to_vec();

    // Serialize COSE_Sign1 to CBOR
    let cwt_bytes = sign1.to_vec()
        .map_err(|e| anyhow!("Failed to serialize COSE_Sign1: {}", e))?;

    // Base64 encode the CWT token
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &cwt_bytes,
    ))
}

/// Verify and decode CWT token using COSET
/// Validates COSE structure, Ed25519 signature, and decodes CWT ClaimsSet
pub fn verify_token_direct(
    token: &str,
    verifying_key: &ed25519_dalek::VerifyingKey,
) -> Result<TokenClaims, anyhow::Error> {
    // Decode base64
    let cwt_bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, token)?;

    // Validate size constraints
    if cwt_bytes.len() < 100 {
        return Err(anyhow!("Token too short: minimum CWT size is ~100 bytes"));
    }

    let max_len = MAX_CBOR_SIZE + 200; // ClaimsSet + COSE overhead
    if cwt_bytes.len() > max_len {
        return Err(anyhow!(
            "Token too large: {} bytes exceeds maximum",
            cwt_bytes.len()
        ));
    }

    // Deserialize COSE_Sign1 from CBOR
    let sign1 = coset::CoseSign1::from_slice(&cwt_bytes)
        .map_err(|e| anyhow!("Invalid COSE_Sign1 structure: {}", e))?;

    // Verify algorithm is EdDSA
    let protected = &sign1.protected.header;
    if protected.alg != Some(coset::Algorithm::Assigned(iana::Algorithm::EdDSA)) {
        return Err(anyhow!("Invalid algorithm: expected EdDSA"));
    }

    // Verify signature using COSE Sig_structure
    use ed25519_dalek::Verifier;
    let tbs = sign1.tbs_data(b"Signature1");
    let signature = ed25519_dalek::Signature::from_slice(&sign1.signature)
        .map_err(|e| anyhow!("Invalid signature format: {}", e))?;

    verifying_key.verify(&tbs, &signature)
        .map_err(|e| anyhow!("Signature verification failed: {}", e))?;

    // Extract and deserialize CWT ClaimsSet from payload
    let payload = sign1.payload.as_ref()
        .ok_or_else(|| anyhow!("Missing CWT payload"))?;

    let claims = coset::cwt::ClaimsSet::from_slice(payload)
        .map_err(|e| anyhow!("Invalid CWT ClaimsSet: {}", e))?;

    // Extract standard claims
    let user_id: i64 = claims.subject
        .as_ref()
        .ok_or_else(|| anyhow!("Missing subject claim"))?
        .parse()
        .map_err(|e| anyhow!("Invalid subject (user_id): {}", e))?;

    let expiration = match claims.expiration_time {
        Some(Timestamp::WholeSeconds(secs)) => secs,
        Some(Timestamp::FractionalSeconds(secs)) => secs as i64,
        None => return Err(anyhow!("Missing expiration_time claim")),
    };

    // Extract custom text claims
    let mut key_id_str = None;
    let mut tier_value = None;
    let mut max_tokens_value = None;
    let mut monthly_quota_value = None;

    for (name, value) in &claims.rest {
        match name {
            coset::cwt::ClaimName::Text(key) if key == "k" => {
                if let ciborium::value::Value::Text(s) = value {
                    key_id_str = Some(s.clone());
                }
            }
            coset::cwt::ClaimName::Text(key) if key == "t" => {
                if let ciborium::value::Value::Integer(i) = value {
                    // Convert ciborium::Integer to i128, then to u8
                    let val: i128 = (*i).into();
                    tier_value = Some(val as u8);
                }
            }
            coset::cwt::ClaimName::Text(key) if key == "m" => {
                if let ciborium::value::Value::Integer(i) = value {
                    // Convert ciborium::Integer to i128, then to i32
                    let val: i128 = (*i).into();
                    max_tokens_value = Some(val as i32);
                }
            }
            coset::cwt::ClaimName::Text(key) if key == "q" => {
                if let ciborium::value::Value::Integer(i) = value {
                    // Convert ciborium::Integer to i128, then to i32
                    let val: i128 = (*i).into();
                    monthly_quota_value = Some(val as i32);
                }
            }
            _ => {} // Ignore unknown claims
        }
    }

    // Reconstruct TokenData from extracted claims
    let key_id = key_id_str
        .ok_or_else(|| anyhow!("Missing 'k' (key_id) claim"))?;
    let key_id = Uuid::parse_str(&key_id)
        .map_err(|e| anyhow!("Invalid key_id UUID: {}", e))?;

    let tier = TierType::from_u8(tier_value
        .ok_or_else(|| anyhow!("Missing 't' (tier) claim"))?)
        .map_err(|e| anyhow!("Invalid tier value: {}", e))?;

    let max_tokens = max_tokens_value
        .ok_or_else(|| anyhow!("Missing 'm' (max_tokens) claim"))?;

    let monthly_quota = monthly_quota_value
        .ok_or_else(|| anyhow!("Missing 'q' (monthly_quota) claim"))?;

    let token_data = TokenData {
        expiration,
        user_id,
        key_id,
        tier,
        max_tokens,
        monthly_quota,
    };

    Ok(TokenClaims::from_token_data(token_data))
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

/// Token validator with stale-while-revalidate revocation checking
pub struct TokenValidator {
    public_key: Vec<u8>,
    revocation_cache: Arc<DashMap<String, RevocationStatus>>,
    redis_client: ConnectionManager,
    fresh_ttl: Duration,
    stale_ttl: Duration,
}

impl TokenValidator {
    /// Create a new token validator
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
            &self.public_key[..]
                .try_into()
                .map_err(|_| anyhow!("Invalid public key length"))?,
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

/// Global token validator instance
static TOKEN_VALIDATOR: once_cell::sync::OnceCell<TokenValidator> =
    once_cell::sync::OnceCell::new();

/// Initialize the global token validator
pub async fn init_token_validator() -> Result<()> {
    let settings = config::get_settings();

    // Get Redis connection
    let redis_client = redis::Client::open(settings.redis_url.as_str())?;
    let conn = ConnectionManager::new(redis_client).await?;

    let validator = TokenValidator::new(
        &settings.token_public_key,
        conn,
        300,  // 5 minutes fresh TTL
        3600, // 60 minutes stale TTL
    )
    .await?;

    TOKEN_VALIDATOR
        .set(validator)
        .map_err(|_| anyhow::anyhow!("Token validator already initialized"))?;

    info!("Token validator initialized");
    Ok(())
}

/// Get the global token validator
pub fn get_validator() -> &'static TokenValidator {
    TOKEN_VALIDATOR
        .get()
        .expect("Token validator not initialized")
}
