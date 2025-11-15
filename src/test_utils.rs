#[cfg(test)]
pub mod helpers {
    use crate::{auth, billing, cache, config, database, inference};
    use std::sync::Once;

    static INIT: Once = Once::new();

    /// Initialize the test environment once (database, Redis, model)
    /// This runs only once for all tests - subsequent calls are no-ops
    pub async fn setup() {
        use std::sync::atomic::{AtomicBool, Ordering};
        static SETUP_COMPLETE: AtomicBool = AtomicBool::new(false);

        // Only run setup once across all tests
        if SETUP_COMPLETE.swap(true, Ordering::SeqCst) {
            return; // Already initialized
        }

        INIT.call_once(|| {
            // Load test environment variables
            dotenvy::from_filename(".env.test").ok();

            // Initialize tracing for tests
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::WARN) // Reduce noise in tests
                .with_test_writer()
                .try_init()
                .ok();
        });

        // Initialize services only once
        // All these functions now handle re-initialization gracefully

        // Database pool (no migrations in test mode)
        database::init_db()
            .await
            .expect("Failed to initialize database");

        // Model
        inference::init_model().expect("Failed to initialize model");

        // Redis cache
        cache::init_cache()
            .await
            .expect("Failed to initialize cache");

        // Billing Redis
        billing::init_redis()
            .await
            .expect("Failed to initialize billing redis");

        // Token validator
        auth::init_token_validator()
            .await
            .expect("Failed to initialize token validator");

        // Note: Usage buffer is NOT initialized in tests to avoid connection pool issues
        // Tests don't record usage metrics anyway
    }

    /// Clean up the test database
    pub async fn cleanup_db() {
        let pool = database::get_db();

        // Clean tables in correct order (respecting foreign keys)
        sqlx::query("DELETE FROM usage").execute(pool).await.ok();
        sqlx::query("DELETE FROM api_keys").execute(pool).await.ok();
        sqlx::query("DELETE FROM organization_members")
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM organizations")
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM users").execute(pool).await.ok();
    }

    /// Create a test user and return (user_id, session_token, org_id)
    pub async fn create_test_user(email: &str, password: &str) -> (i64, String, i64) {
        use crate::auth::session::create_session_token;
        use crate::models::{TierType, User};
        use bcrypt::{hash, DEFAULT_COST};
        use chrono::Utc;

        let pool = database::get_db();

        let password_hash = hash(password, DEFAULT_COST).expect("Failed to hash password");

        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (email, name, password_hash, tier, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING *",
        )
        .bind(email)
        .bind("Test User")
        .bind(&password_hash)
        .bind(TierType::Free)
        .bind(true)
        .bind(Utc::now().naive_utc())
        .bind(Utc::now().naive_utc())
        .fetch_one(pool)
        .await
        .expect("Failed to create user");

        // Create personal organization
        let slug = format!("user-{}-org", user.id);
        let org_name = format!("{}'s Organization", email);

        let org_id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO organizations (name, slug, owner_id, tier, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
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
        .expect("Failed to create organization");

        // Add user as owner
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
        .expect("Failed to add organization member");

        let token =
            create_session_token(user.id, &user.email).expect("Failed to create session token");

        (user.id, token, org_id)
    }

    /// Create a test CWT token for API access
    pub async fn create_test_api_token(org_id: i64, tier: crate::models::TierType) -> String {
        use crate::auth::{sign_token_direct, TokenData};
        use chrono::Utc;
        use uuid::Uuid;

        let settings = config::get_settings();
        let pool = database::get_db();

        let key_id = Uuid::now_v7();

        // Create API key in database
        sqlx::query(
            "INSERT INTO api_keys (organization_id, key_id, name, is_active, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(org_id)
        .bind(key_id)
        .bind("Test API Key")
        .bind(true)
        .bind(Utc::now().naive_utc())
        .execute(pool)
        .await
        .expect("Failed to create API key");

        // Generate CWT token
        let private_key_bytes =
            hex::decode(&settings.token_private_key).expect("Invalid private key");

        let signing_key = ed25519_dalek::SigningKey::from_bytes(
            &private_key_bytes[..]
                .try_into()
                .expect("Invalid private key length"),
        );

        let expiration = Utc::now() + chrono::Duration::days(365);
        let (max_tokens, monthly_quota) = match tier {
            crate::models::TierType::Free => (settings.max_tokens, settings.free_tier_limit),
            crate::models::TierType::Pro => (settings.max_tokens, settings.pro_tier_limit),
            crate::models::TierType::Scale => (settings.max_tokens, settings.scale_tier_limit),
        };

        let token_data = TokenData {
            expiration: expiration.timestamp(),
            user_id: 1, // For backward compatibility
            key_id,
            tier,
            max_tokens: max_tokens as i32,
            monthly_quota,
        };

        let token = sign_token_direct(&token_data, &signing_key).expect("Failed to sign token");

        format!("{}{}", settings.api_key_prefix, token)
    }

    /// Create a test admin token for UI/admin access
    pub fn create_test_admin_token() -> String {
        use crate::auth::sign_admin_token;
        use chrono::Utc;

        let settings = config::get_settings();

        // Generate admin token
        let private_key_bytes =
            hex::decode(&settings.token_private_key).expect("Invalid private key");

        let signing_key = ed25519_dalek::SigningKey::from_bytes(
            &private_key_bytes[..]
                .try_into()
                .expect("Invalid private key length"),
        );

        let expiration = (Utc::now() + chrono::Duration::days(365)).timestamp();
        let token =
            sign_admin_token("ui", expiration, &signing_key).expect("Failed to sign admin token");

        format!("admin_{}", token)
    }
}
