use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum TierType {
    #[default]
    Free,
    Pro,
    Scale,
}

impl TierType {
    /// Convert to u8 (for compact serialization)
    pub fn to_u8(self) -> u8 {
        match self {
            TierType::Free => 0,
            TierType::Pro => 1,
            TierType::Scale => 2,
        }
    }

    /// Convert from u8
    pub fn from_u8(value: u8) -> Result<Self, String> {
        match value {
            0 => Ok(TierType::Free),
            1 => Ok(TierType::Pro),
            2 => Ok(TierType::Scale),
            _ => Err(format!("Invalid tier value: {}", value)),
        }
    }
}

// Custom serialization to use numbers instead of strings (for CBOR tokens)
impl Serialize for TierType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8((*self).to_u8())
    }
}

impl<'de> Deserialize<'de> for TierType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        TierType::from_u8(value).map_err(serde::de::Error::custom)
    }
}

// ============================================================================
// Core Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum OrganizationRole {
    #[serde(rename = "owner")]
    Owner,
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "member")]
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub tier: TierType,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub user_id: Uuid,
    pub role: OrganizationRole,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct APIKey {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub key_id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub last_used_at: Option<NaiveDateTime>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Usage {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub embeddings_count: i32,
    pub timestamp: NaiveDateTime,
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Debug, Deserialize, validator::Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid email format"))]
    #[validate(length(max = 255, message = "Email too long"))]
    pub email: String,

    #[validate(length(
        min = 8,
        max = 128,
        message = "Password must be between 8 and 128 characters"
    ))]
    pub password: String,

    #[validate(length(
        min = 1,
        max = 255,
        message = "Name must be between 1 and 255 characters"
    ))]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: UserResponse,
    pub token: String, // JWT for session management
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub tier: Option<TierType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrganizationResponse {
    pub id: Uuid,
    pub name: String,
    pub tier: TierType,
    pub role: OrganizationRole, // Current user's role
    pub is_active: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Deserialize)]
pub struct CreateAPIKeyRequest {
    pub name: String,
    pub tier: Option<TierType>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct APIKeyResponse {
    pub id: Uuid,
    pub key_id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub last_used_at: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>, // Only included when creating new key
}

#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: OrganizationRole,
}
