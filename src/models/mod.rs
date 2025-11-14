use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, sqlx::Type)]
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

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub tier: TierType,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct APIKey {
    pub id: i64,
    pub user_id: i64,
    pub key_hash: String,
    pub key_prefix: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: NaiveDateTime,
    pub last_used_at: Option<NaiveDateTime>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Usage {
    pub id: i64,
    pub user_id: i64,
    pub api_key_id: Option<i64>,
    pub embeddings_count: i32,
    pub timestamp: NaiveDateTime,
}
