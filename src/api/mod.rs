use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{auth, billing, cache, config, inference, monitoring};

#[derive(Debug, Deserialize)]
pub struct EmbedRequest {
    pub text: String,
    #[serde(default)]
    pub normalize: bool,
}

#[derive(Debug, Serialize)]
pub struct EmbedResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub tokens: usize,
    pub cached: bool,
    pub latency_ms: f64,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub model: String,
    pub build: BuildInfo,
}

#[derive(Debug, Serialize)]
pub struct BuildInfo {
    pub git_hash: String,
    pub git_branch: String,
    pub git_date: String,
    pub git_dirty: bool,
    pub build_timestamp: String,
    pub rust_version: String,
    pub profile: String,
}

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

pub async fn create_embedding_handler(
    headers: HeaderMap,
    Json(req): Json<EmbedRequest>,
) -> Result<Response, ApiError> {
    let start_time = Instant::now();

    // Get authorization header
    let auth_header = headers
        .get("authorization")
        .ok_or(ApiError::Unauthorized("Authorization header is required".to_string()))?;

    // Convert header value to string - handle both ASCII and UTF-8
    let auth_str = auth_header
        .to_str()
        .unwrap_or_else(|_| {
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

    let token = parts[1];

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
        return Err(ApiError::BadRequest("Text exceeds 2000 characters".to_string()));
    }

    // Get model and cache
    let model = inference::get_model();
    let cache = cache::get_cache();
    let settings = config::get_settings();

    // Count tokens
    let token_count = {
        let model_lock = model.read();
        model_lock.count_tokens(&req.text)
    };
    monitoring::TOKEN_COUNT.observe(token_count as f64);

    if token_count > settings.max_tokens {
        monitoring::ERROR_COUNT
            .with_label_values(&["text_too_long"])
            .inc();
        return Err(ApiError::BadRequestWithTokens(
            format!(
                "Input exceeds {} tokens (got {})",
                settings.max_tokens, token_count
            ),
            settings.max_tokens,
        ));
    }

    // Check rate limit using token claims
    let (is_allowed, rate_limit_info) = billing::check_rate_limit_from_claims(&claims)
        .await
        .map_err(|_| ApiError::InternalError("Failed to check rate limit".to_string()))?;

    if !is_allowed {
        let tier = format!("{:?}", claims.tier().map_err(|_| ApiError::InternalError("Failed to decode tier".to_string()))?).to_lowercase();
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
    let (embedding, metadata, cached) = if let Some(cached_embedding) = cache.get(&req.text).await
    {
        monitoring::CACHE_HITS.with_label_values(&["total"]).inc();
        let model_name = settings.model_name
            .split('/')
            .next_back()
            .unwrap_or("unknown")
            .to_string();
        (
            cached_embedding,
            inference::Metadata {
                model: model_name,
                tokens: token_count,
                inference_time_ms: 0.0,
            },
            true,
        )
    } else {
        // Generate embedding
        let (embedding, metadata) = {
            let mut model_lock = model.write();
            model_lock
                .encode(&req.text, req.normalize)
                .map_err(|_| {
                    monitoring::ERROR_COUNT
                        .with_label_values(&["inference_error"])
                        .inc();
                    ApiError::InternalError("Failed to generate embedding".to_string())
                })?
        };

        // Record inference time
        monitoring::INFERENCE_LATENCY.observe(metadata.inference_time_ms / 1000.0);
        monitoring::CACHE_MISSES.inc();

        // Cache the result
        cache.set(&req.text, embedding.clone()).await;

        (embedding, metadata, false)
    };

    // Increment usage using token claims
    let _ = billing::increment_usage_from_claims(&claims).await;

    // Calculate total latency
    let total_latency_ms = start_time.elapsed().as_millis() as f64;

    // Record metrics
    monitoring::REQUEST_COUNT
        .with_label_values(&["success", &cached.to_string()])
        .inc();
    monitoring::REQUEST_LATENCY.observe(total_latency_ms / 1000.0);

    // Send response
    let response = EmbedResponse {
        embedding,
        model: metadata.model,
        tokens: metadata.tokens,
        cached,
        latency_ms: total_latency_ms,
    };

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
            ApiError::InternalError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg, None, None)
            }
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
