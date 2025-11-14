use chrono::{Duration, Utc};
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Key, PasetoAsymmetricPrivateKey, Public, V4};
use rusty_paseto::generic::GenericBuilder;
use rusty_paseto::prelude::CustomClaim;
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

    // Validate and capitalize tier
    let tier = match tier_input.to_lowercase().as_str() {
        "free" => "Free",
        "pro" => "Pro",
        "scale" => "Scale",
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
    let (max_tokens, monthly_quota) = match tier {
        "Free" => (128, 20_000),
        "Pro" => (8192, 100_000),
        "Scale" => (8192, 2_000_000),
        _ => unreachable!(),
    };

    // Create token claims (long-lived: 5 years)
    let now = Utc::now();
    let exp = now + Duration::days(365 * 5);

    // Create PASETO private key (64 bytes = 32 private + 32 public)
    let verifying_bytes = signing_key.verifying_key().to_bytes();
    let mut full_key = [0u8; 64];
    full_key[..32].copy_from_slice(&signing_key.to_bytes());
    full_key[32..].copy_from_slice(&verifying_bytes);

    let key = Key::<64>::try_from(&full_key[..]).unwrap();
    let private_key = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);

    /*
    Build and sign token with compact claims (single-letter keys).
    Using GenericBuilder to avoid auto-added iat/nbf claims.
    Use "e" instead of reserved "exp" to allow Unix timestamp.
    Flatten all fields to top level to avoid nested JSON structure.
    */
    let exp_timestamp = exp.timestamp();

    let token = GenericBuilder::<V4, Public>::default()
        .set_claim(CustomClaim::try_from(("e", exp_timestamp)).unwrap())
        .set_claim(CustomClaim::try_from(("u", user_id as i64)).unwrap())
        .set_claim(CustomClaim::try_from(("k", key_id.to_string())).unwrap())
        .set_claim(CustomClaim::try_from(("t", tier.to_string())).unwrap())
        .set_claim(CustomClaim::try_from(("m", max_tokens as i64)).unwrap())
        .set_claim(CustomClaim::try_from(("q", monthly_quota as i64)).unwrap())
        .try_sign(&private_key)
        .unwrap();

    println!("\n=== PASETO Token Generated ===\n");
    println!("User ID: {}", user_id);
    println!("Tier: {}", tier);
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
