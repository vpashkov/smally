use once_cell::sync::Lazy;
use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    // API Settings
    pub app_name: String,
    pub version: String,
    pub debug: bool,

    // Server Settings
    pub host: String,
    pub port: u16,
    pub workers: usize,

    // Model Settings
    pub model_name: String,
    pub model_path: String,
    pub max_tokens: usize,
    pub embedding_dim: usize,

    // Cache Settings
    pub l1_cache_size: usize,
    pub l2_cache_ttl: u64,
    pub redis_url: String,
    pub redis_db: i32,

    // Database Settings
    pub database_url: String,

    // Security Settings
    pub secret_key: String,
    pub api_key_prefix: String,

    // Rate Limiting
    pub free_tier_limit: i32,
    pub pro_tier_limit: i32,
    pub scale_tier_limit: i32,

    // Performance Settings
    pub max_batch_size: usize,
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            app_name: get_env("APP_NAME", "FastEmbed Query API"),
            version: get_env("VERSION", "0.1.0"),
            debug: get_env_bool("DEBUG", false),

            host: get_env("HOST", "0.0.0.0"),
            port: get_env_int("PORT", 8000) as u16,
            workers: get_env_int("WORKERS", 4) as usize,

            model_name: get_env("MODEL_NAME", "sentence-transformers/all-MiniLM-L6-v2"),
            model_path: get_env("MODEL_PATH", "./models/all-MiniLM-L6-v2-onnx"),
            max_tokens: get_env_int("MAX_TOKENS", 128) as usize,
            embedding_dim: get_env_int("EMBEDDING_DIM", 384) as usize,

            l1_cache_size: get_env_int("L1_CACHE_SIZE", 10000) as usize,
            l2_cache_ttl: get_env_int("L2_CACHE_TTL", 86400) as u64,
            redis_url: get_env("REDIS_URL", "redis://localhost:6379"),
            redis_db: get_env_int("REDIS_DB", 0),

            database_url: get_env(
                "DATABASE_URL",
                "postgres://localhost:5433/fastembed?sslmode=disable",
            ),

            secret_key: get_env(
                "SECRET_KEY",
                "change-this-to-a-secure-random-key-in-production",
            ),
            api_key_prefix: get_env("API_KEY_PREFIX", "fe_"),

            free_tier_limit: get_env_int("FREE_TIER_LIMIT", 20000),
            pro_tier_limit: get_env_int("PRO_TIER_LIMIT", 100000),
            scale_tier_limit: get_env_int("SCALE_TIER_LIMIT", 2000000),

            max_batch_size: get_env_int("MAX_BATCH_SIZE", 1) as usize,
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

pub static SETTINGS: Lazy<Settings> = Lazy::new(Settings::new);

pub fn get_settings() -> &'static Settings {
    &SETTINGS
}

fn get_env(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

fn get_env_int(key: &str, default: i32) -> i32 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
