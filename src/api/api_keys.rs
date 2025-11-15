use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::auth::session::SessionClaims;
use crate::auth::{sign_token_direct, TokenData};
use crate::config;
use crate::database;
use crate::models::{APIKey, APIKeyResponse, CreateAPIKeyRequest, OrganizationRole, TierType};

use super::users::ApiError;

/// Create a new API key (CWT token) for an organization
pub async fn create_api_key_handler(
    claims: SessionClaims,
    Path(org_id): Path<uuid::Uuid>,
    Json(payload): Json<CreateAPIKeyRequest>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();
    let user_id: uuid::Uuid = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;

    // Check if user is a member of the organization
    #[derive(sqlx::FromRow)]
    struct MemberInfo {
        role: OrganizationRole,
        tier: TierType,
    }

    let member = sqlx::query_as::<_, MemberInfo>(
        "SELECT om.role, o.tier
         FROM organization_members om
         INNER JOIN organizations o ON om.organization_id = o.id
         WHERE om.organization_id = $1 AND om.user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| {
        ApiError::Unauthorized("You are not a member of this organization".to_string())
    })?;

    // Only owners and admins can create API keys
    if member.role != OrganizationRole::Owner && member.role != OrganizationRole::Admin {
        return Err(ApiError::Unauthorized(
            "Only owners and admins can create API keys".to_string(),
        ));
    }

    // Get organization tier (use provided tier or organization's tier)
    let tier = payload.tier.unwrap_or(member.tier);

    // Generate key_id (UUIDv7)
    let key_id = Uuid::now_v7();

    // Create API key record in database
    let api_key = sqlx::query_as::<_, APIKey>(
        "INSERT INTO api_keys (organization_id, key_id, name, is_active, created_at, last_used_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING *",
    )
    .bind(org_id)
    .bind(key_id)
    .bind(&payload.name)
    .bind(true)
    .bind(Utc::now().naive_utc())
    .bind(None::<chrono::NaiveDateTime>)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create API key: {}", e)))?;

    // Generate CWT token
    let settings = config::get_settings();
    let private_key_bytes = hex::decode(&settings.token_private_key)
        .map_err(|e| ApiError::InternalError(format!("Invalid private key: {}", e)))?;

    let signing_key = ed25519_dalek::SigningKey::from_bytes(
        &private_key_bytes[..]
            .try_into()
            .map_err(|_| ApiError::InternalError("Invalid private key length".to_string()))?,
    );

    // Create token data
    let (max_tokens, monthly_quota) = get_tier_limits(tier);

    let token_data = TokenData {
        org_id,
        key_id,
        tier,
        max_tokens: max_tokens as i32,
        monthly_quota,
    };

    let token = sign_token_direct(&token_data, &signing_key)
        .map_err(|e| ApiError::InternalError(format!("Failed to sign token: {}", e)))?;

    // Add prefix to token
    let prefixed_token = format!("{}{}", settings.api_key_prefix, token);

    let response = APIKeyResponse {
        id: api_key.id,
        key_id: api_key.key_id,
        name: api_key.name,
        is_active: api_key.is_active,
        created_at: api_key.created_at,
        last_used_at: api_key.last_used_at,
        token: Some(prefixed_token),
    };

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// List API keys for an organization
pub async fn list_api_keys_handler(
    claims: SessionClaims,
    Path(org_id): Path<i64>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;

    // Check if user is a member of the organization
    let member_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM organization_members WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if member_exists == 0 {
        return Err(ApiError::Unauthorized(
            "You are not a member of this organization".to_string(),
        ));
    }

    // Get API keys
    let api_keys = sqlx::query_as::<_, APIKey>(
        "SELECT * FROM api_keys WHERE organization_id = $1 ORDER BY created_at DESC",
    )
    .bind(org_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let responses: Vec<APIKeyResponse> = api_keys
        .into_iter()
        .map(|key| APIKeyResponse {
            id: key.id,
            key_id: key.key_id,
            name: key.name,
            is_active: key.is_active,
            created_at: key.created_at,
            last_used_at: key.last_used_at,
            token: None, // Don't return token in list
        })
        .collect();

    Ok((StatusCode::OK, Json(responses)).into_response())
}

/// Revoke an API key
pub async fn revoke_api_key_handler(
    claims: SessionClaims,
    Path((org_id, key_id)): Path<(i64, i64)>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;

    // Check if user is owner or admin of the organization
    let member_role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM organization_members WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| {
        ApiError::Unauthorized("You are not a member of this organization".to_string())
    })?;

    let role: OrganizationRole = serde_json::from_str(&format!("\"{}\"", member_role))
        .map_err(|e| ApiError::InternalError(format!("Invalid role: {}", e)))?;

    if role != OrganizationRole::Owner && role != OrganizationRole::Admin {
        return Err(ApiError::Unauthorized(
            "Only owners and admins can revoke API keys".to_string(),
        ));
    }

    // Deactivate the API key
    let result =
        sqlx::query("UPDATE api_keys SET is_active = false WHERE id = $1 AND organization_id = $2")
            .bind(key_id)
            .bind(org_id)
            .execute(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::BadRequest("API key not found".to_string()));
    }

    // TODO: Add key to Redis revocation list
    // This requires getting the UUID key_id from the database first
    let uuid_key_id = sqlx::query_scalar::<_, Uuid>("SELECT key_id FROM api_keys WHERE id = $1")
        .bind(key_id)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    // Add to Redis revocation list (expires in 1 year - same as token expiration)
    if let Ok(redis_client) = redis::Client::open(config::get_settings().redis_url.as_str()) {
        if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
            use redis::AsyncCommands;
            let _: Result<(), _> = conn
                .set_ex(
                    format!("revoked:{}", uuid_key_id),
                    1,
                    365 * 24 * 60 * 60, // 1 year in seconds
                )
                .await;
        }
    }

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "API key revoked successfully" })),
    )
        .into_response())
}

/// Get tier limits
fn get_tier_limits(tier: TierType) -> (usize, i32) {
    let settings = config::get_settings();
    match tier {
        TierType::Free => (settings.max_tokens, settings.free_tier_limit),
        TierType::Pro => (settings.max_tokens, settings.pro_tier_limit),
        TierType::Scale => (settings.max_tokens, settings.scale_tier_limit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::helpers::{cleanup_db, create_test_user, setup};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use serde_json::json;
    use serial_test::serial;
    use tower::ServiceExt;

    fn app() -> Router {
        Router::new()
            .route(
                "/organizations/:org_id/keys",
                axum::routing::post(create_api_key_handler),
            )
            .route(
                "/organizations/:org_id/keys",
                axum::routing::get(list_api_keys_handler),
            )
            .route(
                "/organizations/:org_id/keys/:key_id",
                axum::routing::delete(revoke_api_key_handler),
            )
    }

    #[tokio::test]
    #[serial]
    async fn test_create_api_key() {
        setup().await;
        cleanup_db().await;

        let (_user_id, token, org_id) = create_test_user("test@example.com", "password123").await;

        let app = app();

        let payload = json!({
            "name": "Test API Key"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/organizations/{}/keys", org_id))
                    .header("authorization", format!("Bearer {}", token))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let key_response: APIKeyResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(key_response.name, "Test API Key");
        assert!(key_response.is_active);
        assert!(key_response.token.is_some());

        // Verify token starts with prefix
        let token_str = key_response.token.unwrap();
        assert!(token_str.starts_with("fe_"));

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_list_api_keys() {
        setup().await;
        cleanup_db().await;

        let (_user_id, token, org_id) = create_test_user("test@example.com", "password123").await;

        // Create a key first
        let app1 = app();
        let payload = json!({"name": "Test Key"});

        app1.oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/organizations/{}/keys", org_id))
                .header("authorization", format!("Bearer {}", token))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

        // Now list keys
        let app2 = app();
        let response = app2
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/organizations/{}/keys", org_id))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let keys: Vec<APIKeyResponse> = serde_json::from_slice(&body).unwrap();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "Test Key");
        // Token should not be included in list
        assert!(keys[0].token.is_none());

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_revoke_api_key() {
        setup().await;
        cleanup_db().await;

        let (_user_id, token, org_id) = create_test_user("test@example.com", "password123").await;

        // Create a key first
        let app1 = app();
        let payload = json!({"name": "Key to Revoke"});

        let create_response = app1
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/organizations/{}/keys", org_id))
                    .header("authorization", format!("Bearer {}", token))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let key_response: APIKeyResponse = serde_json::from_slice(&body).unwrap();

        // Now revoke it
        let app2 = app();
        let response = app2
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(&format!(
                        "/organizations/{}/keys/{}",
                        org_id, key_response.id
                    ))
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_create_api_key_non_member() {
        setup().await;
        cleanup_db().await;

        let (_user_id1, _token1, org_id1) =
            create_test_user("owner@example.com", "password123").await;
        let (_user_id2, token2, _org_id2) =
            create_test_user("other@example.com", "password123").await;

        let app = app();

        let payload = json!({
            "name": "Unauthorized Key"
        });

        // Try to create key in org1 using token2 (not a member)
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/organizations/{}/keys", org_id1))
                    .header("authorization", format!("Bearer {}", token2))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        cleanup_db().await;
    }
}
