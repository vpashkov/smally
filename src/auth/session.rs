use anyhow::{anyhow, Result};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{Cookie, SameSite};
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
pub fn create_session_token(user_id: uuid::Uuid, email: &str) -> Result<String> {
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

/// Session cookie name
pub const SESSION_COOKIE_NAME: &str = "session";

/// Create a session cookie with security settings
pub fn create_session_cookie(token: &str) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, token.to_string()))
        .path("/")
        .max_age(time::Duration::days(7))
        .same_site(SameSite::Lax)
        .http_only(true)
        // TODO: Enable secure flag in production (requires HTTPS)
        // .secure(true)
        .build()
}

/// Create a cookie that clears the session
pub fn clear_session_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE_NAME, ""))
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build()
}

/// Session cookie extractor for authenticated web requests
#[derive(Debug, Clone)]
pub struct SessionCookie {
    pub claims: SessionClaims,
}

impl SessionCookie {
    pub fn user_id(&self) -> uuid::Uuid {
        uuid::Uuid::parse_str(&self.claims.sub).unwrap_or_default()
    }

    pub fn email(&self) -> &str {
        &self.claims.email
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for SessionCookie
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Build redirect URL with next parameter
        let path_and_query = parts
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let redirect_url = format!("/login?next={}", urlencoding::encode(path_and_query));

        // Get session cookie
        let cookies_header = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Redirect::to(&redirect_url).into_response())?;

        // Parse cookies and find session
        let session_token = cookies_header
            .split(';')
            .map(|s| s.trim())
            .find_map(|cookie| {
                let mut parts = cookie.splitn(2, '=');
                let name = parts.next()?;
                let value = parts.next()?;
                if name == SESSION_COOKIE_NAME {
                    Some(value)
                } else {
                    None
                }
            })
            .ok_or_else(|| Redirect::to(&redirect_url).into_response())?;

        // Verify token
        let claims = verify_session_token(session_token).map_err(|e| {
            tracing::warn!("Invalid session token: {}", e);
            Redirect::to(&redirect_url).into_response()
        })?;

        Ok(SessionCookie { claims })
    }
}
