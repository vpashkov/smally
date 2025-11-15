use axum::{
    async_trait,
    extract::{FromRequestParts, Json},
    http::{request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use utoipa::ToSchema;

use crate::{auth, billing, cache, config, inference, monitoring};

pub mod api_keys;
pub mod organizations;
pub mod users;

/// Request to create text embeddings
#[derive(Debug, Deserialize, ToSchema)]
pub struct EmbedRequest {
    /// Text to embed (max 2000 characters)
    #[schema(example = "Hello world")]
    pub text: String,
    /// Whether to L2 normalize the embedding vector
    #[serde(default)]
    #[schema(default = false)]
    pub normalize: bool,
}

/// Embedding response with metadata
#[derive(Debug, Serialize, ToSchema)]
pub struct EmbedResponse {
    /// 384-dimensional embedding vector
    #[schema(value_type = Vec<f32>, example = json!([0.1, 0.2, 0.3]))]
    pub embedding: Vec<f32>,
    /// Model used for embedding
    #[schema(example = "all-MiniLM-L6-v2")]
    pub model: String,
    /// Number of tokens in input text
    #[schema(example = 5)]
    pub tokens: usize,
    /// Whether result was served from cache
    #[schema(example = false)]
    pub cached: bool,
    /// Total request latency in milliseconds
    #[schema(example = 25.3)]
    pub latency_ms: f64,
}

/// Error response
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Error type
    #[schema(example = "invalid_request")]
    pub error: String,
    /// Human-readable error message
    #[schema(example = "Text cannot be empty")]
    pub message: String,
    /// Maximum allowed tokens (for token limit errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Rate limit reset timestamp (for rate limit errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    #[schema(example = "healthy")]
    pub status: String,
    /// API version
    #[schema(example = "0.1.0")]
    pub version: String,
    /// Embedding model name
    #[schema(example = "sentence-transformers/all-MiniLM-L6-v2")]
    pub model: String,
    /// Build information
    pub build: BuildInfo,
}

/// Build and version information
#[derive(Debug, Serialize, ToSchema)]
pub struct BuildInfo {
    /// Git commit hash
    pub git_hash: String,
    /// Git branch name
    pub git_branch: String,
    /// Git commit date
    pub git_date: String,
    /// Whether build includes uncommitted changes
    pub git_dirty: bool,
    /// Build timestamp
    pub build_timestamp: String,
    /// Rust compiler version
    pub rust_version: String,
    /// Build profile (debug/release)
    pub profile: String,
}

/// Health check endpoint
///
/// Returns service status, version, and build information
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
pub async fn health_handler() -> Json<HealthResponse> {
    let settings = config::get_settings();

    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };

    Json(HealthResponse {
        status: "healthy".to_string(),
        version: settings.version.clone(),
        model: settings.model_name.clone(),
        build: BuildInfo {
            git_hash: env!("GIT_HASH").to_string(),
            git_branch: env!("GIT_BRANCH").to_string(),
            git_date: env!("GIT_DATE").to_string(),
            git_dirty: env!("GIT_DIRTY").parse().unwrap_or(false),
            build_timestamp: env!("BUILD_TIMESTAMP").to_string(),
            rust_version: env!("RUST_VERSION").to_string(),
            profile: profile.to_string(),
        },
    })
}

/// API information endpoint
///
/// Returns basic API information and available endpoints
#[utoipa::path(
    get,
    path = "/",
    tag = "health",
    responses(
        (status = 200, description = "API information")
    )
)]
pub async fn root_handler() -> Json<serde_json::Value> {
    let settings = config::get_settings();

    Json(serde_json::json!({
        "name": settings.app_name,
        "version": settings.version,
        "endpoints": {
            "/v1/embed": "POST - Create embeddings",
            "/health": "GET - Health check",
            "/metrics": "GET - Prometheus metrics"
        }
    }))
}

/// Create text embeddings
///
/// Generates a 384-dimensional embedding vector for the input text using
/// the all-MiniLM-L6-v2 sentence transformer model.
///
/// The endpoint supports caching for faster responses and includes rate limiting
/// based on your subscription tier.
#[utoipa::path(
    post,
    path = "/v1/embed",
    tag = "embeddings",
    request_body = EmbedRequest,
    responses(
        (status = 200, description = "Successfully generated embedding", body = EmbedResponse,
         headers(
             ("X-RateLimit-Limit" = String, description = "Monthly request limit"),
             ("X-RateLimit-Remaining" = String, description = "Remaining requests this month"),
             ("X-RateLimit-Reset" = String, description = "Reset timestamp")
         )
        ),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized - invalid or missing API key", body = ErrorResponse),
        (status = 429, description = "Rate limit exceeded", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_embedding_handler(
    headers: HeaderMap,
    Json(req): Json<EmbedRequest>,
) -> Result<Response, ApiError> {
    let start_time = Instant::now();

    // Generate request ID for tracking
    let request_id = uuid::Uuid::now_v7();

    // Get authorization header
    let auth_header = headers.get("authorization").ok_or(ApiError::Unauthorized(
        "Authorization header is required".to_string(),
    ))?;

    // Convert header value to string - handle both ASCII and UTF-8
    let auth_str = auth_header.to_str().unwrap_or_else(|_| {
        // Try as bytes
        std::str::from_utf8(auth_header.as_bytes()).unwrap_or("")
    });

    // Extract Bearer token
    let parts: Vec<&str> = auth_str.splitn(2, ' ').collect();
    if parts.len() != 2 || parts[0].to_lowercase() != "bearer" {
        return Err(ApiError::Unauthorized(
            "Authorization header must be 'Bearer <token>'".to_string(),
        ));
    }

    let full_token = parts[1];

    // Check if token has configured prefix and strip it
    let settings = config::get_settings();
    let token = if full_token.starts_with(&settings.api_key_prefix) {
        &full_token[settings.api_key_prefix.len()..] // Remove prefix
    } else {
        // Allow tokens without prefix for backward compatibility
        full_token
    };

    // Validate token
    let validator = auth::get_validator();
    let claims = validator
        .validate(token)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Token validation failed: {}", e)))?;

    // Validate text
    if req.text.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Text cannot be empty or only whitespace".to_string(),
        ));
    }

    if req.text.len() > 2000 {
        return Err(ApiError::BadRequest(
            "Text exceeds 2000 characters".to_string(),
        ));
    }

    // Get settings early
    let settings = config::get_settings();

    // Fast validation: estimate tokens from text length
    // Average: ~4 chars per token for BERT tokenizers
    let estimated_tokens = req.text.len() / 4;

    // Reject if estimate is way over limit (2x buffer for safety)
    if estimated_tokens > settings.max_tokens * 2 {
        monitoring::ERROR_COUNT
            .with_label_values(&["text_too_long"])
            .inc();
        return Err(ApiError::BadRequestWithTokens(
            format!(
                "Input text too long (estimated ~{} tokens, max {})",
                estimated_tokens, settings.max_tokens
            ),
            settings.max_tokens,
        ));
    }

    // Record request immediately to api_request_log (audit trail)
    let buffer = billing::get_usage_buffer();
    buffer.record_request(
        request_id,
        claims.org_id(),
        claims.key_id(),
        "embeddings".to_string(),
        "/v1/embed".to_string(),
        req.text.clone(),
        Some(serde_json::json!({
            "normalize": req.normalize
        })),
    );

    // Get model and cache
    let model = inference::get_model();
    let cache = cache::get_cache();

    // Check rate limit using token claims
    let (is_allowed, rate_limit_info) = billing::check_rate_limit_from_claims(&claims)
        .await
        .map_err(|_| ApiError::InternalError("Failed to check rate limit".to_string()))?;

    if !is_allowed {
        let tier = format!(
            "{:?}",
            claims
                .tier()
                .map_err(|_| ApiError::InternalError("Failed to decode tier".to_string()))?
        )
        .to_lowercase();
        monitoring::RATE_LIMIT_EXCEEDED
            .with_label_values(&[&tier])
            .inc();

        let reset_at = rate_limit_info.get("reset_at").cloned();
        return Err(ApiError::RateLimitExceeded(
            "Monthly quota exhausted".to_string(),
            reset_at,
        ));
    }

    // Check cache
    let (embedding, model_name, cached, exact_tokens) =
        if let Some(cached_data) = cache.get(&req.text).await {
            monitoring::CACHE_HITS.with_label_values(&["total"]).inc();

            // Cache hit: use metadata from cache (no token counting needed!)
            (
                cached_data.embedding,
                cached_data.model,
                true,
                cached_data.tokens,
            )
        } else {
            // Cache miss: generate embedding
            let (embedding, metadata) = {
                let mut model_lock = model.write();
                model_lock.encode(&req.text, req.normalize).map_err(|_| {
                    monitoring::ERROR_COUNT
                        .with_label_values(&["inference_error"])
                        .inc();
                    ApiError::InternalError("Failed to generate embedding".to_string())
                })?
            };

            // Record inference time
            monitoring::INFERENCE_LATENCY.observe(metadata.inference_time_ms / 1000.0);
            monitoring::CACHE_MISSES.inc();

            // Cache the result WITH metadata
            cache
                .set(
                    &req.text,
                    cache::CachedEmbedding {
                        embedding: embedding.clone(),
                        tokens: metadata.tokens,
                        model: metadata.model.clone(),
                    },
                )
                .await;

            // Use tokens from inference metadata (already counted!)
            (embedding, metadata.model, false, metadata.tokens)
        };

    // Increment Redis counter for free tier rate limiting
    let tier = claims
        .tier()
        .map_err(|_| ApiError::InternalError("Failed to decode tier".to_string()))?;
    if tier == crate::models::TierType::Free {
        billing::increment_free_tier_counter(claims.org_id());
    }

    let mut headers = HeaderMap::new();
    if let Some(limit) = rate_limit_info.get("limit") {
        if let Ok(value) = limit.parse() {
            headers.insert("X-RateLimit-Limit", value);
        }
    }
    if let Some(remaining) = rate_limit_info.get("remaining") {
        if let Ok(value) = remaining.parse() {
            headers.insert("X-RateLimit-Remaining", value);
        }
    }
    if let Some(reset_at) = rate_limit_info.get("reset_at") {
        if let Ok(value) = reset_at.parse() {
            headers.insert("X-RateLimit-Reset", value);
        }
    }

    monitoring::TOKEN_COUNT.observe(exact_tokens as f64);
    monitoring::REQUEST_COUNT
        .with_label_values(&["success", &cached.to_string()])
        .inc();

    // Calculate total latency
    let total_latency_ms = start_time.elapsed().as_millis() as f64;

    monitoring::REQUEST_LATENCY.observe(total_latency_ms / 1000.0);

    // Record response with exact token count (for billing)
    buffer.record_response(
        request_id,
        claims.org_id(),
        claims.key_id(),
        "embeddings",
        exact_tokens as i32,
        serde_json::json!({
            "model": model_name,
            "cached": cached,
            "latency_ms": total_latency_ms,
            "normalize": req.normalize
        }),
    );

    let response = EmbedResponse {
        embedding,
        model: model_name,
        tokens: exact_tokens,
        cached,
        latency_ms: total_latency_ms,
    };

    Ok((StatusCode::OK, headers, Json(response)).into_response())
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    BadRequestWithTokens(String, usize),
    Unauthorized(String),
    RateLimitExceeded(String, Option<String>),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message, max_tokens, reset_at) = match self {
            ApiError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, "invalid_request", msg, None, None)
            }
            ApiError::BadRequestWithTokens(msg, tokens) => (
                StatusCode::BAD_REQUEST,
                "text_too_long",
                msg,
                Some(tokens),
                None,
            ),
            ApiError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, "invalid_api_key", msg, None, None)
            }
            ApiError::RateLimitExceeded(msg, reset) => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_exceeded",
                msg,
                None,
                reset,
            ),
            ApiError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                msg,
                None,
                None,
            ),
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message,
            max_tokens,
            reset_at,
        };

        (status, Json(error_response)).into_response()
    }
}

/// Extractor for session authentication
#[async_trait]
impl<S> FromRequestParts<S> for auth::session::SessionClaims
where
    S: Send + Sync,
{
    type Rejection = users::ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get authorization header
        let auth_header = parts.headers.get("authorization").ok_or_else(|| {
            users::ApiError::Unauthorized("Authorization header is required".to_string())
        })?;

        // Convert header value to string
        let auth_str = auth_header.to_str().map_err(|_| {
            users::ApiError::Unauthorized("Invalid authorization header".to_string())
        })?;

        // Extract Bearer token
        let parts: Vec<&str> = auth_str.splitn(2, ' ').collect();
        if parts.len() != 2 || parts[0].to_lowercase() != "bearer" {
            return Err(users::ApiError::Unauthorized(
                "Authorization header must be 'Bearer <token>'".to_string(),
            ));
        }

        let token = parts[1];

        // Verify session token
        let claims = auth::session::verify_session_token(token)
            .map_err(|e| users::ApiError::Unauthorized(format!("Invalid session token: {}", e)))?;

        Ok(claims)
    }
}

/// Extractor for admin token authentication (protects registration/login endpoints)
#[async_trait]
impl<S> FromRequestParts<S> for auth::AdminTokenClaims
where
    S: Send + Sync,
{
    type Rejection = users::ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Get authorization header
        let auth_header = parts.headers.get("authorization").ok_or_else(|| {
            users::ApiError::Unauthorized("Authorization header is required".to_string())
        })?;

        // Convert header value to string
        let auth_str = auth_header.to_str().map_err(|_| {
            users::ApiError::Unauthorized("Invalid authorization header".to_string())
        })?;

        // Extract Bearer token
        let token_parts: Vec<&str> = auth_str.splitn(2, ' ').collect();
        if token_parts.len() != 2 || token_parts[0].to_lowercase() != "bearer" {
            return Err(users::ApiError::Unauthorized(
                "Authorization header must be 'Bearer <token>'".to_string(),
            ));
        }

        let full_token = token_parts[1];

        // Check if token has admin_ prefix
        if !full_token.starts_with("admin_") {
            return Err(users::ApiError::Unauthorized(
                "Invalid admin token format".to_string(),
            ));
        }

        // Strip prefix and validate
        let token = &full_token[6..]; // Remove "admin_" prefix

        // Get public key from settings
        let settings = config::get_settings();
        let public_key_bytes = hex::decode(&settings.token_public_key).map_err(|_| {
            users::ApiError::InternalError("Failed to decode public key".to_string())
        })?;
        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(
            &public_key_bytes[..]
                .try_into()
                .map_err(|_| users::ApiError::InternalError("Invalid public key".to_string()))?,
        )
        .map_err(|_| users::ApiError::InternalError("Invalid public key".to_string()))?;

        // Verify admin token
        let token_data = auth::validate_admin_token(token, &verifying_key)
            .map_err(|e| users::ApiError::Unauthorized(format!("Invalid admin token: {}", e)))?;

        Ok(auth::AdminTokenClaims::new(token_data))
    }
}

/// OpenAPI documentation
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        create_embedding_handler,
        health_handler,
        root_handler,
    ),
    components(
        schemas(
            EmbedRequest,
            EmbedResponse,
            ErrorResponse,
            HealthResponse,
            BuildInfo,
        )
    ),
    tags(
        (name = "embeddings", description = "Text embedding endpoints"),
        (name = "health", description = "Health check and status endpoints"),
    ),
    info(
        title = "Smally Embeddings API",
        version = "0.1.0",
        description = "Fast, production-ready text embedding API using sentence transformers",
        contact(
            name = "API Support",
            url = "https://github.com/yourusername/smally"
        ),
        license(
            name = "MIT"
        )
    ),
    servers(
        (url = "http://localhost:8000", description = "Local development server"),
        (url = "https://api.example.com", description = "Production server")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security scheme for Bearer token authentication
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Enter your API key (with or without fe_ prefix)"))
                        .build(),
                ),
            )
        }
    }
}
