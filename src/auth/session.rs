use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::config;

/// JWT session claims for authenticated users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// User email
    pub email: String,
}

/// Generate a JWT session token for a user
pub fn create_session_token(user_id: i64, email: &str) -> Result<String> {
    let settings = config::get_settings();

    let now = Utc::now();
    let exp = now + Duration::days(7); // 7_day session

    let claims = SessionClaims {
        sub: user_id.to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
        email: email.to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(settings.jwt_secret.as_bytes()),
    )?;

    Ok(token)
}

/// Verify and decode a JWT session token
pub fn verify_session_token(token: &str) -> Result<SessionClaims> {
    let settings = config::get_settings();

    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(settings.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| anyhow!("Invalid session token: {}", e))?;

    Ok(token_data.claims)
}
