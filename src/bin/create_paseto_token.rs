use api::auth::{sign_token_direct, TokenData};
use api::models::TierType;
use chrono::{Duration, Utc};
use ed25519_dalek::SigningKey;
use std::env;

fn main() {
    // Get private key from environment or args
    let private_key_hex = env::var("PASETO_PRIVATE_KEY").unwrap_or_else(|_| {
        eprintln!("Error: PASETO_PRIVATE_KEY environment variable not set");
        eprintln!("Usage: PASETO_PRIVATE_KEY=<hex> cargo run --bin create_paseto_token <user_id> <tier> <key_id>");
        std::process::exit(1);
    });

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: cargo run --bin create_paseto_token <user_id> <tier> <key_id>");
        eprintln!("Example: cargo run --bin create_paseto_token 123 free my-api-key-1");
        eprintln!("\nTiers: free, pro, scale");
        std::process::exit(1);
    }

    let user_id: i64 = args[1].parse().unwrap_or_else(|_| {
        eprintln!("Error: user_id must be a number");
        std::process::exit(1);
    });

    let tier_input = &args[2];
    let key_id = &args[3];

    // Validate tier and convert to TierType
    let (tier_name, tier_value) = match tier_input.to_lowercase().as_str() {
        "free" => ("Free", TierType::Free),
        "pro" => ("Pro", TierType::Pro),
        "scale" => ("Scale", TierType::Scale),
        _ => {
            eprintln!("Error: tier must be one of: free, pro, scale");
            std::process::exit(1);
        }
    };

    // Decode private key
    let private_key_bytes = hex::decode(&private_key_hex).unwrap_or_else(|_| {
        eprintln!("Error: Invalid private key hex");
        std::process::exit(1);
    });

    // Create signing key
    let signing_key = SigningKey::from_bytes(&private_key_bytes.try_into().unwrap_or_else(|_| {
        eprintln!("Error: Private key must be 32 bytes");
        std::process::exit(1);
    }));

    // Determine limits based on tier
    let (max_tokens, monthly_quota) = match tier_value {
        TierType::Free => (128, 20_000),
        TierType::Pro => (8192, 100_000),
        TierType::Scale => (8192, 2_000_000),
    };

    // Create token claims (long-lived: 5 years)
    let now = Utc::now();
    let exp = now + Duration::days(365 * 5);
    let exp_timestamp = exp.timestamp();

    let token_data = TokenData {
        e: exp_timestamp,
        u: user_id,
        k: key_id.to_string(),
        t: tier_value,
        m: max_tokens as i32,
        q: monthly_quota as i32,
    };

    // Sign token directly with Ed25519 (no PASETO overhead)
    let token = sign_token_direct(&token_data, &signing_key).unwrap();

    println!("\n=== Direct Signed Token Generated ===\n");
    println!("User ID: {}", user_id);
    println!("Tier: {}", tier_name);
    println!("Key ID: {}", key_id);
    println!("Expiration: {} (5 years)", exp.format("%Y-%m-%d"));
    println!("\nToken:");
    println!("{}\n", token);
    println!("=== Usage ===");
    println!("curl -X POST http://localhost:8000/v1/embed \\");
    println!("  -H \"Content-Type: application/json\" \\");
    println!("  -H \"Authorization: Bearer {}\" \\", token);
    println!("  -d '{{\"text\": \"Hello world\"}}'");
    println!();
}
