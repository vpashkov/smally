use anyhow::Result;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use serde_json::json;
use validator::Validate;

use crate::auth::session::{create_session_token, SessionClaims};
use crate::database;
use crate::models::{AuthResponse, CreateUserRequest, LoginRequest, TierType, User, UserResponse};

/// Register a new user (requires admin token)
pub async fn register_handler(
    _admin_token: crate::auth::AdminTokenClaims,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();

    // Validate input using validator crate
    payload.validate().map_err(|e| {
        let error_msg = e
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                format!(
                    "{}: {}",
                    field,
                    errors
                        .iter()
                        .filter_map(|e| e.message.as_ref())
                        .map(|m| m.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        ApiError::BadRequest(format!("Validation failed: {}", error_msg))
    })?;

    // Additional email validation - check for disposable/temporary email domains
    let email_lower = payload.email.to_lowercase();
    let disposable_domains = [
        "tempmail.com",
        "throwaway.email",
        "guerrillamail.com",
        "10minutemail.com",
        "mailinator.com",
    ];

    if disposable_domains
        .iter()
        .any(|domain| email_lower.ends_with(domain))
    {
        return Err(ApiError::BadRequest(
            "Disposable email addresses are not allowed".to_string(),
        ));
    }

    // Check if user already exists
    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if existing_user.is_some() {
        return Err(ApiError::BadRequest("Email already registered".to_string()));
    }

    // Hash password
    let password_hash = hash(&payload.password, DEFAULT_COST)
        .map_err(|e| ApiError::InternalError(format!("Password hashing failed: {}", e)))?;

    // Create user
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, name, password_hash, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING *",
    )
    .bind(&payload.email)
    .bind(&payload.name)
    .bind(&password_hash)
    .bind(true)
    .bind(Utc::now().naive_utc())
    .bind(Utc::now().naive_utc())
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create user: {}", e)))?;

    // Create personal organization for the user
    let slug = format!("user-{}-org", user.id);
    let org_name = format!("{}' Organization", payload.email);

    let org_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO organizations (name, slug, owner_id, tier, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(&org_name)
    .bind(&slug)
    .bind(user.id)
    .bind(TierType::Free)
    .bind(true)
    .bind(Utc::now().naive_utc())
    .bind(Utc::now().naive_utc())
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create organization: {}", e)))?;

    // Add user as owner of the organization
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(org_id)
    .bind(user.id)
    .bind("owner")
    .bind(Utc::now().naive_utc())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to add organization member: {}", e)))?;

    // Generate session token
    let token = create_session_token(user.id, &user.email)
        .map_err(|e| ApiError::InternalError(format!("Failed to create session token: {}", e)))?;

    let response = AuthResponse {
        user: UserResponse {
            id: user.id,
            email: user.email.clone(),
            name: user.name.clone(),
            is_active: user.is_active,
            created_at: user.created_at,
        },
        token,
    };

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// Login user (requires admin token)
pub async fn login_handler(
    _admin_token: crate::auth::AdminTokenClaims,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();

    // Find user by email
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid email or password".to_string()))?;

    // Check if user is active
    if !user.is_active {
        return Err(ApiError::Unauthorized("Account is disabled".to_string()));
    }

    // Verify password
    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or_else(|| ApiError::Unauthorized("Invalid email or password".to_string()))?;

    let valid = verify(&payload.password, password_hash)
        .map_err(|e| ApiError::InternalError(format!("Password verification failed: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized(
            "Invalid email or password".to_string(),
        ));
    }

    // Generate session token
    let token = create_session_token(user.id, &user.email)
        .map_err(|e| ApiError::InternalError(format!("Failed to create session token: {}", e)))?;

    let response = AuthResponse {
        user: UserResponse {
            id: user.id,
            email: user.email.clone(),
            name: user.name.clone(),
            is_active: user.is_active,
            created_at: user.created_at,
        },
        token,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Get current user profile (requires authentication)
pub async fn get_profile_handler(claims: SessionClaims) -> Result<Response, ApiError> {
    let pool = database::get_db();

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(claims.sub.parse::<i64>().unwrap())
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    let response = UserResponse {
        id: user.id,
        email: user.email.clone(),
        name: user.name.clone(),
        is_active: user.is_active,
        created_at: user.created_at,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Error responses for user API
#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            ApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::helpers::{
        cleanup_db, create_test_admin_token, create_test_user, setup,
    };
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
            .route("/register", axum::routing::post(register_handler))
            .route("/login", axum::routing::post(login_handler))
            .route("/me", axum::routing::get(get_profile_handler))
    }

    #[tokio::test]
    #[serial]
    async fn test_user_registration() {
        setup().await;
        cleanup_db().await;

        let app = app();
        let admin_token = create_test_admin_token();

        let payload = json!({
            "email": "test@example.com",
            "password": "testpassword123",
            "name": "Test User"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", admin_token))
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let auth_response: AuthResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(auth_response.user.email, "test@example.com");
        assert!(!auth_response.token.is_empty());

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_duplicate_registration() {
        setup().await;
        cleanup_db().await;

        create_test_user("test@example.com", "password123").await;

        let app = app();
        let admin_token = create_test_admin_token();

        let payload = json!({
            "email": "test@example.com",
            "password": "newpassword123"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", admin_token))
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_user_login() {
        setup().await;
        cleanup_db().await;

        create_test_user("test@example.com", "password123").await;

        let app = app();
        let admin_token = create_test_admin_token();

        let payload = json!({
            "email": "test@example.com",
            "password": "password123"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", admin_token))
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let auth_response: AuthResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(auth_response.user.email, "test@example.com");
        assert!(!auth_response.token.is_empty());

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_login_invalid_password() {
        setup().await;
        cleanup_db().await;

        create_test_user("test@example.com", "password123").await;

        let app = app();
        let admin_token = create_test_admin_token();

        let payload = json!({
            "email": "test@example.com",
            "password": "wrongpassword"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {}", admin_token))
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_get_profile() {
        setup().await;
        cleanup_db().await;

        let (_user_id, token, _org_id) = create_test_user("test@example.com", "password123").await;

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/me")
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
        let user_response: UserResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(user_response.email, "test@example.com");

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_get_profile_unauthorized() {
        setup().await;
        cleanup_db().await;

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_registration_requires_admin_token() {
        setup().await;
        cleanup_db().await;

        let app = app();

        let payload = json!({
            "email": "test@example.com",
            "password": "testpassword123",
            "name": "Test User"
        });

        // Test without admin token
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Test with invalid admin token
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer admin_invalid_token")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_login_requires_admin_token() {
        setup().await;
        cleanup_db().await;

        create_test_user("test@example.com", "password123").await;

        let app = app();

        let payload = json!({
            "email": "test@example.com",
            "password": "password123"
        });

        // Test without admin token
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/login")
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
