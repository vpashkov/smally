//! Dashless UUID support for URLs and slugs
//!
//! This module provides a wrapper around `uuid::Uuid` that serializes/deserializes
//! without dashes for cleaner URLs.
//!
//! Example:
//! - With dashes: `550e8400-e29b-41d4-a716-446655440000`
//! - Without dashes: `550e8400e29b41d4a716446655440000`

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use uuid::Uuid;

/// A UUID wrapper that serializes/deserializes without dashes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DashlessUuid(pub Uuid);

impl DashlessUuid {
    /// Create a new DashlessUuid from a standard UUID
    pub fn new(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the inner UUID
    pub fn into_inner(self) -> Uuid {
        self.0
    }

    /// Convert to dashless string representation
    pub fn to_dashless_string(&self) -> String {
        self.0.simple().to_string()
    }

    /// Parse from dashless string
    pub fn from_dashless_string(s: &str) -> Result<Self, uuid::Error> {
        // If the string has dashes, remove them
        let cleaned = s.replace('-', "");
        let uuid = Uuid::parse_str(&cleaned)?;
        Ok(Self(uuid))
    }
}

impl From<Uuid> for DashlessUuid {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<DashlessUuid> for Uuid {
    fn from(dashless: DashlessUuid) -> Self {
        dashless.0
    }
}

impl fmt::Display for DashlessUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_dashless_string())
    }
}

impl std::str::FromStr for DashlessUuid {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_dashless_string(s)
    }
}

// Serialize as dashless string
impl Serialize for DashlessUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_dashless_string())
    }
}

// Deserialize from dashless string (also accepts dashed format for compatibility)
impl<'de> Deserialize<'de> for DashlessUuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_dashless_string(&s).map_err(serde::de::Error::custom)
    }
}

// Axum automatically uses FromStr for path extraction, so no custom implementation needed

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashless_conversion() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let dashless = DashlessUuid::new(uuid);

        assert_eq!(dashless.to_dashless_string(), "550e8400e29b41d4a716446655440000");
        assert_eq!(dashless.to_string(), "550e8400e29b41d4a716446655440000");
    }

    #[test]
    fn test_parse_dashless() {
        let dashless = DashlessUuid::from_dashless_string("550e8400e29b41d4a716446655440000").unwrap();
        let expected = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        assert_eq!(dashless.into_inner(), expected);
    }

    #[test]
    fn test_parse_with_dashes() {
        // Should also accept dashed format for backward compatibility
        let dashless = DashlessUuid::from_dashless_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let expected = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        assert_eq!(dashless.into_inner(), expected);
    }

    #[test]
    fn test_serialize() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let dashless = DashlessUuid::new(uuid);

        let json = serde_json::to_string(&dashless).unwrap();
        assert_eq!(json, r#""550e8400e29b41d4a716446655440000""#);
    }

    #[test]
    fn test_deserialize() {
        let json = r#""550e8400e29b41d4a716446655440000""#;
        let dashless: DashlessUuid = serde_json::from_str(json).unwrap();
        let expected = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        assert_eq!(dashless.into_inner(), expected);
    }
}
