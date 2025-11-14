use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde_json::json;

use crate::auth::session::SessionClaims;
use crate::database;
use crate::models::{
    CreateOrganizationRequest, InviteMemberRequest, Organization, OrganizationResponse,
    OrganizationRole, TierType,
};

use super::users::ApiError;

/// Create a new organization
pub async fn create_organization_handler(
    claims: SessionClaims,
    Json(payload): Json<CreateOrganizationRequest>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;

    // Generate slug from name if not provided
    let slug = payload.slug.unwrap_or_else(|| {
        payload
            .name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect()
    });

    // Validate slug is unique
    let existing =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM organizations WHERE slug = $1")
            .bind(&slug)
            .fetch_one(pool)
            .await
            .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if existing > 0 {
        return Err(ApiError::BadRequest(
            "Organization slug already exists".to_string(),
        ));
    }

    // Create organization
    let tier = payload.tier.unwrap_or(TierType::Free);

    let org = sqlx::query_as::<_, Organization>(
        "INSERT INTO organizations (name, slug, owner_id, tier, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING *",
    )
    .bind(&payload.name)
    .bind(&slug)
    .bind(user_id)
    .bind(tier)
    .bind(true)
    .bind(Utc::now().naive_utc())
    .bind(Utc::now().naive_utc())
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to create organization: {}", e)))?;

    // Add creator as owner
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(org.id)
    .bind(user_id)
    .bind("owner")
    .bind(Utc::now().naive_utc())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to add organization member: {}", e)))?;

    let response = OrganizationResponse {
        id: org.id,
        name: org.name,
        slug: org.slug,
        tier: org.tier,
        role: OrganizationRole::Owner,
        is_active: org.is_active,
        created_at: org.created_at,
    };

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// List user's organizations
pub async fn list_organizations_handler(claims: SessionClaims) -> Result<Response, ApiError> {
    let pool = database::get_db();
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;

    #[derive(sqlx::FromRow)]
    struct OrgWithRole {
        id: i64,
        name: String,
        slug: String,
        tier: TierType,
        is_active: bool,
        created_at: chrono::NaiveDateTime,
        role: OrganizationRole,
    }

    let orgs = sqlx::query_as::<_, OrgWithRole>(
        "SELECT o.id, o.name, o.slug, o.tier, o.is_active, o.created_at, om.role
         FROM organizations o
         INNER JOIN organization_members om ON o.id = om.organization_id
         WHERE om.user_id = $1
         ORDER BY o.created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    let responses: Vec<OrganizationResponse> = orgs
        .into_iter()
        .map(|org| OrganizationResponse {
            id: org.id,
            name: org.name,
            slug: org.slug,
            tier: org.tier,
            role: org.role,
            is_active: org.is_active,
            created_at: org.created_at,
        })
        .collect();

    Ok((StatusCode::OK, Json(responses)).into_response())
}

/// Get organization by ID
pub async fn get_organization_handler(
    claims: SessionClaims,
    Path(org_id): Path<i64>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;

    #[derive(sqlx::FromRow)]
    struct OrgWithRole {
        id: i64,
        name: String,
        slug: String,
        tier: TierType,
        is_active: bool,
        created_at: chrono::NaiveDateTime,
        role: OrganizationRole,
    }

    let org = sqlx::query_as::<_, OrgWithRole>(
        "SELECT o.id, o.name, o.slug, o.tier, o.is_active, o.created_at, om.role
         FROM organizations o
         INNER JOIN organization_members om ON o.id = om.organization_id
         WHERE o.id = $1 AND om.user_id = $2",
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::Unauthorized("Organization not found or access denied".to_string()))?;

    let response = OrganizationResponse {
        id: org.id,
        name: org.name,
        slug: org.slug,
        tier: org.tier,
        role: org.role,
        is_active: org.is_active,
        created_at: org.created_at,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

/// Invite member to organization
pub async fn invite_member_handler(
    claims: SessionClaims,
    Path(org_id): Path<i64>,
    Json(payload): Json<InviteMemberRequest>,
) -> Result<Response, ApiError> {
    let pool = database::get_db();
    let user_id: i64 = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Unauthorized("Invalid user ID".to_string()))?;

    // Check if requester is owner or admin
    let member_role = sqlx::query_scalar::<_, OrganizationRole>(
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

    if member_role != OrganizationRole::Owner && member_role != OrganizationRole::Admin {
        return Err(ApiError::Unauthorized(
            "Only owners and admins can invite members".to_string(),
        ));
    }

    // Find user by email
    let invited_user = sqlx::query_scalar::<_, i64>("SELECT id FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("User not found".to_string()))?;

    // Check if already a member
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM organization_members WHERE organization_id = $1 AND user_id = $2",
    )
    .bind(org_id)
    .bind(invited_user)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Database error: {}", e)))?;

    if existing > 0 {
        return Err(ApiError::BadRequest("User is already a member".to_string()));
    }

    // Add member
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(org_id)
    .bind(invited_user)
    .bind(payload.role)
    .bind(Utc::now().naive_utc())
    .execute(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "message": "Member invited successfully" })),
    )
        .into_response())
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
                "/organizations",
                axum::routing::post(create_organization_handler),
            )
            .route(
                "/organizations",
                axum::routing::get(list_organizations_handler),
            )
            .route(
                "/organizations/:org_id",
                axum::routing::get(get_organization_handler),
            )
            .route(
                "/organizations/:org_id/members",
                axum::routing::post(invite_member_handler),
            )
    }

    #[tokio::test]
    #[serial]
    async fn test_create_organization() {
        setup().await;
        cleanup_db().await;

        let (_user_id, token, _org_id) = create_test_user("test@example.com", "password123").await;

        let app = app();

        let payload = json!({
            "name": "Test Organization",
            "slug": "test-org"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/organizations")
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
        let org_response: OrganizationResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(org_response.name, "Test Organization");
        assert_eq!(org_response.slug, "test-org");
        assert_eq!(org_response.role, OrganizationRole::Owner);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_list_organizations() {
        setup().await;
        cleanup_db().await;

        let (_user_id, token, _org_id) = create_test_user("test@example.com", "password123").await;

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/organizations")
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
        let orgs: Vec<OrganizationResponse> = serde_json::from_slice(&body).unwrap();

        // Should have personal organization
        assert_eq!(orgs.len(), 1);
        assert_eq!(orgs[0].role, OrganizationRole::Owner);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_get_organization() {
        setup().await;
        cleanup_db().await;

        let (_user_id, token, org_id) = create_test_user("test@example.com", "password123").await;

        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(&format!("/organizations/{}", org_id))
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
        let org: OrganizationResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(org.id, org_id);
        assert_eq!(org.role, OrganizationRole::Owner);

        cleanup_db().await;
    }

    #[tokio::test]
    #[serial]
    async fn test_invite_member() {
        setup().await;
        cleanup_db().await;

        let (_user_id1, token1, org_id) =
            create_test_user("owner@example.com", "password123").await;
        let (_user_id2, _token2, _org_id2) =
            create_test_user("member@example.com", "password123").await;

        let app = app();

        let payload = json!({
            "email": "member@example.com",
            "role": "member"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/organizations/{}/members", org_id))
                    .header("authorization", format!("Bearer {}", token1))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        cleanup_db().await;
    }
}
