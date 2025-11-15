use api::auth::{sign_token_direct, TokenData};
use api::config;
use api::models::TierType;
use ed25519_dalek::SigningKey;
use sqlx::postgres::PgPoolOptions;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load settings
    dotenvy::dotenv().ok();
    let settings = config::get_settings();

    // Get private key from environment
    let private_key_hex = env::var("TOKEN_PRIVATE_KEY")?;

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cargo run --bin create_api_key <org_id> <name>");
        eprintln!(
            "Example: cargo run --bin create_api_key 018d1234-5678-7abc-9def-0123456789ab \"Production API Key\""
        );
        eprintln!("\nThis will:");
        eprintln!("  1. Verify the organization exists");
        eprintln!("  2. Create an API key record in the database");
        eprintln!("  3. Generate and return a signed token");
        std::process::exit(1);
    }

    let org_id: Uuid = Uuid::parse_str(&args[1])?;
    let key_name = &args[2];

    // Connect to database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&settings.database_url)
        .await?;

    // Verify organization exists and get tier
    let result: Option<(Uuid, String, bool)> = sqlx::query_as(
        "SELECT id, tier, is_active FROM organizations WHERE id = $1",
    )
    .bind(org_id)
    .fetch_optional(&pool)
    .await?;

    let (org_id, tier_str, is_active) = match result {
        Some(org) => org,
        None => {
            eprintln!("Error: Organization {} not found", org_id);
            eprintln!("\nTo list organizations:");
            eprintln!(
                "  PGPASSWORD=$DB_PASSWORD psql -h localhost -p 5433 -U smally -d smally -c 'SELECT id, name, tier FROM organizations;'"
            );
            std::process::exit(1);
        }
    };

    if !is_active {
        eprintln!("Error: Organization {} is not active", org_id);
        std::process::exit(1);
    }

    // Parse tier
    let tier = match tier_str.to_lowercase().as_str() {
        "free" => TierType::Free,
        "pro" => TierType::Pro,
        "scale" => TierType::Scale,
        _ => {
            eprintln!("Error: Unknown tier: {}", tier_str);
            std::process::exit(1);
        }
    };

    // Generate key_id (UUIDv7 - time-ordered)
    let key_id = Uuid::now_v7();

    // Insert API key record
    sqlx::query(
        "INSERT INTO api_keys (organization_id, key_id, name, is_active, created_at)
         VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(org_id)
    .bind(key_id)
    .bind(key_name)
    .bind(true)
    .execute(&pool)
    .await?;

    println!("✅ API key record created in database");

    // Determine limits based on tier
    let (max_tokens, monthly_quota) = match tier {
        TierType::Free => (128, 20_000),
        TierType::Pro => (8192, 100_000),
        TierType::Scale => (8192, 2_000_000),
    };

    // Decode private key
    let private_key_bytes = hex::decode(&private_key_hex)?;

    // Create signing key
    let signing_key = SigningKey::from_bytes(&private_key_bytes.try_into().unwrap());

    let token_data = TokenData {
        org_id,
        key_id,
        tier,
        max_tokens,
        monthly_quota,
    };

    // Sign token
    let token = sign_token_direct(&token_data, &signing_key)?;

    // Add prefix
    let full_token = format!("{}{}", settings.api_key_prefix, token);

    println!("\n=== API Key Created ===\n");
    println!("Organization ID: {}", org_id);
    println!("Tier: {:?}", tier);
    println!("Key ID: {}", key_id);
    println!("Name: {}", key_name);
    println!("\nToken:");
    println!("{}\n", full_token);
    println!("=== Usage ===");
    println!("curl -X POST http://localhost:8000/v1/embed \\");
    println!("  -H \"Content-Type: application/json\" \\");
    println!("  -H \"Authorization: Bearer {}\" \\", full_token);
    println!("  -d '{{\"text\": \"Hello world\"}}'");
    println!();

    Ok(())
}
