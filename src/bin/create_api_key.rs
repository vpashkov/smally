// Simple utility to create API keys
// Usage: cargo run --bin create_api_key -- email@example.com tier

use anyhow::Result;
use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env
    dotenvy::dotenv().ok();

    // Get arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <email> [tier]", args[0]);
        eprintln!("");
        eprintln!("Examples:");
        eprintln!("  {} user@example.com", args[0]);
        eprintln!("  {} user@example.com pro", args[0]);
        eprintln!("  {} admin@example.com scale", args[0]);
        eprintln!("");
        eprintln!("Tiers: free (default), pro, scale");
        std::process::exit(1);
    }

    let email = &args[1];
    let tier = if args.len() > 2 { &args[2] } else { "free" };

    // Get database URL
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    println!("Creating API key for: {} (tier: {})", email, tier);
    println!("");

    // Connect to database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Generate API key
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    let api_key = format!("fe_{}", hex::encode(random_bytes));

    // Hash the key
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let key_prefix = format!("{}...", &api_key[0..13]);

    // Create or update user
    let user_id: i64 = sqlx::query_scalar(
        "INSERT INTO users (email, tier, is_active, created_at, updated_at)
         VALUES ($1, $2, true, NOW(), NOW())
         ON CONFLICT (email) DO UPDATE SET tier = $2, updated_at = NOW()
         RETURNING id"
    )
    .bind(email)
    .bind(tier)
    .fetch_one(&pool)
    .await?;

    // Create API key
    sqlx::query(
        "INSERT INTO api_keys (user_id, key_hash, key_prefix, name, is_active, created_at)
         VALUES ($1, $2, $3, $4, true, NOW())"
    )
    .bind(user_id)
    .bind(&key_hash)
    .bind(&key_prefix)
    .bind("Default API Key")
    .execute(&pool)
    .await?;

    // Display result
    println!("============================================================");
    println!("API KEY GENERATED (save this, it won't be shown again):");
    println!("============================================================");
    println!("");
    println!("{}", api_key);
    println!("");
    println!("============================================================");
    println!("");
    println!("Add this to your requests as:");
    println!("Authorization: Bearer {}", api_key);
    println!("============================================================");
    println!("");
    println!("✓ API key created successfully!");

    Ok(())
}
